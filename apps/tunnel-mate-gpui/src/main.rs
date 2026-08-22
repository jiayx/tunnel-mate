#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod i18n;
#[cfg(target_os = "macos")]
mod single_instance;
mod system;
mod text_input;

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::ops::Range;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use gpui::{
    actions, anchored, deferred, div, img, point, prelude::*, px, relative, rgb, rgba, size,
    uniform_list, Anchor, AnchoredPositionMode, App, Bounds, ClipboardItem, Context, Entity,
    FontWeight, IntoElement, KeyBinding, MouseButton, PathPromptOptions, RenderImage, Rgba,
    SharedString, Subscription, Task, UniformListScrollHandle, Window, WindowAppearance,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowOptions,
};
#[cfg(target_os = "macos")]
use gpui::{Menu as AppMenu, MenuItem as AppMenuItem, OsAction, SystemMenuType};
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use tunnel_core::diagnostics::{run_diagnostics, DiagnosticLanguage, DiagnosticStep};
use tunnel_core::event_logger::EventLogger;
use tunnel_core::event_logger::LogEvent;
use tunnel_core::{
    config::validate_tunnel, export_config_string, import_config_string, parse_ssh_config,
    AppConfig, ConfigStore, Endpoint, ForwardSpec, Group, LogSink, RuntimeEvent, SshHostConfig,
    Tunnel, TunnelManager, TunnelStatus,
};

use i18n::Language;
use text_input::TextInput;

