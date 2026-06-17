mod config;
mod diagnostics;
mod event_logger;
mod manager;
mod ssh;
mod ssh_config;

use crate::config::{
    validate_config, AppConfig, ConfigStore, GlobalSettings, Tunnel, TunnelStatus,
};
use crate::diagnostics::{run_diagnostics, DiagnosticStep};
use crate::event_logger::{EventLogger, EventType, LogEvent};
use crate::manager::TunnelManager;
use crate::ssh::engine::SshSession;
use crate::ssh_config::{parse_ssh_config, SshHostConfig};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::Mutex;

struct TunnelState(Arc<Mutex<TunnelManager>>);
struct SettingsState(Arc<Mutex<GlobalSettings>>);

#[tauri::command]
fn get_config() -> Result<AppConfig, String> {
    let store = ConfigStore::new();
    store.load_config()
}

#[tauri::command]
async fn save_config(
    app: AppHandle,
    settings_state: State<'_, SettingsState>,
    config: AppConfig,
) -> Result<(), String> {
    validate_config(&config)?;
    let store = ConfigStore::new();
    store.save_config(&config)?;
    sync_autostart(&app, config.settings.launch_on_startup)?;
    *settings_state.0.lock().await = config.settings.clone();
    update_tray_menu(&app);
    Ok(())
}

#[tauri::command]
fn get_events() -> Result<Vec<LogEvent>, String> {
    let logger = EventLogger::new();
    logger.get_events()
}

#[tauri::command]
fn clear_events() -> Result<(), String> {
    let logger = EventLogger::new();
    logger.clear_events()
}

#[tauri::command]
fn import_ssh_config() -> Result<Vec<SshHostConfig>, String> {
    let store = ConfigStore::new();
    let ssh_config_path = store
        .load_config()
        .ok()
        .and_then(|config| config.settings.ssh_config_path);
    Ok(parse_ssh_config(ssh_config_path.as_deref()))
}

#[tauri::command]
async fn select_private_key_file(app: AppHandle) -> Result<Option<String>, String> {
    let mut dialog = app.dialog().file().set_title("Select SSH Private Key");

    if let Some(ssh_dir) = dirs::home_dir().map(|home| home.join(".ssh")) {
        if ssh_dir.exists() {
            dialog = dialog.set_directory(ssh_dir);
        }
    }

    let selected = dialog.blocking_pick_file();

    Ok(selected.map(|path| path.to_string()))
}

#[tauri::command]
async fn test_connection(
    tunnel: Tunnel,
    passphrase: Option<String>,
) -> Result<Vec<DiagnosticStep>, String> {
    let config = ConfigStore::new().load_config().unwrap_or_default();
    Ok(run_diagnostics(&tunnel, &config.tunnels, passphrase.as_deref()).await)
}

#[tauri::command]
async fn trust_host_key(host: String, port: u16) -> Result<(), String> {
    SshSession::trust_host_key(&host, port).await
}

#[tauri::command]
async fn start_tunnel(
    app: AppHandle,
    state: State<'_, TunnelState>,
    tunnel_id: String,
    passphrase: Option<String>,
    log_channel: tauri::ipc::Channel<String>,
) -> Result<(), String> {
    let config = get_config()?;
    let tunnel = config
        .tunnels
        .iter()
        .find(|t| t.id == tunnel_id)
        .ok_or_else(|| format!("Tunnel with ID {} not found", tunnel_id))?
        .clone();

    TunnelManager::start_tunnel(state.0.clone(), app, tunnel, passphrase, log_channel).await
}

#[tauri::command]
async fn stop_tunnel(
    app: AppHandle,
    state: State<'_, TunnelState>,
    tunnel_id: String,
) -> Result<(), String> {
    let mut manager = state.0.lock().await;
    manager.stop_tunnel(&app, &tunnel_id).await
}

#[tauri::command]
async fn get_tunnel_status(
    state: State<'_, TunnelState>,
    tunnel_id: String,
) -> Result<String, String> {
    let manager = state.0.lock().await;
    let status = manager.get_status(&tunnel_id);
    Ok(match status {
        TunnelStatus::Stopped => "stopped",
        TunnelStatus::Running => "running",
        TunnelStatus::Connecting => "connecting",
        TunnelStatus::Reconnecting => "reconnecting",
        TunnelStatus::Failed => "failed",
    }
    .to_string())
}

#[tauri::command]
fn export_config() -> Result<String, String> {
    let store = ConfigStore::new();
    let config = store.load_config()?;
    serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config for export: {}", e))
}

#[tauri::command]
fn import_config(app: AppHandle, config_str: String) -> Result<(), String> {
    let store = ConfigStore::new();
    let new_config: AppConfig = serde_json::from_str(&config_str)
        .map_err(|e| format!("Invalid configuration format: {}", e))?;

    validate_config(&new_config)?;

    store.save_config(&new_config)?;

    // Log import event
    let logger = EventLogger::new();
    if let Ok(event) = logger.log(
        None,
        None,
        EventType::Updated,
        "Configuration imported successfully".to_string(),
    ) {
        let _ = app.emit("activity-event-created", event);
    }

    update_tray_menu(&app);
    Ok(())
}

