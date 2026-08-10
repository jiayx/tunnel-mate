use std::collections::HashMap;
use std::env;

use auto_launcher::AutoLaunchBuilder;
use image::GenericImageView;
use tray_icon::menu::{IconMenuItem, Menu, MenuEvent, MenuItem, NativeIcon, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use tunnel_core::{Tunnel, TunnelStatus};

pub fn sync_autostart(enabled: bool, start_minimized: bool) -> Result<(), String> {
    let executable = env::current_exe().map_err(|error| format!("无法确定应用路径：{error}"))?;
    let executable = executable
        .to_str()
        .ok_or_else(|| "应用路径不是有效的 UTF-8".to_string())?;
    let mut builder = AutoLaunchBuilder::new();
    builder.set_app_name("Tunnel Mate").set_app_path(executable);
    if start_minimized {
        builder.set_args(&["--minimized"]);
    }
    let launcher = builder
        .build()
        .map_err(|error| format!("无法配置开机启动：{error}"))?;
    if enabled {
        launcher
            .enable()
            .map_err(|error| format!("启用开机启动失败：{error}"))
    } else {
        launcher
            .disable()
            .map_err(|error| format!("关闭开机启动失败：{error}"))
    }
}

pub fn install_tray_event_handler(callback: impl Fn(String) + Send + Sync + 'static) {
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| callback(event.id.0.clone())));
}

pub fn build_tray(
    tunnels: &[Tunnel],
    statuses: &HashMap<String, TunnelStatus>,
    chinese: bool,
) -> Result<TrayIcon, String> {
    let menu = build_tray_menu(tunnels, statuses, chinese)?;

    let decoded = image::load_from_memory(include_bytes!(
        "../../../src-tauri/icons/trayTemplate@2x.png"
    ))
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
        .with_menu_on_left_click(true)
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
