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
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;

struct TunnelState(Arc<Mutex<TunnelManager>>);

#[tauri::command]
fn get_config() -> Result<AppConfig, String> {
    let store = ConfigStore::new();
    store.load_config()
}

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    let store = ConfigStore::new();
    store.save_config(&config)
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
async fn test_connection(
    tunnel: Tunnel,
    passphrase: Option<String>,
) -> Result<Vec<DiagnosticStep>, String> {
    tokio::task::spawn_blocking(move || Ok(run_diagnostics(&tunnel, passphrase.as_deref())))
        .await
        .map_err(|e| format!("Failed to join diagnostics thread: {}", e))?
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
async fn stop_tunnel(app: AppHandle, state: State<'_, TunnelState>, tunnel_id: String) -> Result<(), String> {
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
fn import_config(config_str: String) -> Result<(), String> {
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

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let manager = Arc::new(Mutex::new(TunnelManager::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(TunnelState(manager))
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_events,
            clear_events,
            import_ssh_config,
            test_connection,
            start_tunnel,
            stop_tunnel,
            get_tunnel_status,
            export_config,
            import_config
        ])
        .setup(|app| {
            // Create a menu for the tray
            let show_i = MenuItem::with_id(app, "show", "Show Main Window", true, None::<&str>)?;
            let hide_i = MenuItem::with_id(app, "hide", "Hide Main Window", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &hide_i, &quit_i])?;

            let mut tray_builder = TrayIconBuilder::new();
            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            let _tray = tray_builder
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
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
                })
                .build(app)?;

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
