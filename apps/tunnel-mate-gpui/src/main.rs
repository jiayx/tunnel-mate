#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod i18n;
mod system;
mod text_input;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use gpui::{
    actions, anchored, deferred, div, img, point, prelude::*, px, relative, rgb, rgba, size,
    Anchor, AnchoredPositionMode, App, Bounds, ClipboardItem, Context, Entity, FontWeight,
    IntoElement, KeyBinding, MouseButton, PathPromptOptions, RenderImage, Rgba, SharedString,
    Subscription, Task, Window, WindowAppearance, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowOptions,
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

fn close_to_tray_description(language: Language) -> &'static str {
    #[cfg(target_os = "macos")]
    {
        language.pick(
            "点关闭按钮只隐藏窗口，隧道继续运行；可从 Dock 或菜单栏恢复",
            "Hide the window without stopping tunnels; restore it from the Dock or menu bar",
        )
    }
    #[cfg(target_os = "windows")]
    {
        language.pick(
            "点关闭按钮只隐藏窗口，隧道继续运行；可从任务栏通知区域恢复",
            "Hide the window without stopping tunnels; restore it from the notification area",
        )
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        language.pick(
            "点关闭按钮只隐藏窗口，隧道继续运行；可从系统托盘或状态区恢复",
            "Hide the window without stopping tunnels; restore it from the system tray or status area",
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

#[derive(Clone, PartialEq, Eq)]
enum TunnelFilter {
    All,
    Active,
    Activity,
    Group(String),
}

enum AppMessage {
    Runtime(RuntimeEvent),
    Log {
        tunnel_id: String,
        message: String,
    },
    OperationError {
        tunnel_name: String,
        message: String,
    },
    Diagnostics(Vec<DiagnosticStep>),
    ImportSelected(AppConfig),
    ImportFailed(String),
    ConfigImported(AppConfig),
    DeleteReady(String),
    DeleteFailed {
        tunnel_name: String,
        message: String,
    },
    FileOperation(String),
    PrivateKeySelected(String),
    Tray(String),
    QuitReady,
    HostTrusted(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ForwardKind {
    Local,
    Remote,
    Socks5,
}

#[derive(Clone, Copy)]
enum SettingToggle {
    Launch,
    Minimized,
    CloseToTray,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SshPickerTarget {
    Primary,
    JumpHost,
}

struct TunnelForm {
    editing_id: Option<String>,
    validation_error: Option<SharedString>,
    kind: ForwardKind,
    advanced: bool,
    start_after_save: bool,
    auto_reconnect: bool,
    start_with_app: bool,
    jump_enabled: bool,
    jump_host_id: Option<String>,
    group_id: Option<String>,
    group_menu_open: bool,
    name: Entity<TextInput>,
    description: Entity<TextInput>,
    ssh_host: Entity<TextInput>,
    ssh_port: Entity<TextInput>,
    ssh_user: Entity<TextInput>,
    identity_file: Entity<TextInput>,
    ssh_password: Entity<TextInput>,
    jump_host: Entity<TextInput>,
    jump_port: Entity<TextInput>,
    jump_user: Entity<TextInput>,
    jump_identity_file: Entity<TextInput>,
    jump_password: Entity<TextInput>,
    listen_host: Entity<TextInput>,
    listen_port: Entity<TextInput>,
    target_host: Entity<TextInput>,
    target_port: Entity<TextInput>,
    retry_count: Entity<TextInput>,
    retry_interval: Entity<TextInput>,
    ssh_hosts: Vec<SshHostConfig>,
    ssh_picker_target: Option<SshPickerTarget>,
}

struct SettingsForm {
    launch_on_startup: bool,
    start_minimized: bool,
    close_to_tray: bool,
    keep_alive: Entity<TextInput>,
    connect_timeout: Entity<TextInput>,
    ssh_config_path: Entity<TextInput>,
}

struct GroupForm {
    editing_id: Option<String>,
    name: Entity<TextInput>,
    description: Entity<TextInput>,
}

enum AuthPrompt {
    HostKey {
        tunnel_id: String,
        issue: HostKeyIssue,
        host: String,
        port: u16,
        fingerprint: String,
        saved_fingerprints: Vec<String>,
        confirm_replace: bool,
    },
    Passphrase {
        tunnel_id: String,
        input: Entity<TextInput>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostKeyIssue {
    Unknown,
    Changed,
    Revoked,
}

impl TunnelForm {
    fn new(tunnel: Option<&Tunnel>, language: Language, cx: &mut Context<TunnelMateApp>) -> Self {
        let input = |placeholder: &'static str, value: String, cx: &mut Context<TunnelMateApp>| {
            cx.new(|cx| TextInput::new(cx, placeholder, value))
        };
        let (kind, listen, target) = tunnel
            .map(|tunnel| match &tunnel.forward {
                ForwardSpec::Local { listen, target } => {
                    (ForwardKind::Local, listen.clone(), Some(target.clone()))
                }
                ForwardSpec::Remote { listen, target } => {
                    (ForwardKind::Remote, listen.clone(), Some(target.clone()))
                }
                ForwardSpec::Socks5 { listen } => (ForwardKind::Socks5, listen.clone(), None),
            })
            .unwrap_or((
                ForwardKind::Local,
                Endpoint {
                    host: "127.0.0.1".into(),
                    port: 5432,
                },
                Some(Endpoint {
                    host: "127.0.0.1".into(),
                    port: 5432,
                }),
            ));
        let target = target.unwrap_or(Endpoint {
            host: "127.0.0.1".into(),
            port: 80,
        });
        Self {
            editing_id: tunnel.map(|tunnel| tunnel.id.clone()),
            validation_error: None,
            kind,
            advanced: false,
            start_after_save: tunnel.is_none(),
            auto_reconnect: tunnel.map(|tunnel| tunnel.auto_reconnect).unwrap_or(true),
            start_with_app: tunnel.map(|tunnel| tunnel.start_with_app).unwrap_or(true),
            jump_enabled: tunnel
                .map(|tunnel| tunnel.jump_host_enabled)
                .unwrap_or(false),
            jump_host_id: tunnel.and_then(|tunnel| tunnel.jump_host_id.clone()),
            group_id: tunnel.and_then(|tunnel| tunnel.group_id.clone()),
            group_menu_open: false,
            name: input(
                language.pick("例如：生产数据库", "e.g. Production database"),
                tunnel.map(|tunnel| tunnel.name.clone()).unwrap_or_default(),
                cx,
            ),
            description: input(
                language.pick("用途说明（可选）", "Description (optional)"),
                tunnel
                    .and_then(|tunnel| tunnel.description.clone())
                    .unwrap_or_default(),
                cx,
            ),
            ssh_host: input(
                "ssh.example.com",
                tunnel
                    .map(|tunnel| tunnel.ssh_host.clone())
                    .unwrap_or_default(),
                cx,
            ),
            ssh_port: input(
                "22",
                tunnel
                    .map(|tunnel| tunnel.ssh_port.to_string())
                    .unwrap_or_else(|| "22".into()),
                cx,
            ),
            ssh_user: input(
                "root",
                tunnel
                    .map(|tunnel| tunnel.ssh_user.clone())
                    .unwrap_or_default(),
                cx,
            ),
            identity_file: input(
                language.pick(
                    "~/.ssh/id_ed25519（留空则自动选择）",
                    "~/.ssh/id_ed25519 (blank for automatic)",
                ),
                tunnel
                    .and_then(|tunnel| tunnel.ssh_identity_file.clone())
                    .unwrap_or_default(),
                cx,
            ),
            ssh_password: cx.new(|cx| {
                TextInput::new_secure(
                    cx,
                    language.pick(
                        "密码（可选，保存到系统钥匙串）",
                        "Password (optional, stored in Keychain)",
                    ),
                    tunnel
                        .and_then(|t| t.ssh_password.clone())
                        .unwrap_or_default(),
                )
            }),
            jump_host: input(
                "jump.example.com",
                tunnel.and_then(|t| t.jump_host.clone()).unwrap_or_default(),
                cx,
            ),
            jump_port: input(
                "22",
                tunnel.and_then(|t| t.jump_port).unwrap_or(22).to_string(),
                cx,
            ),
            jump_user: input(
                "root",
                tunnel.and_then(|t| t.jump_user.clone()).unwrap_or_default(),
                cx,
            ),
            jump_identity_file: input(
                "~/.ssh/id_ed25519",
                tunnel
                    .and_then(|t| t.jump_identity_file.clone())
                    .unwrap_or_default(),
                cx,
            ),
            jump_password: cx.new(|cx| {
                TextInput::new_secure(
                    cx,
                    language.pick("跳板机密码（可选）", "Jump host password (optional)"),
                    tunnel
                        .and_then(|t| t.jump_password.clone())
                        .unwrap_or_default(),
                )
            }),
            listen_host: input("127.0.0.1", listen.host, cx),
            listen_port: input("5432", listen.port.to_string(), cx),
            target_host: input("db.internal", target.host, cx),
            target_port: input("5432", target.port.to_string(), cx),
            retry_count: input(
                "3",
                tunnel
                    .map(|tunnel| tunnel.retry_count.to_string())
                    .unwrap_or_else(|| "3".into()),
                cx,
            ),
            retry_interval: input(
                "5",
                tunnel
                    .map(|tunnel| tunnel.retry_interval.to_string())
                    .unwrap_or_else(|| "5".into()),
                cx,
            ),
            ssh_hosts: Vec::new(),
            ssh_picker_target: None,
        }
    }
}

struct TunnelMateApp {
    language: Language,
    logo: Arc<RenderImage>,
    config: AppConfig,
    search: Entity<TextInput>,
    filter: TunnelFilter,
    selected_tunnel: Option<String>,
    form: Option<TunnelForm>,
    notice: Option<SharedString>,
    load_error: Option<SharedString>,
    statuses: HashMap<String, TunnelStatus>,
    pending_starts: HashSet<String>,
    logs: HashMap<String, Vec<String>>,
    events: Vec<LogEvent>,
    diagnostics: Option<Vec<DiagnosticStep>>,
    settings_form: Option<SettingsForm>,
    pending_import: Option<AppConfig>,
    group_form: Option<GroupForm>,
    save_confirmation: Option<Tunnel>,
    delete_confirmation: Option<String>,
    pending_delete: Option<String>,
    auth_prompt: Option<AuthPrompt>,
    about_open: bool,
    manager: Arc<Mutex<TunnelManager>>,
    runtime: Arc<Runtime>,
    messages: async_channel::Sender<AppMessage>,
    _event_task: Task<()>,
    _keystroke_subscription: Subscription,
    _tray: Option<tray_icon::TrayIcon>,
}

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
        div()
            .relative()
            .key_context("TunnelMate")
            .flex()
            .size_full()
            .pt(window_content_top_padding())
            .bg(APP_BG)
            .text_color(TEXT)
            .child(self.render_sidebar(cx))
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

#[cfg(target_os = "macos")]
fn install_native_behavior(cx: &mut App, language: Language) {
    cx.bind_keys([
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-q", QuitApplication, None),
        KeyBinding::new("cmd-h", HideApplication, None),
        KeyBinding::new("cmd-m", MinimizeWindow, None),
        KeyBinding::new("ctrl-cmd-f", ToggleFullScreen, None),
    ]);

    cx.set_menus([
        AppMenu::new("Tunnel Mate").items([
            AppMenuItem::action(
                language.pick("关于 Tunnel Mate", "About Tunnel Mate"),
                ShowAbout,
            ),
            AppMenuItem::separator(),
            AppMenuItem::action(language.pick("设置…", "Settings…"), OpenSettings),
            AppMenuItem::separator(),
            AppMenuItem::os_submenu(language.pick("服务", "Services"), SystemMenuType::Services),
            AppMenuItem::separator(),
            AppMenuItem::action(
                language.pick("隐藏 Tunnel Mate", "Hide Tunnel Mate"),
                HideApplication,
            ),
            AppMenuItem::separator(),
            AppMenuItem::action(
                language.pick("退出 Tunnel Mate", "Quit Tunnel Mate"),
                QuitApplication,
            ),
        ]),
        AppMenu::new(language.pick("文件", "File")).items([AppMenuItem::action(
            language.pick("关闭窗口", "Close Window"),
            CloseWindow,
        )]),
        AppMenu::new(language.pick("编辑", "Edit")).items([
            AppMenuItem::os_action(language.pick("剪切", "Cut"), text_input::Cut, OsAction::Cut),
            AppMenuItem::os_action(
                language.pick("复制", "Copy"),
                text_input::Copy,
                OsAction::Copy,
            ),
            AppMenuItem::os_action(
                language.pick("粘贴", "Paste"),
                text_input::Paste,
                OsAction::Paste,
            ),
            AppMenuItem::separator(),
            AppMenuItem::os_action(
                language.pick("全选", "Select All"),
                text_input::SelectAll,
                OsAction::SelectAll,
            ),
        ]),
        AppMenu::new(language.pick("窗口", "Window")).items([
            AppMenuItem::action(language.pick("最小化", "Minimize"), MinimizeWindow),
            AppMenuItem::action(language.pick("缩放", "Zoom"), ZoomWindow),
            AppMenuItem::action(
                language.pick("进入全屏幕", "Enter Full Screen"),
                ToggleFullScreen,
            ),
            AppMenuItem::separator(),
            AppMenuItem::action(
                language.pick("前置全部窗口", "Bring All to Front"),
                BringAllToFront,
            ),
        ]),
    ]);
}

#[cfg(not(target_os = "macos"))]
fn install_native_behavior(cx: &mut App, _language: Language) {
    cx.bind_keys([
        KeyBinding::new("ctrl-,", OpenSettings, None),
        KeyBinding::new("ctrl-w", CloseWindow, None),
        KeyBinding::new("ctrl-q", QuitApplication, None),
        KeyBinding::new("f11", ToggleFullScreen, None),
    ]);
}

fn platform_window_options(bounds: Bounds<gpui::Pixels>) -> WindowOptions {
    let mut options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(800.0), px(580.0))),
        ..Default::default()
    };

    #[cfg(target_os = "macos")]
    {
        options.window_background = WindowBackgroundAppearance::Blurred;
        options.titlebar = Some(gpui::TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: Some(point(px(16.0), px(16.0))),
        });
    }

    #[cfg(target_os = "windows")]
    {
        options.window_background = WindowBackgroundAppearance::MicaBackdrop;
        options.titlebar = Some(gpui::TitlebarOptions {
            title: Some("Tunnel Mate".into()),
            appears_transparent: false,
            traffic_light_position: None,
        });
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        options.window_background = WindowBackgroundAppearance::Opaque;
        options.titlebar = Some(gpui::TitlebarOptions {
            title: Some("Tunnel Mate".into()),
            appears_transparent: false,
            traffic_light_position: None,
        });
        options.app_id = Some("com.jiayx.tunnel-mate".to_string());
        options.icon = image::load_from_memory(include_bytes!("../../../assets/icons/128x128.png"))
            .ok()
            .map(|image| Arc::new(image.into_rgba8()));
    }

    options
}

fn register_global_actions(
    cx: &mut App,
    window_handle: WindowHandle<TunnelMateApp>,
    app: Entity<TunnelMateApp>,
) {
    let weak = app.downgrade();
    cx.on_action(move |_: &ShowAbout, cx| {
        let _ = weak.update(cx, |app, cx| app.show_about(cx));
    });

    let weak = app.downgrade();
    cx.on_action(move |_: &OpenSettings, cx| {
        let _ = weak.update(cx, |app, cx| app.open_settings(cx));
    });

    let weak = app.downgrade();
    cx.on_action(move |_: &CloseWindow, cx| {
        let _ = weak.update(cx, |app, cx| app.request_close(cx));
    });

    let weak = app.downgrade();
    cx.on_action(move |_: &QuitApplication, cx| {
        let _ = weak.update(cx, |app, _| app.request_quit());
    });

    cx.on_action(|_: &HideApplication, cx| cx.hide());

    let handle = window_handle;
    cx.on_action(move |_: &MinimizeWindow, cx| {
        let _ = handle.update(cx, |_, window, _| window.minimize_window());
    });

    let handle = window_handle;
    cx.on_action(move |_: &ZoomWindow, cx| {
        let _ = handle.update(cx, |_, window, _| window.zoom_window());
    });

    let handle = window_handle;
    cx.on_action(move |_: &ToggleFullScreen, cx| {
        let _ = handle.update(cx, |_, window, _| window.toggle_fullscreen());
    });

    cx.on_action(move |_: &BringAllToFront, cx| {
        cx.activate(true);
        let _ = window_handle.update(cx, |_, window, _| window.activate_window());
    });
}

fn main() {
    let application = gpui_platform::application();
    application.on_reopen(|cx| {
        cx.activate(true);
        for handle in cx.windows() {
            let _ = handle.update(cx, |_, window, _| window.activate_window());
        }
    });
    application.run(|cx: &mut App| {
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
                window.on_window_should_close(cx, move |_, cx| {
                    if let Some(app) = weak.upgrade() {
                        app.update(cx, |app, cx| app.request_close(cx));
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
        let minimized_arg = std::env::args().any(|arg| arg == "--minimized");
        if minimized_arg {
            cx.hide();
        } else {
            cx.activate(true);
            let _ = window_handle.update(cx, |_, window, _| window.activate_window());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{parse_host_key_prompt, ssh_host_matches, HostKeyIssue, Language, SshHostConfig};

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
