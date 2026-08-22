use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use auto_launcher::AutoLaunchBuilder;
#[cfg(target_os = "windows")]
use auto_launcher::WindowsEnableMode;
#[cfg(target_os = "macos")]
use auto_launcher::{Error as AutoLaunchError, MacOSLaunchMode};
use image::GenericImageView;
use tray_icon::menu::{IconMenuItem, Menu, MenuEvent, MenuItem, NativeIcon, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
#[cfg(target_os = "windows")]
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
use tunnel_core::{Tunnel, TunnelStatus};

#[cfg(target_os = "macos")]
const AUTOSTART_NAME: &str = "com.jiayx.tunnel-mate";
#[cfg(not(target_os = "macos"))]
const AUTOSTART_NAME: &str = "Tunnel Mate";
#[cfg(target_os = "macos")]
const MACOS_APP_NAME: &str = "Tunnel Mate";

pub fn sync_autostart(enabled: bool, start_minimized: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return sync_macos_autostart(enabled, start_minimized);

    #[cfg(not(target_os = "macos"))]
    sync_legacy_autostart(enabled, start_minimized)
}

pub fn autostart_migration_needed() -> bool {
    #[cfg(target_os = "macos")]
    return macos_app_bundle().ok().flatten().is_some()
        && legacy_autostart(false)
            .and_then(|launcher| {
                launcher
                    .is_enabled()
                    .map_err(|error| format!("检查旧版开机启动状态失败：{error}"))
            })
            .unwrap_or(false);

    #[cfg(not(target_os = "macos"))]
    false
}

#[cfg(target_os = "macos")]
fn sync_macos_autostart(enabled: bool, start_minimized: bool) -> Result<(), String> {
    if macos_app_bundle()?.is_none() {
        return Err("请从打包后的 Tunnel Mate.app 配置开机启动".to_string());
    }
    let mut builder = AutoLaunchBuilder::new();
    builder.set_macos_launch_mode(MacOSLaunchMode::SMAppService);
    let modern = match builder.build() {
        Ok(launcher) => launcher,
        Err(AutoLaunchError::UnsupportedOS) => {
            return sync_pre_macos_13_autostart(enabled, start_minimized);
        }
        Err(error) => return Err(format!("无法配置开机启动：{error}")),
    };
    let modern_was_enabled = modern
        .is_enabled()
        .map_err(|error| format!("检查开机启动状态失败：{error}"))?;

    if enabled && !modern_was_enabled {
        modern
            .enable()
            .map_err(|error| format!("启用开机启动失败：{error}"))?;
    } else if !enabled && modern_was_enabled {
        modern
            .disable()
            .map_err(|error| format!("关闭开机启动失败：{error}"))?;
    }

    let legacy = legacy_autostart(start_minimized)?;
    let legacy_enabled = legacy
        .is_enabled()
        .map_err(|error| format!("检查旧版开机启动状态失败：{error}"))?;
    if legacy_enabled {
        if let Err(error) = legacy.disable() {
            if enabled && !modern_was_enabled {
                let _ = modern.disable();
            }
            return Err(format!("迁移旧版开机启动失败：{error}"));
        }
    }

    verify_autostart(&modern, enabled)
}

#[cfg(target_os = "macos")]
fn sync_pre_macos_13_autostart(enabled: bool, start_minimized: bool) -> Result<(), String> {
    let Some(login_item) = traditional_macos_login_item(start_minimized)? else {
        return sync_legacy_autostart(enabled, start_minimized);
    };
    let login_item_was_enabled = login_item
        .is_enabled()
        .map_err(|error| format!("检查开机启动状态失败：{error}"))?;

    if enabled && login_item_was_enabled {
        login_item
            .disable()
            .map_err(|error| format!("更新开机启动失败：{error}"))?;
        login_item
            .enable()
            .map_err(|error| format!("更新开机启动失败：{error}"))?;
    } else if enabled {
        login_item
            .enable()
            .map_err(|error| format!("启用开机启动失败：{error}"))?;
    } else if !enabled && login_item_was_enabled {
        login_item
            .disable()
            .map_err(|error| format!("关闭开机启动失败：{error}"))?;
    }

    let legacy = legacy_autostart(start_minimized)?;
    if legacy
        .is_enabled()
        .map_err(|error| format!("检查旧版开机启动状态失败：{error}"))?
    {
        if let Err(error) = legacy.disable() {
            if enabled && !login_item_was_enabled {
                let _ = login_item.disable();
            }
            return Err(format!("迁移旧版开机启动失败：{error}"));
        }
    }

    verify_autostart(&login_item, enabled)
}

#[cfg(target_os = "macos")]
fn traditional_macos_login_item(
    start_minimized: bool,
) -> Result<Option<auto_launcher::AutoLaunch>, String> {
    let Some(app_bundle) = macos_app_bundle()? else {
        return Ok(None);
    };
    let app_bundle = app_bundle
        .to_str()
        .ok_or_else(|| "应用路径不是有效的 UTF-8".to_string())?;
    let mut builder = AutoLaunchBuilder::new();
    builder
        .set_app_name(MACOS_APP_NAME)
        .set_app_path(app_bundle)
        .set_macos_launch_mode(MacOSLaunchMode::AppleScript);
    if start_minimized {
        builder.set_args(&["--minimized"]);
    }
    builder
        .build()
        .map(Some)
        .map_err(|error| format!("无法配置开机启动：{error}"))
}

#[cfg(target_os = "macos")]
fn macos_app_bundle() -> Result<Option<PathBuf>, String> {
    let executable = autostart_executable()?;
    Ok(executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .map(PathBuf::from))
}

fn sync_legacy_autostart(enabled: bool, start_minimized: bool) -> Result<(), String> {
    let launcher = legacy_autostart(start_minimized)?;
    let current = launcher
        .is_enabled()
        .map_err(|error| format!("检查开机启动状态失败：{error}"))?;
    if enabled && current {
        launcher
            .disable()
            .map_err(|error| format!("更新开机启动失败：{error}"))?;
        launcher
            .enable()
            .map_err(|error| format!("更新开机启动失败：{error}"))?;
    } else if enabled {
        launcher
            .enable()
            .map_err(|error| format!("启用开机启动失败：{error}"))?;
    } else if !enabled && current {
        launcher
            .disable()
            .map_err(|error| format!("关闭开机启动失败：{error}"))?;
    }
    verify_autostart(&launcher, enabled)
}

fn legacy_autostart(start_minimized: bool) -> Result<auto_launcher::AutoLaunch, String> {
    let executable = autostart_executable()?;
    let executable = executable
        .to_str()
        .ok_or_else(|| "应用路径不是有效的 UTF-8".to_string())?;
    let mut builder = AutoLaunchBuilder::new();
    builder
        .set_app_name(AUTOSTART_NAME)
        .set_app_path(executable);

    #[cfg(target_os = "macos")]
    builder.set_bundle_identifiers(&[AUTOSTART_NAME]);
    #[cfg(target_os = "windows")]
    builder.set_windows_enable_mode(WindowsEnableMode::CurrentUser);

    if start_minimized {
        builder.set_args(&["--minimized"]);
    }
    builder
        .build()
        .map_err(|error| format!("无法配置开机启动：{error}"))
}

fn verify_autostart(launcher: &auto_launcher::AutoLaunch, enabled: bool) -> Result<(), String> {
    let actual = launcher
        .is_enabled()
        .map_err(|error| format!("检查开机启动状态失败：{error}"))?;
    if actual != enabled {
        return Err(if enabled {
            "系统未能启用开机启动，请检查登录项权限后重试".to_string()
        } else {
            "系统未能关闭开机启动，请检查登录项权限后重试".to_string()
        });
    }

    Ok(())
}

fn autostart_executable() -> Result<PathBuf, String> {
    // AppImage runs from a temporary mount. Registering current_exe() would
    // leave an invalid path after the current session ends.
    #[cfg(target_os = "linux")]
    if let Some(appimage) = env::var_os("APPIMAGE").filter(|path| !path.is_empty()) {
        return Ok(PathBuf::from(appimage));
    }

    env::current_exe().map_err(|error| format!("无法确定应用路径：{error}"))
}

pub fn install_tray_event_handler(callback: impl Fn(String) + Send + Sync + 'static) {
    let callback = std::sync::Arc::new(callback);
    let menu_callback = callback.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        menu_callback(event.id.0.clone())
    }));

    // Windows users expect a normal left click on the notification-area icon to
    // restore the window. Linux AppIndicator does not emit tray click events, so
    // its menu keeps an explicit Open action instead.
    #[cfg(target_os = "windows")]
    TrayIconEvent::set_event_handler(Some(move |event| match event {
        TrayIconEvent::Click {
            id,
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        }
        | TrayIconEvent::DoubleClick {
            id,
            button: MouseButton::Left,
            ..
        } if id.0 == "main-tray" => callback("show".to_string()),
        _ => {}
    }));
}