pub fn update_tray_menu(app: &AppHandle) {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app_handle.state::<TunnelState>();
        let manager = state.0.lock().await;

        let store = ConfigStore::new();
        let config = match store.load_config() {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Failed to load config for tray menu: {}", e);
                return;
            }
        };

        let mut menu_items: Vec<Box<dyn tauri::menu::IsMenuItem<tauri::Wry>>> = Vec::new();

        // 1. Title/Header
        if let Ok(title) = MenuItem::with_id(
            &app_handle,
            "tunnels_title",
            "--- Tunnels ---",
            false,
            None::<&str>,
        ) {
            menu_items.push(Box::new(title));
        }

        // 2. Dynamic Tunnel items
        if config.tunnels.is_empty() {
            if let Ok(empty) = MenuItem::with_id(
                &app_handle,
                "tunnels_empty",
                "No tunnels configured",
                false,
                None::<&str>,
            ) {
                menu_items.push(Box::new(empty));
            }
        } else {
            for tunnel in &config.tunnels {
                let status = manager.get_status(&tunnel.id);
                let status_icon = match status {
                    TunnelStatus::Running => "🟢",
                    TunnelStatus::Connecting => "🟡",
                    TunnelStatus::Reconnecting => "🔄",
                    TunnelStatus::Failed => "🔴",
                    TunnelStatus::Stopped => "⚪",
                };
                let listen = tunnel.forward.listen();
                let label = format!(
                    "{} {} (Listen {}:{})",
                    status_icon, tunnel.name, listen.host, listen.port
                );
                let item_id = format!("toggle_{}", tunnel.id);
                if let Ok(item) = MenuItem::with_id(&app_handle, item_id, label, true, None::<&str>)
                {
                    menu_items.push(Box::new(item));
                }
            }
        }

        // 3. Static actions
        if let Ok(sep1) = tauri::menu::PredefinedMenuItem::separator(&app_handle) {
            menu_items.push(Box::new(sep1));
        }
        if let Ok(show) =
            MenuItem::with_id(&app_handle, "show", "Show Main Window", true, None::<&str>)
        {
            menu_items.push(Box::new(show));
        }
        if let Ok(hide) =
            MenuItem::with_id(&app_handle, "hide", "Hide Main Window", true, None::<&str>)
        {
            menu_items.push(Box::new(hide));
        }
        if let Ok(sep2) = tauri::menu::PredefinedMenuItem::separator(&app_handle) {
            menu_items.push(Box::new(sep2));
        }
        if let Ok(quit) = MenuItem::with_id(&app_handle, "quit", "Quit", true, None::<&str>) {
            menu_items.push(Box::new(quit));
        }

        let ref_menu_items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = menu_items
            .iter()
            .map(|item| item.as_ref() as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
            .collect();

        if let Ok(menu) = Menu::with_items(&app_handle, &ref_menu_items) {
            if let Some(tray) = app_handle.tray_by_id("main-tray") {
                let _ = tray.set_menu(Some(menu));
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let manager = Arc::new(Mutex::new(TunnelManager::new()));
    let manager_state = manager.clone();
    let initial_settings = ConfigStore::new()
        .load_config()
        .map(|config| config.settings)
        .unwrap_or_default();
    let settings_state = Arc::new(Mutex::new(initial_settings));

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(TunnelState(manager))
        .manage(SettingsState(settings_state))
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_events,
            clear_events,
            import_ssh_config,
            select_private_key_file,
            test_connection,
            trust_host_key,
            start_tunnel,
            stop_tunnel,
            get_tunnel_status,
            export_config,
            import_config
        ])
        .setup(move |app| {
            let mut tray_builder = TrayIconBuilder::with_id("main-tray");
            let icon_path = app.path().resolve(
                "icons/trayTemplate.png",
                tauri::path::BaseDirectory::Resource,
            );
            let icon = icon_path
                .ok()
                .and_then(|p| tauri::image::Image::from_path(p).ok());

            if let Some(icon) = icon {
                tray_builder = tray_builder.icon(icon).icon_as_template(true);
            } else if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            let _tray = tray_builder
                .tooltip("Tunnel Mate")
                .on_menu_event(|app, event| {
                    let id = event.id().as_ref();
                    if id.starts_with("toggle_") {
                        let tunnel_id = id.strip_prefix("toggle_").unwrap().to_string();
                        let _ = app.emit("tray-toggle-tunnel", tunnel_id);
                    } else {
                        match id {
                            "show" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                            "hide" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.hide();
                                }
                            }
                            "quit" => {
                                app.exit(0);
                            }
                            _ => {}
                        }
                    }
                })
                .build(app)?;

            // Build initial tray menu
            update_tray_menu(app.handle());

            let app_handle = app.handle().clone();
            let auto_start_manager = manager_state.clone();
            tauri::async_runtime::spawn(async move {
                let store = ConfigStore::new();
                let config = match store.load_config() {
                    Ok(config) => config,
                    Err(_) => return,
                };

                for tunnel in config
                    .tunnels
                    .into_iter()
                    .filter(|tunnel| tunnel.start_with_app)
                {
                    let _ = TunnelManager::start_tunnel_silent(
                        auto_start_manager.clone(),
                        app_handle.clone(),
                        tunnel,
                    )
                    .await;
                }
            });

            if let Some(window) = app.get_webview_window("main") {
                let settings = app.state::<SettingsState>().0.blocking_lock().clone();
                if settings.start_minimized {
                    let _ = window.hide();
                }

                let window_ = window.clone();
                let settings_state = app.state::<SettingsState>().0.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        let settings = settings_state.blocking_lock().clone();
                        if settings.close_to_tray {
                            api.prevent_close();
                            let _ = window_.hide();
                        }
                    }
                });
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Reopen { .. } = event {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });
}

fn sync_autostart(app: &AppHandle, launch_on_startup: bool) -> Result<(), String> {
    let autostart = app.autolaunch();
    if launch_on_startup {
        autostart
            .enable()
            .map_err(|e| format!("Failed to enable launch on startup: {}", e))
    } else {
        autostart
            .disable()
            .map_err(|e| format!("Failed to disable launch on startup: {}", e))
    }
}