const fn color(hex: u32) -> Rgba {
    Rgba {
        r: ((hex >> 16) & 0xff) as f32 / 255.0,
        g: ((hex >> 8) & 0xff) as f32 / 255.0,
        b: (hex & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

const fn glass(hex: u32, alpha: f32) -> Rgba {
    Rgba {
        r: ((hex >> 16) & 0xff) as f32 / 255.0,
        g: ((hex >> 8) & 0xff) as f32 / 255.0,
        b: (hex & 0xff) as f32 / 255.0,
        a: alpha,
    }
}

fn stop_scroll_propagation(_: &gpui::ScrollWheelEvent, _: &mut Window, cx: &mut App) {
    cx.stop_propagation();
}

fn modal_backdrop() -> gpui::Div {
    div()
        .absolute()
        .inset_0()
        .occlude()
        .on_scroll_wheel(stop_scroll_propagation)
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

fn downloads_dir() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|home| home.join("Downloads"))
        .filter(|path| path.is_dir())
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn ssh_host_matches(host: &SshHostConfig, current: &str) -> bool {
    let current = current.trim();
    !current.is_empty()
        && (host.host.eq_ignore_ascii_case(current)
            || host
                .host_name
                .as_deref()
                .is_some_and(|host_name| host_name.eq_ignore_ascii_case(current)))
}

fn notice_is_current(current_id: Option<u64>, expected_id: u64) -> bool {
    current_id == Some(expected_id)
}

fn progress_notice_clears_on_status(
    kind: NoticeKind,
    notice_tunnel_id: Option<&str>,
    updated_tunnel_id: &str,
    has_pending_tunnels: bool,
) -> bool {
    kind == NoticeKind::Progress
        && match notice_tunnel_id {
            Some(notice_tunnel_id) => notice_tunnel_id == updated_tunnel_id,
            None => !has_pending_tunnels,
        }
}

const APP_BG: Rgba = glass(0x0b1019, 0.90);
const SIDEBAR_BG: Rgba = glass(0x101622, 0.82);
const SURFACE: Rgba = glass(0x171d29, 0.90);
const SURFACE_HOVER: Rgba = glass(0x202a3a, 0.90);
const BORDER: Rgba = color(0x292d33);
const BORDER_SOFT: Rgba = color(0x20242a);
const TEXT: Rgba = color(0xf1f0ec);
const MUTED: Rgba = color(0x8b9099);
const MUTED_DARK: Rgba = color(0x616770);
const PRIMARY: Rgba = color(0x075bea);
const PRIMARY_HOVER: Rgba = color(0x2f76f6);
const PRIMARY_TEXT: Rgba = color(0xffffff);
const SUCCESS: Rgba = color(0x63cda7);
const WARNING: Rgba = color(0xd2a85e);
const DANGER: Rgba = color(0xdc747c);

actions!(
    tunnel_mate,
    [
        ShowAbout,
        OpenSettings,
        CloseWindow,
        QuitApplication,
        HideApplication,
        MinimizeWindow,
        ZoomWindow,
        ToggleFullScreen,
        BringAllToFront,
    ]
);

fn start_in_background_description(language: Language) -> &'static str {
    #[cfg(target_os = "macos")]
    {
        language.pick(
            "启动后不显示主窗口和 Dock 图标，仅显示菜单栏图标",
            "Start without the main window or Dock icon; show only the menu bar icon",
        )
    }
    #[cfg(target_os = "windows")]
    {
        language.pick(
            "启动后不显示主窗口，仅显示通知区域图标",
            "Start without the main window; show only the notification area icon",
        )
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        language.pick(
            "启动后不显示主窗口，仅显示系统托盘图标",
            "Start without the main window; show only the system tray icon",
        )
    }
}

fn keep_running_after_close_description(language: Language) -> &'static str {
    #[cfg(target_os = "macos")]
    {
        language.pick(
            "关闭主窗口后隐藏 Dock 图标，隧道继续运行；可从菜单栏重新打开",
            "Hide the Dock icon and keep tunnels running after closing the main window; reopen from the menu bar",
        )
    }
    #[cfg(target_os = "windows")]
    {
        language.pick(
            "关闭主窗口后在通知区域继续运行，隧道保持连接；可从通知区域重新打开",
            "Keep running in the notification area after closing the main window; reopen from the notification area",
        )
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        language.pick(
            "关闭主窗口后在系统托盘继续运行，隧道保持连接；可从托盘重新打开",
            "Keep running in the system tray after closing the main window; reopen from the tray",
        )
    }
}

fn close_to_tray_title(language: Language) -> &'static str {
    #[cfg(target_os = "macos")]
    {
        language.pick(
            "关闭窗口时隐藏 Dock 图标",
            "Hide the Dock icon when closing the window",
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        language.pick(
            "关闭窗口后继续运行",
            "Keep running after closing the window",
        )
    }
}

fn window_content_top_padding() -> gpui::Pixels {
    if cfg!(target_os = "macos") {
        px(38.0)
    } else {
        px(0.0)
    }
}

mod state;
pub(crate) use state::*;
#[path = "app/actions.rs"]
mod actions;
#[path = "app/backups.rs"]
mod backups;
#[path = "app/dialogs.rs"]
mod dialogs;
#[path = "app/lifecycle.rs"]
mod lifecycle;
#[path = "app/settings.rs"]
mod settings;
#[path = "app/sidebar.rs"]
mod sidebar;
#[path = "app/tunnel_form.rs"]
mod tunnel_form;
#[path = "app/tunnels.rs"]
mod tunnels;
#[path = "app/workspace.rs"]
mod workspace;

impl Render for TunnelMateApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ssh_picker_open = self
            .form
            .as_ref()
            .is_some_and(|form| form.ssh_picker_target.is_some());
        let root = div()
            .relative()
            .key_context("TunnelMate")
            .flex()
            .size_full()
            .pt(window_content_top_padding())
            .bg(APP_BG)
            .text_color(TEXT);
        #[cfg(target_os = "macos")]
        let root = root
            .on_action(|_: &CloseWindow, window, _| perform_window_close(window))
            .on_action(|_: &MinimizeWindow, window, _| window.minimize_window())
            .on_action(|_: &ZoomWindow, window, _| window.zoom_window())
            .on_action(|_: &ToggleFullScreen, window, _| window.toggle_fullscreen())
            .on_action(|_: &BringAllToFront, window, cx| {
                cx.activate(true);
                window.activate_window();
            });
        root.child(self.render_sidebar(cx))
            .child(self.render_workspace(cx))
            .when(self.notice.is_some(), |root| {
                root.child(self.render_notice(cx))
            })
            .when(self.form.is_some(), |root| {
                root.child(self.render_create_sheet(cx))
            })
            .when(ssh_picker_open, |root| {
                root.child(self.render_ssh_host_picker(cx))
            })
            .when(self.save_confirmation.is_some(), |root| {
                root.child(self.render_save_confirmation(cx))
            })
            .when(self.delete_confirmation.is_some(), |root| {
                root.child(self.render_delete_confirmation(cx))
            })
            .when(self.group_delete_confirmation.is_some(), |root| {
                root.child(self.render_group_delete_confirmation(cx))
            })
            .when(self.diagnostics.is_some(), |root| {
                root.child(self.render_diagnostics(cx))
            })
            .when(self.group_form.is_some(), |root| {
                root.child(self.render_group_form(cx))
            })
            .when(self.settings_form.is_some(), |root| {
                root.child(self.render_settings(cx))
            })
            .when(self.pending_import.is_some(), |root| {
                root.child(self.render_import_confirmation(cx))
            })
            .when(self.auth_prompt.is_some(), |root| {
                root.child(self.render_auth_prompt(cx))
            })
            .when(self.about_open, |root| root.child(self.render_about(cx)))
    }
}