pub fn build_tray(
    tunnels: &[Tunnel],
    statuses: &HashMap<String, TunnelStatus>,
    chinese: bool,
) -> Result<TrayIcon, String> {
    let menu = build_tray_menu(tunnels, statuses, chinese)?;

    #[cfg(target_os = "macos")]
    let icon_bytes = include_bytes!("../../../assets/icons/trayTemplate@2x.png").as_slice();
    #[cfg(not(target_os = "macos"))]
    let icon_bytes = include_bytes!("../../../assets/icons/32x32.png").as_slice();

    let decoded = image::load_from_memory(icon_bytes)
        .map_err(|error| format!("读取托盘图标失败：{error}"))?;
    let (width, height) = decoded.dimensions();
    let icon = Icon::from_rgba(decoded.into_rgba8().into_raw(), width, height)
        .map_err(|error| format!("创建托盘图标失败：{error}"))?;
    TrayIconBuilder::new()
        .with_id("main-tray")
        .with_tooltip("Tunnel Mate")
        .with_icon(icon)
        .with_icon_as_template(cfg!(target_os = "macos"))
        .with_menu(Box::new(menu))
        // macOS opens the status menu from either mouse button. Windows uses
        // left click to restore and right click for the menu. Linux ignores this
        // option and follows the desktop's AppIndicator behavior.
        .with_menu_on_left_click(!cfg!(target_os = "windows"))
        .with_menu_on_right_click(true)
        .build()
        .map_err(|error| format!("创建托盘图标失败：{error}"))
}

