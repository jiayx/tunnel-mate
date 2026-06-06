mod config;
mod diagnostics;
mod event_logger;
mod manager;
mod ssh;
mod ssh_config;

use crate::config::{AppConfig, ConfigStore, Tunnel, TunnelStatus};
use crate::diagnostics::{run_diagnostics, DiagnosticStep};
use crate::event_logger::{EventLogger, EventType, LogEvent};
use crate::manager::TunnelManager;
use crate::ssh_config::{parse_ssh_config, SshHostConfig};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::Mutex;

struct TunnelState(Arc<Mutex<TunnelManager>>);

#[tauri::command]
fn get_config() -> Result<AppConfig, String> {
    let store = ConfigStore::new();
    store.load_config()
}

#[tauri::command]
fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    let store = ConfigStore::new();
    store.save_config(&config)?;
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
    Ok(parse_ssh_config())
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
    Ok(run_diagnostics(&tunnel, passphrase.as_deref()).await)
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

    // Simple validation
    if new_config.version == 0 {
        return Err("Invalid configuration version".to_string());
    }

    store.save_config(&new_config)?;

    // Log import event
    let logger = EventLogger::new();
    let _ = logger.log(
        None,
        None,
        EventType::Updated,
        "Configuration imported successfully".to_string(),
    );

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
                let label = format!(
                    "{} {} (Port {})",
                    status_icon, tunnel.name, tunnel.local_port
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

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(TunnelState(manager))
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_events,
            clear_events,
            import_ssh_config,
            select_private_key_file,
            test_connection,
            start_tunnel,
            stop_tunnel,
            get_tunnel_status,
            export_config,
            import_config
        ])
        .setup(|app| {
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

            if let Some(window) = app.get_webview_window("main") {
                let store = ConfigStore::new();
                if let Ok(config) = store.load_config() {
                    if let Some(settings) = config.settings {
                        if settings.start_minimized {
                            let _ = window.hide();
                        }
                    }
                }

                let window_ = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        let store = ConfigStore::new();
                        if let Ok(config) = store.load_config() {
                            if let Some(settings) = config.settings {
                                if settings.close_to_tray {
                                    api.prevent_close();
                                    let _ = window_.hide();
                                }
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