fn parse_host_key_prompt(
    message: &str,
) -> Option<(HostKeyIssue, String, u16, String, Vec<String>)> {
    let (issue, value) = if let Some(value) = message.strip_prefix("HOST_KEY_NOT_TRUSTED|") {
        (HostKeyIssue::Unknown, value)
    } else if let Some(value) = message.strip_prefix("HOST_KEY_CHANGED|") {
        (HostKeyIssue::Changed, value)
    } else {
        (
            HostKeyIssue::Revoked,
            message.strip_prefix("HOST_KEY_REVOKED|")?,
        )
    };
    let mut parts = value.splitn(4, '|');
    let host = parts.next()?.to_string();
    let port = parts.next()?.parse().ok()?;
    let fingerprint = parts.next()?.to_string();
    let saved_fingerprints = parts
        .next()
        .unwrap_or_default()
        .split(',')
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect();
    (!host.is_empty() && !fingerprint.is_empty()).then_some((
        issue,
        host,
        port,
        fingerprint,
        saved_fingerprints,
    ))
}

mod platform;
use platform::*;

fn main() {
    #[cfg(target_os = "macos")]
    let _instance_guard = match single_instance::SingleInstanceGuard::acquire() {
        Ok(Some(guard)) => guard,
        Ok(None) => return,
        Err(error) => {
            eprintln!("Tunnel Mate could not acquire its single-instance lock: {error}");
            return;
        }
    };

    let minimized_arg = std::env::args().any(|arg| arg == "--minimized");
    let application = gpui_platform::application();
    application.on_reopen(|cx| {
        set_dock_visible(true);
        cx.activate(true);
        for handle in cx.windows() {
            let _ = handle.update(cx, |_, window, _| {
                show_window(window);
                window.activate_window();
            });
        }
    });
    application.run(move |cx: &mut App| {
        let start_minimized = minimized_arg
            || (launched_as_login_item()
                && ConfigStore::new()
                    .load_config()
                    .is_ok_and(|config| config.settings.start_minimized));
        if start_minimized {
            set_dock_visible(false);
        }
        text_input::init(cx);
        cx.set_window_appearance(Some(WindowAppearance::Dark));
        let window_size = size(px(920.0), px(620.0));
        let bounds = cx
            .primary_display()
            .map(|display| Bounds::centered_at(display.visible_bounds().center(), window_size))
            .unwrap_or_else(|| Bounds::centered(None, window_size, cx));
        let window_handle = cx
            .open_window(platform_window_options(bounds), |window, cx| {
                let app = cx.new(TunnelMateApp::load);
                let weak = app.downgrade();
                window.on_window_should_close(cx, move |window, cx| {
                    if let Some(app) = weak.upgrade() {
                        app.update(cx, |app, cx| app.request_close(window, cx));
                    } else {
                        cx.quit();
                    }
                    false
                });
                app
            })
            .expect("failed to open Tunnel Mate window");
        let app = window_handle
            .entity(cx)
            .expect("failed to access Tunnel Mate root view");
        register_global_actions(cx, window_handle, app);
        // On macOS tray initialization replaces NSApp's main menu, so native behavior is
        // installed afterwards. Other platforms only register their conventional shortcuts.
        install_native_behavior(cx, Language::system());
        if start_minimized {
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            let _ = window_handle.update(cx, |_, window, _| hide_window(window));
            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            cx.hide();
        } else {
            cx.activate(true);
            let _ = window_handle.update(cx, |_, window, _| {
                show_window(window);
                window.activate_window();
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{
        notice_is_current, parse_host_key_prompt, progress_notice_clears_on_status,
        ssh_host_matches, HostKeyIssue, Language, NoticeKind, SshHostConfig,
    };

    #[test]
    fn only_dismisses_the_notice_that_started_the_timer() {
        assert!(notice_is_current(Some(7), 7));
        assert!(!notice_is_current(Some(8), 7));
        assert!(!notice_is_current(None, 7));
    }

    #[test]
    fn status_changes_only_dismiss_the_matching_progress_notice() {
        assert!(progress_notice_clears_on_status(
            NoticeKind::Progress,
            Some("a"),
            "a",
            true
        ));
        assert!(!progress_notice_clears_on_status(
            NoticeKind::Progress,
            Some("b"),
            "a",
            false
        ));
        assert!(!progress_notice_clears_on_status(
            NoticeKind::Persistent,
            Some("a"),
            "a",
            false
        ));
        assert!(!progress_notice_clears_on_status(
            NoticeKind::Progress,
            None,
            "a",
            true
        ));
        assert!(progress_notice_clears_on_status(
            NoticeKind::Progress,
            None,
            "a",
            false
        ));
    }

    #[test]
    fn parses_host_key_intervention_message() {
        assert_eq!(
            parse_host_key_prompt("HOST_KEY_NOT_TRUSTED|example.com|2222|SHA256:abc"),
            Some((
                HostKeyIssue::Unknown,
                "example.com".to_string(),
                2222,
                "SHA256:abc".to_string(),
                vec![]
            ))
        );
        assert_eq!(
            parse_host_key_prompt(
                "HOST_KEY_CHANGED|example.com|22|SHA256:new|SHA256:old,SHA256:older"
            )
            .unwrap()
            .4,
            vec!["SHA256:old", "SHA256:older"]
        );
        assert!(parse_host_key_prompt("ordinary error").is_none());
    }

    #[test]
    fn selects_chinese_only_for_chinese_system_locales() {
        assert_eq!(Language::from_locale("zh-CN"), Language::Zh);
        assert_eq!(Language::from_locale("zh_Hant_TW"), Language::Zh);
        assert_eq!(Language::from_locale("en-US"), Language::En);
    }

    #[test]
    fn matches_ssh_config_alias_or_resolved_host_case_insensitively() {
        let host = SshHostConfig {
            host: "Production-DB".into(),
            host_name: Some("db.internal.example.com".into()),
            ..Default::default()
        };
        assert!(ssh_host_matches(&host, "production-db"));
        assert!(ssh_host_matches(&host, "DB.INTERNAL.EXAMPLE.COM"));
        assert!(!ssh_host_matches(&host, "other.example.com"));
        assert!(!ssh_host_matches(&host, ""));
    }
}