pub fn build_tray_menu(
    tunnels: &[Tunnel],
    statuses: &HashMap<String, TunnelStatus>,
    chinese: bool,
) -> Result<Menu, String> {
    let menu = Menu::new();
    menu.append(&IconMenuItem::with_id_and_native_icon(
        "show",
        if chinese {
            "打开 Tunnel Mate"
        } else {
            "Open Tunnel Mate"
        },
        true,
        Some(NativeIcon::Home),
        None,
    ))
    .map_err(|error| format!("创建托盘菜单失败：{error}"))?;
    menu.append(&PredefinedMenuItem::separator())
        .map_err(|error| format!("创建托盘菜单失败：{error}"))?;
    let running_count = tunnels
        .iter()
        .filter(|tunnel| matches!(statuses.get(&tunnel.id), Some(TunnelStatus::Running)))
        .count();
    let summary = if chinese {
        format!("隧道  ·  {running_count}/{} 已连接", tunnels.len())
    } else {
        format!("Tunnels  ·  {running_count}/{} connected", tunnels.len())
    };
    menu.append(&MenuItem::with_id("tunnels-summary", summary, false, None))
        .map_err(|error| format!("创建托盘菜单失败：{error}"))?;

    if tunnels.is_empty() {
        menu.append(&MenuItem::with_id(
            "empty",
            if chinese {
                "还没有隧道"
            } else {
                "No tunnels yet"
            },
            false,
            None,
        ))
        .map_err(|error| format!("创建托盘菜单失败：{error}"))?;
    } else {
        for tunnel in tunnels {
            let status = statuses.get(&tunnel.id).unwrap_or(&TunnelStatus::Stopped);
            let (suffix, icon) = match status {
                TunnelStatus::Running => (
                    if chinese { "已连接" } else { "Connected" },
                    NativeIcon::StatusAvailable,
                ),
                TunnelStatus::Connecting => (
                    if chinese { "连接中" } else { "Connecting" },
                    NativeIcon::StatusPartiallyAvailable,
                ),
                TunnelStatus::Reconnecting => (
                    if chinese { "重连中" } else { "Reconnecting" },
                    NativeIcon::StatusPartiallyAvailable,
                ),
                TunnelStatus::Failed => (
                    if chinese { "连接失败" } else { "Failed" },
                    NativeIcon::StatusUnavailable,
                ),
                TunnelStatus::Stopped => (
                    if chinese { "未连接" } else { "Disconnected" },
                    NativeIcon::StatusNone,
                ),
            };
            let label = format!("{}  ·  {suffix}", tunnel.name);
            menu.append(&IconMenuItem::with_id_and_native_icon(
                format!("tunnel:{}", tunnel.id),
                label,
                true,
                Some(icon),
                None,
            ))
            .map_err(|error| format!("创建托盘菜单失败：{error}"))?;
        }
    }
    menu.append(&PredefinedMenuItem::separator())
        .map_err(|error| format!("创建托盘菜单失败：{error}"))?;
    menu.append(&MenuItem::with_id(
        "quit",
        if chinese {
            "退出 Tunnel Mate"
        } else {
            "Quit Tunnel Mate"
        },
        true,
        None,
    ))
    .map_err(|error| format!("创建托盘菜单失败：{error}"))?;

    Ok(menu)
}
