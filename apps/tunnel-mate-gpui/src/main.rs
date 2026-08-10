mod system;
mod text_input;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use gpui::{
    actions, anchored, deferred, div, img, point, prelude::*, px, relative, rgb, rgba, size,
    Anchor, AnchoredPositionMode, App, Bounds, ClipboardItem, Context, Entity, FontWeight,
    IntoElement, KeyBinding, Menu as AppMenu, MenuItem as AppMenuItem, MouseButton, OsAction,
    PathPromptOptions, RenderImage, Rgba, SharedString, Subscription, SystemMenuType, Task, Window,
    WindowAppearance, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowOptions,
};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    Zh,
    En,
}

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

impl Language {
    fn system() -> Self {
        let locale = std::env::var("TUNNEL_MATE_LANG")
            .ok()
            .unwrap_or_else(|| sys_locale::get_locale().unwrap_or_default());
        Self::from_locale(&locale)
    }

    fn from_locale(locale: &str) -> Self {
        if locale.to_ascii_lowercase().starts_with("zh") {
            Self::Zh
        } else {
            Self::En
        }
    }

    fn pick(self, zh: &'static str, en: &'static str) -> &'static str {
        if self == Self::Zh {
            zh
        } else {
            en
        }
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
        host: String,
        port: u16,
        fingerprint: String,
    },
    Passphrase {
        tunnel_id: String,
        input: Entity<TextInput>,
    },
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
    auth_prompt: Option<AuthPrompt>,
    about_open: bool,
    manager: Arc<Mutex<TunnelManager>>,
    runtime: Arc<Runtime>,
    messages: async_channel::Sender<AppMessage>,
    _event_task: Task<()>,
    _keystroke_subscription: Subscription,
    _tray: Option<tray_icon::TrayIcon>,
}

impl TunnelMateApp {
    fn load(cx: &mut Context<Self>) -> Self {
        let language = Language::system();
        let (config, load_error) = match ConfigStore::new().load_config() {
            Ok(config) => (config, None),
            Err(error) => (
                AppConfig::default(),
                Some(format!("配置读取失败：{error}").into()),
            ),
        };

        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("tunnel-mate")
                .build()
                .expect("failed to create Tunnel Mate runtime"),
        );
        let (messages, receiver) = async_channel::unbounded();
        let event_sender = messages.clone();
        let manager = Arc::new(Mutex::new(TunnelManager::with_event_sink(Arc::new(
            move |event| {
                let _ = event_sender.try_send(AppMessage::Runtime(event));
            },
        ))));
        let mut statuses = config
            .tunnels
            .iter()
            .map(|tunnel| (tunnel.id.clone(), TunnelStatus::Stopped))
            .collect();
        let tray_sender = messages.clone();
        system::install_tray_event_handler(move |id| {
            let _ = tray_sender.try_send(AppMessage::Tray(id));
        });
        let tray = system::build_tray(&config.tunnels, &statuses, language == Language::Zh).ok();
        let event_task = cx.spawn(async move |this, cx| {
            while let Ok(message) = receiver.recv().await {
                if this
                    .update(cx, |this, cx| {
                        this.handle_message(message, cx);
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        });

        let events = EventLogger::new().get_events().unwrap_or_default();
        for tunnel in config
            .tunnels
            .iter()
            .filter(|tunnel| tunnel.start_with_app)
            .cloned()
        {
            statuses.insert(tunnel.id.clone(), TunnelStatus::Connecting);
            let startup_manager = manager.clone();
            let startup_sender = messages.clone();
            let tunnel_name = tunnel.name.clone();
            runtime.spawn(async move {
                if let Err(message) =
                    TunnelManager::start_tunnel_silent(startup_manager, tunnel).await
                {
                    let _ = startup_sender
                        .send(AppMessage::OperationError {
                            tunnel_name,
                            message,
                        })
                        .await;
                }
            });
        }

        let selected_tunnel = config.tunnels.first().map(|tunnel| tunnel.id.clone());
        let search_placeholder =
            language.pick("搜索名称、主机或地址", "Search name, host, or address");
        let search = cx.new(|cx| TextInput::new(cx, search_placeholder, ""));
        cx.observe(&search, |_, _, cx| cx.notify()).detach();
        let mut logo = image::load_from_memory(include_bytes!("../../../assets/icons/128x128.png"))
            .expect("embedded app icon must be valid")
            .into_rgba8();
        for pixel in logo.pixels_mut() {
            pixel.0.swap(0, 2);
        }
        let keystroke_subscription = cx.observe_keystrokes(|this, event, _, cx| {
            if event.keystroke.key == "escape" {
                this.dismiss(cx);
            }
        });
        Self {
            language,
            logo: Arc::new(RenderImage::new(vec![image::Frame::new(logo)])),
            config,
            search,
            filter: TunnelFilter::All,
            selected_tunnel,
            form: None,
            notice: None,
            load_error,
            statuses,
            pending_starts: HashSet::new(),
            logs: HashMap::new(),
            events,
            diagnostics: None,
            settings_form: None,
            pending_import: None,
            group_form: None,
            save_confirmation: None,
            auth_prompt: None,
            about_open: false,
            manager,
            runtime,
            messages,
            _event_task: event_task,
            _keystroke_subscription: keystroke_subscription,
            _tray: tray,
        }
    }

    fn handle_message(&mut self, message: AppMessage, cx: &mut Context<Self>) {
        match message {
            AppMessage::Runtime(RuntimeEvent::Status(payload)) => {
                let mut intervention = false;
                if let Some(message) = payload.message.as_deref() {
                    if message == "PASSPHRASE_REQUIRED" {
                        intervention = true;
                        self.auth_prompt = Some(AuthPrompt::Passphrase {
                            tunnel_id: payload.tunnel_id.clone(),
                            input: cx.new(|cx| {
                                TextInput::new_secure(
                                    cx,
                                    self.language.pick("私钥口令", "Private key passphrase"),
                                    "",
                                )
                            }),
                        });
                    } else if let Some((host, port, fingerprint)) = parse_host_key_prompt(message) {
                        intervention = true;
                        self.auth_prompt = Some(AuthPrompt::HostKey {
                            tunnel_id: payload.tunnel_id.clone(),
                            host,
                            port,
                            fingerprint,
                        });
                    }
                }
                let tunnel_id = payload.tunnel_id;
                let status = payload.status;
                self.statuses.insert(tunnel_id.clone(), status.clone());
                let restart_after_stop =
                    status == TunnelStatus::Stopped && self.pending_starts.remove(&tunnel_id);
                match status {
                    TunnelStatus::Running | TunnelStatus::Stopped => self.notice = None,
                    TunnelStatus::Failed if !intervention => {
                        if let Some(message) = payload.message {
                            self.notice = Some(message.into());
                        }
                    }
                    _ => {}
                }
                self.refresh_tray();
                if restart_after_stop {
                    self.start_tunnel_with_passphrase(tunnel_id, None, cx);
                }
            }
            AppMessage::Runtime(RuntimeEvent::Activity(event)) => {
                self.events.retain(|existing| existing.id != event.id);
                self.events.push(event);
                if self.events.len() > 1000 {
                    self.events.drain(..self.events.len() - 1000);
                }
            }
            AppMessage::Log { tunnel_id, message } => {
                let logs = self.logs.entry(tunnel_id).or_default();
                logs.push(message);
                if logs.len() > 2000 {
                    logs.drain(..logs.len() - 2000);
                }
            }
            AppMessage::OperationError {
                tunnel_name,
                message,
            } => {
                self.notice = Some(
                    if self.language == Language::Zh {
                        format!("“{tunnel_name}”操作失败：{message}")
                    } else {
                        format!("Operation failed for “{tunnel_name}”: {message}")
                    }
                    .into(),
                );
            }
            AppMessage::Diagnostics(steps) => {
                self.diagnostics = Some(steps);
                self.notice = None;
            }
            AppMessage::ImportSelected(config) => {
                if self.statuses.values().any(|status| {
                    matches!(
                        status,
                        TunnelStatus::Running
                            | TunnelStatus::Connecting
                            | TunnelStatus::Reconnecting
                    )
                }) {
                    self.pending_import = Some(config);
                } else {
                    self.commit_import(config);
                }
            }
            AppMessage::ImportFailed(message) => {
                self.settings_form = None;
                self.pending_import = None;
                self.notice = Some(message.into());
            }
            AppMessage::ConfigImported(config) => {
                self.config = config;
                self.statuses = self
                    .config
                    .tunnels
                    .iter()
                    .map(|tunnel| (tunnel.id.clone(), TunnelStatus::Stopped))
                    .collect();
                self.selected_tunnel = self.config.tunnels.first().map(|tunnel| tunnel.id.clone());
                self.filter = TunnelFilter::All;
                self.settings_form = None;
                self.pending_import = None;
                self.refresh_tray();
                let auto_start_ids = self
                    .config
                    .tunnels
                    .iter()
                    .filter(|tunnel| tunnel.start_with_app)
                    .map(|tunnel| tunnel.id.clone())
                    .collect::<Vec<_>>();
                for tunnel_id in &auto_start_ids {
                    self.start_tunnel_with_passphrase(tunnel_id.clone(), None, cx);
                }
                self.notice = Some(
                    if auto_start_ids.is_empty() {
                        self.language
                            .pick("配置已导入并保存", "Configuration imported and saved")
                            .to_string()
                    } else if self.language == Language::Zh {
                        format!("配置已导入，正在自动连接 {} 个隧道", auto_start_ids.len())
                    } else {
                        format!(
                            "Configuration imported; connecting {} tunnel(s)",
                            auto_start_ids.len()
                        )
                    }
                    .into(),
                );
            }
            AppMessage::FileOperation(message) => self.notice = Some(message.into()),
            AppMessage::PrivateKeySelected(path) => {
                if let Some(form) = &self.form {
                    form.identity_file
                        .update(cx, |input, cx| input.set_value(path, cx));
                }
            }
            AppMessage::Tray(id) if id == "show" => {
                cx.activate(true);
                for handle in cx.windows() {
                    let _ = handle.update(cx, |_, window, _| window.activate_window());
                }
            }
            AppMessage::Tray(id) if id == "quit" => {
                let manager = self.manager.clone();
                let sender = self.messages.clone();
                self.runtime.spawn(async move {
                    manager.lock().await.stop_all().await;
                    let _ = sender.send(AppMessage::QuitReady).await;
                });
            }
            AppMessage::Tray(id) if id.starts_with("tunnel:") => {
                self.request_toggle(id.trim_start_matches("tunnel:").to_string(), cx);
            }
            AppMessage::Tray(_) => {}
            AppMessage::QuitReady => cx.quit(),
            AppMessage::HostTrusted(tunnel_id) => {
                self.auth_prompt = None;
                self.start_tunnel_with_passphrase(tunnel_id, None, cx);
            }
        }
    }

    fn refresh_tray(&mut self) {
        let menu = system::build_tray_menu(
            &self.config.tunnels,
            &self.statuses,
            self.language == Language::Zh,
        );
        match (&self._tray, menu) {
            (Some(tray), Ok(menu)) => tray.set_menu(Some(Box::new(menu))),
            (None, Ok(_)) => match system::build_tray(
                &self.config.tunnels,
                &self.statuses,
                self.language == Language::Zh,
            ) {
                Ok(tray) => self._tray = Some(tray),
                Err(error) => self.notice = Some(error.into()),
            },
            (_, Err(error)) => self.notice = Some(error.into()),
        }
    }

    fn set_filter(&mut self, filter: TunnelFilter, cx: &mut Context<Self>) {
        self.filter = filter;
        self.selected_tunnel = self
            .config
            .tunnels
            .iter()
            .find(|tunnel| match &self.filter {
                TunnelFilter::All => true,
                TunnelFilter::Active => self.is_active(&tunnel.id),
                TunnelFilter::Activity => false,
                TunnelFilter::Group(group_id) => tunnel.group_id.as_ref() == Some(group_id),
            })
            .map(|tunnel| tunnel.id.clone());
        self.notice = None;
        cx.notify();
    }

    fn open_create_sheet(&mut self, cx: &mut Context<Self>) {
        let mut form = TunnelForm::new(None, self.language, cx);
        form.ssh_hosts = parse_ssh_config(self.config.settings.ssh_config_path.as_deref());
        self.form = Some(form);
        self.notice = None;
        cx.notify();
    }

    fn close_create_sheet(&mut self, cx: &mut Context<Self>) {
        self.form = None;
        cx.notify();
    }

    fn edit_selected(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_tunnel.clone() else {
            return;
        };
        self.edit_tunnel(id, cx);
    }

    fn edit_tunnel(&mut self, id: String, cx: &mut Context<Self>) {
        self.selected_tunnel = Some(id.clone());
        let tunnel = self
            .config
            .tunnels
            .iter()
            .find(|tunnel| tunnel.id == id)
            .cloned();
        if let Some(mut tunnel) = tunnel {
            if let Some(reference) = tunnel.jump_host_id.as_ref().and_then(|reference_id| {
                self.config
                    .tunnels
                    .iter()
                    .find(|candidate| &candidate.id == reference_id)
            }) {
                tunnel.jump_host_id = None;
                tunnel.jump_host = Some(reference.ssh_host.clone());
                tunnel.jump_port = Some(reference.ssh_port);
                tunnel.jump_user = Some(reference.ssh_user.clone());
                tunnel.jump_identity_file = reference.ssh_identity_file.clone();
                tunnel.jump_password = reference.ssh_password.clone();
            }
            let mut form = TunnelForm::new(Some(&tunnel), self.language, cx);
            form.ssh_hosts = parse_ssh_config(self.config.settings.ssh_config_path.as_deref());
            self.form = Some(form);
            self.notice = None;
            cx.notify();
        }
    }

    fn open_advanced_selected(&mut self, cx: &mut Context<Self>) {
        self.edit_selected(cx);
        if let Some(form) = &mut self.form {
            form.advanced = true;
        }
        cx.notify();
    }

    fn delete_selected(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_tunnel.clone() else {
            return;
        };
        if self.is_active(&id) {
            self.notice = Some("请先停止隧道再删除".into());
            cx.notify();
            return;
        }
        let name = self
            .config
            .tunnels
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.name.clone())
            .unwrap_or_default();
        let mut next_config = self.config.clone();
        next_config.tunnels.retain(|t| t.id != id);
        match ConfigStore::new().save_config(&next_config) {
            Ok(()) => {
                self.config = next_config;
                self.statuses.remove(&id);
                self.selected_tunnel = None;
                self.notice = Some(format!("已删除“{name}”").into());
                self.refresh_tray();
            }
            Err(error) => self.notice = Some(format!("删除失败：{error}").into()),
        }
        cx.notify();
    }

    fn run_selected_diagnostics(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_tunnel.clone() else {
            return;
        };
        self.run_tunnel_diagnostics(id, cx);
    }

    fn run_tunnel_diagnostics(&mut self, id: String, cx: &mut Context<Self>) {
        self.selected_tunnel = Some(id.clone());
        let listener_is_current_tunnel = self.status(&id) == TunnelStatus::Running;
        let Some(tunnel) = self.config.tunnels.iter().find(|t| t.id == id).cloned() else {
            return;
        };
        let all = self.config.tunnels.clone();
        let sender = self.messages.clone();
        let language = if self.language == Language::Zh {
            DiagnosticLanguage::Chinese
        } else {
            DiagnosticLanguage::English
        };
        self.diagnostics = Some(Vec::new());
        self.notice = None;
        self.runtime.spawn(async move {
            let steps =
                run_diagnostics(&tunnel, &all, None, language, listener_is_current_tunnel).await;
            let _ = sender.send(AppMessage::Diagnostics(steps)).await;
        });
        cx.notify();
    }

    fn close_diagnostics(&mut self, cx: &mut Context<Self>) {
        self.diagnostics = None;
        cx.notify();
    }

    fn show_about(&mut self, cx: &mut Context<Self>) {
        self.about_open = true;
        cx.notify();
    }

    fn close_about(&mut self, cx: &mut Context<Self>) {
        self.about_open = false;
        cx.notify();
    }

    fn request_quit(&mut self) {
        let manager = self.manager.clone();
        let sender = self.messages.clone();
        self.runtime.spawn(async move {
            manager.lock().await.stop_all().await;
            let _ = sender.send(AppMessage::QuitReady).await;
        });
    }

    fn request_close(&mut self, cx: &mut Context<Self>) {
        if self.config.settings.close_to_tray && self._tray.is_some() {
            cx.hide();
        } else {
            self.request_quit();
        }
    }

    fn dismiss(&mut self, cx: &mut Context<Self>) {
        if self.about_open {
            self.about_open = false;
            cx.notify();
            return;
        }
        if let Some(form) = &mut self.form {
            if form.ssh_picker_target.is_some() {
                form.ssh_picker_target = None;
                cx.notify();
                return;
            }
            if form.group_menu_open {
                form.group_menu_open = false;
                cx.notify();
                return;
            }
        }
        if self.pending_import.is_some() {
            self.pending_import = None;
            cx.notify();
            return;
        }
        if self.save_confirmation.take().is_none()
            && self.auth_prompt.take().is_none()
            && self.diagnostics.take().is_none()
            && self.group_form.take().is_none()
            && self.settings_form.take().is_none()
        {
            self.form = None;
        }
        cx.notify();
    }

    fn clear_activity(&mut self, cx: &mut Context<Self>) {
        match EventLogger::new().clear_events() {
            Ok(()) => {
                self.events.clear();
                self.notice = Some(
                    self.language
                        .pick("活动记录已清空", "Activity cleared")
                        .into(),
                );
            }
            Err(error) => self.notice = Some(format!("清空失败：{error}").into()),
        }
        cx.notify();
    }

    fn clear_selected_logs(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = &self.selected_tunnel {
            self.logs.remove(id);
        }
        self.notice = Some(
            self.language
                .pick("当前隧道日志已清空", "Tunnel logs cleared")
                .into(),
        );
        cx.notify();
    }

    fn copy_selected_logs(&mut self, cx: &mut Context<Self>) {
        let content = self
            .selected_tunnel
            .as_ref()
            .and_then(|id| self.logs.get(id))
            .map(|logs| logs.join("\n"))
            .unwrap_or_default();
        cx.write_to_clipboard(ClipboardItem::new_string(content));
        self.notice = Some(self.language.pick("日志已复制", "Logs copied").into());
        cx.notify();
    }

    fn export_selected_logs(&mut self, cx: &mut Context<Self>) {
        let Some(id) = &self.selected_tunnel else {
            return;
        };
        let content = self
            .logs
            .get(id)
            .map(|logs| logs.join("\n"))
            .unwrap_or_default();
        let receiver = cx.prompt_for_new_path(Path::new("."), Some("tunnel-mate.log"));
        let sender = self.messages.clone();
        cx.spawn(async move |_, _| {
            if let Ok(Ok(Some(path))) = receiver.await {
                let message = match fs::write(&path, content) {
                    Ok(()) => format!("日志已导出到 {}", path.display()),
                    Err(error) => format!("日志导出失败：{error}"),
                };
                let _ = sender.send(AppMessage::FileOperation(message)).await;
            }
        })
        .detach();
    }

    fn open_settings(&mut self, cx: &mut Context<Self>) {
        let keep_alive = self.config.settings.keep_alive_interval.to_string();
        let connect_timeout = self.config.settings.connect_timeout.to_string();
        let ssh_path = self
            .config
            .settings
            .ssh_config_path
            .clone()
            .unwrap_or_default();
        self.settings_form = Some(SettingsForm {
            launch_on_startup: self.config.settings.launch_on_startup,
            start_minimized: self.config.settings.start_minimized,
            close_to_tray: self.config.settings.close_to_tray,
            keep_alive: cx.new(|cx| TextInput::new(cx, "30", keep_alive)),
            connect_timeout: cx.new(|cx| TextInput::new(cx, "15", connect_timeout)),
            ssh_config_path: cx.new(|cx| TextInput::new(cx, "~/.ssh/config", ssh_path)),
        });
        cx.notify();
    }

    fn cancel_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_form = None;
        cx.notify();
    }

    fn save_settings(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &self.settings_form else {
            return;
        };
        let keep_alive = match form.keep_alive.read(cx).value().parse::<u32>() {
            Ok(value) if value > 0 => value,
            _ => {
                self.notice = Some(
                    self.language
                        .pick(
                            "保活间隔必须是大于 0 的秒数",
                            "Keep-alive must be greater than 0 seconds",
                        )
                        .into(),
                );
                cx.notify();
                return;
            }
        };
        let connect_timeout = match form.connect_timeout.read(cx).value().parse::<u32>() {
            Ok(value) if value > 0 => value,
            _ => {
                self.notice = Some(
                    self.language
                        .pick(
                            "连接超时必须是大于 0 的秒数",
                            "Connection timeout must be greater than 0 seconds",
                        )
                        .into(),
                );
                cx.notify();
                return;
            }
        };
        let ssh_path = form.ssh_config_path.read(cx).value();
        let mut next_config = self.config.clone();
        next_config.settings.launch_on_startup = form.launch_on_startup;
        next_config.settings.start_minimized = form.start_minimized;
        next_config.settings.close_to_tray = form.close_to_tray;
        next_config.settings.keep_alive_interval = keep_alive;
        next_config.settings.connect_timeout = connect_timeout;
        next_config.settings.ssh_config_path = (!ssh_path.trim().is_empty()).then_some(ssh_path);
        if let Err(error) = system::sync_autostart(
            next_config.settings.launch_on_startup,
            next_config.settings.start_minimized,
        ) {
            self.notice = Some(error.into());
            cx.notify();
            return;
        }
        match ConfigStore::new().save_config(&next_config) {
            Ok(()) => {
                self.config = next_config;
                self.settings_form = None;
                self.notice = Some(self.language.pick("设置已保存", "Settings saved").into());
            }
            Err(error) => self.notice = Some(format!("设置保存失败：{error}").into()),
        }
        cx.notify();
    }

    fn toggle_setting(&mut self, setting: SettingToggle, cx: &mut Context<Self>) {
        let Some(form) = &mut self.settings_form else {
            return;
        };
        match setting {
            SettingToggle::Launch => form.launch_on_startup = !form.launch_on_startup,
            SettingToggle::Minimized => form.start_minimized = !form.start_minimized,
            SettingToggle::CloseToTray => form.close_to_tray = !form.close_to_tray,
        }
        cx.notify();
    }

    fn set_form_kind(&mut self, kind: ForwardKind, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.kind = kind;
            cx.notify();
        }
    }

    fn toggle_form_advanced(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.advanced = !form.advanced;
            cx.notify();
        }
    }

    fn toggle_form_reconnect(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.auto_reconnect = !form.auto_reconnect;
            cx.notify();
        }
    }

    fn toggle_form_start_with_app(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.start_with_app = !form.start_with_app;
            cx.notify();
        }
    }

    fn toggle_jump_host(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.jump_enabled = !form.jump_enabled;
            cx.notify();
        }
    }

    fn toggle_form_group_menu(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &mut self.form else { return };
        form.group_menu_open = !form.group_menu_open;
        cx.notify();
    }

    fn select_form_group(&mut self, group_id: Option<String>, cx: &mut Context<Self>) {
        let Some(form) = &mut self.form else { return };
        form.group_id = group_id;
        form.group_menu_open = false;
        cx.notify();
    }

    fn open_group_form(&mut self, cx: &mut Context<Self>) {
        let name_placeholder = self.language.pick("例如：生产环境", "e.g. Production");
        let description_placeholder = self.language.pick("说明（可选）", "Description (optional)");
        self.group_form = Some(GroupForm {
            editing_id: None,
            name: cx.new(|cx| TextInput::new(cx, name_placeholder, "")),
            description: cx.new(|cx| TextInput::new(cx, description_placeholder, "")),
        });
        cx.notify();
    }

    fn edit_current_group(&mut self, cx: &mut Context<Self>) {
        let TunnelFilter::Group(id) = &self.filter else {
            return;
        };
        let Some(group) = self.config.groups.iter().find(|group| &group.id == id) else {
            return;
        };
        let name_placeholder = self.language.pick("分组名称", "Group name");
        let description_placeholder = self.language.pick("说明（可选）", "Description (optional)");
        self.group_form = Some(GroupForm {
            editing_id: Some(group.id.clone()),
            name: cx.new(|cx| TextInput::new(cx, name_placeholder, group.name.clone())),
            description: cx.new(|cx| {
                TextInput::new(
                    cx,
                    description_placeholder,
                    group.description.clone().unwrap_or_default(),
                )
            }),
        });
        cx.notify();
    }

    fn close_group_form(&mut self, cx: &mut Context<Self>) {
        self.group_form = None;
        cx.notify();
    }

    fn save_group(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &self.group_form else { return };
        let name = form.name.read(cx).value();
        if name.trim().is_empty() {
            self.notice = Some(
                self.language
                    .pick("分组名称不能为空", "Group name cannot be empty")
                    .into(),
            );
            cx.notify();
            return;
        }
        let description = form.description.read(cx).value();
        let group = Group {
            id: form
                .editing_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name,
            description: (!description.trim().is_empty()).then_some(description),
        };
        let mut next_config = self.config.clone();
        if let Some(existing) = next_config
            .groups
            .iter_mut()
            .find(|existing| existing.id == group.id)
        {
            *existing = group;
        } else {
            next_config.groups.push(group);
        }
        match ConfigStore::new().save_config(&next_config) {
            Ok(()) => {
                self.config = next_config;
                self.group_form = None;
                self.notice = Some(self.language.pick("分组已保存", "Group saved").into());
            }
            Err(error) => self.notice = Some(format!("保存分组失败：{error}").into()),
        }
        cx.notify();
    }

    fn delete_current_group(&mut self, cx: &mut Context<Self>) {
        let TunnelFilter::Group(id) = self.filter.clone() else {
            return;
        };
        let mut next_config = self.config.clone();
        next_config.groups.retain(|group| group.id != id);
        for tunnel in &mut next_config.tunnels {
            if tunnel.group_id.as_deref() == Some(&id) {
                tunnel.group_id = None;
            }
        }
        match ConfigStore::new().save_config(&next_config) {
            Ok(()) => {
                self.config = next_config;
                self.filter = TunnelFilter::All;
                self.notice = Some(
                    self.language
                        .pick(
                            "分组已删除，隧道已移到未分组",
                            "Group deleted; its tunnels were moved to Ungrouped",
                        )
                        .into(),
                );
            }
            Err(error) => self.notice = Some(format!("删除分组失败：{error}").into()),
        }
        cx.notify();
    }

    fn toggle_start_after_save(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.start_after_save = !form.start_after_save;
            cx.notify();
        }
    }

    fn open_primary_ssh_hosts(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.ssh_picker_target = Some(SshPickerTarget::Primary);
            cx.notify();
        }
    }

    fn open_jump_ssh_hosts(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.ssh_picker_target = Some(SshPickerTarget::JumpHost);
            cx.notify();
        }
    }

    fn close_ssh_hosts(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.ssh_picker_target = None;
            cx.notify();
        }
    }

    fn dismiss_notice(&mut self, cx: &mut Context<Self>) {
        self.notice = None;
        cx.notify();
    }

    fn apply_ssh_host(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(form) = &mut self.form else { return };
        let Some(host) = form.ssh_hosts.get(index).cloned() else {
            return;
        };
        let target = form.ssh_picker_target.unwrap_or(SshPickerTarget::Primary);
        let resolved_host = host.host_name.unwrap_or(host.host);
        match target {
            SshPickerTarget::Primary => {
                form.ssh_host
                    .update(cx, |input, cx| input.set_value(resolved_host, cx));
                if let Some(port) = host.port {
                    form.ssh_port
                        .update(cx, |input, cx| input.set_value(port.to_string(), cx));
                }
                if let Some(user) = host.user {
                    form.ssh_user
                        .update(cx, |input, cx| input.set_value(user, cx));
                }
                if let Some(path) = host.identity_file {
                    form.identity_file
                        .update(cx, |input, cx| input.set_value(path, cx));
                }
            }
            SshPickerTarget::JumpHost => {
                form.jump_host_id = None;
                form.jump_host
                    .update(cx, |input, cx| input.set_value(resolved_host, cx));
                if let Some(port) = host.port {
                    form.jump_port
                        .update(cx, |input, cx| input.set_value(port.to_string(), cx));
                }
                if let Some(user) = host.user {
                    form.jump_user
                        .update(cx, |input, cx| input.set_value(user, cx));
                }
                if let Some(path) = host.identity_file {
                    form.jump_identity_file
                        .update(cx, |input, cx| input.set_value(path, cx));
                }
            }
        }
        form.ssh_picker_target = None;
        cx.notify();
    }

    fn select_private_key(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("选择私钥".into()),
        });
        let sender = self.messages.clone();
        cx.spawn(async move |_, _| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(path) = paths.into_iter().next() {
                    let _ = sender
                        .send(AppMessage::PrivateKeySelected(
                            path.to_string_lossy().into_owned(),
                        ))
                        .await;
                }
            }
        })
        .detach();
    }

    fn export_backup(&mut self, cx: &mut Context<Self>) {
        let content = match export_config_string(&self.config) {
            Ok(content) => content,
            Err(error) => {
                self.notice = Some(format!("导出失败：{error}").into());
                cx.notify();
                return;
            }
        };
        let directory = downloads_dir();
        let receiver = cx.prompt_for_new_path(&directory, Some("config.tunnelmate.json"));
        let sender = self.messages.clone();
        cx.spawn(async move |_, _| {
            if let Ok(Ok(Some(path))) = receiver.await {
                let message = match fs::write(&path, content) {
                    Ok(()) => format!("配置已导出到 {}", path.display()),
                    Err(error) => format!("导出失败：{error}"),
                };
                let _ = sender.send(AppMessage::FileOperation(message)).await;
            }
        })
        .detach();
    }

    fn import_backup(&mut self, cx: &mut Context<Self>) {
        self.open_import_picker(cx);
    }

    fn cancel_import_backup(&mut self, cx: &mut Context<Self>) {
        self.pending_import = None;
        cx.notify();
    }

    fn confirm_import_backup(&mut self, cx: &mut Context<Self>) {
        let Some(config) = self.pending_import.take() else {
            return;
        };
        self.settings_form = None;
        let manager = self.manager.clone();
        let sender = self.messages.clone();
        let language = self.language;
        self.runtime.spawn(async move {
            manager.lock().await.stop_all().await;
            let message = match ConfigStore::new().save_config(&config) {
                Ok(()) => AppMessage::ConfigImported(config),
                Err(error) => AppMessage::ImportFailed(format!(
                    "{}: {error}",
                    language.pick("导入保存失败", "Could not save imported configuration")
                )),
            };
            let _ = sender.send(message).await;
        });
        cx.notify();
    }

    fn commit_import(&self, config: AppConfig) {
        let sender = self.messages.clone();
        let language = self.language;
        self.runtime.spawn(async move {
            let message = match ConfigStore::new().save_config(&config) {
                Ok(()) => AppMessage::ConfigImported(config),
                Err(error) => AppMessage::ImportFailed(format!(
                    "{}: {error}",
                    language.pick("导入保存失败", "Could not save imported configuration")
                )),
            };
            let _ = sender.send(message).await;
        });
    }

    fn open_import_picker(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("导入配置".into()),
        });
        let sender = self.messages.clone();
        let language = self.language;
        cx.spawn(async move |_, _| match receiver.await {
            Ok(Ok(Some(paths))) => {
                if let Some(path) = paths.into_iter().next() {
                    let result = fs::read_to_string(&path)
                        .map_err(|error| format!("读取失败：{error}"))
                        .and_then(|content| import_config_string(&content));
                    match result {
                        Ok(config) => {
                            let _ = sender.send(AppMessage::ImportSelected(config)).await;
                        }
                        Err(error) => {
                            let _ = sender
                                .send(AppMessage::ImportFailed(format!("导入失败：{error}")))
                                .await;
                        }
                    }
                }
            }
            Ok(Ok(None)) => {}
            Ok(Err(error)) => {
                let _ = sender
                    .send(AppMessage::ImportFailed(format!(
                        "{}: {error}",
                        language.pick("无法打开文件选择器", "Could not open file picker")
                    )))
                    .await;
            }
            Err(error) => {
                let _ = sender
                    .send(AppMessage::ImportFailed(format!(
                        "{}: {error}",
                        language.pick("文件选择器意外关闭", "File picker closed unexpectedly")
                    )))
                    .await;
            }
        })
        .detach();
    }

    fn save_form(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &self.form else { return };
        let parse_port = |input: &Entity<TextInput>, label: &str| -> Result<u16, String> {
            input
                .read(cx)
                .value()
                .parse::<u16>()
                .map_err(|_| format!("{label}必须是 1–65535 的端口"))
        };
        let parse_u32 = |input: &Entity<TextInput>, label: &str| -> Result<u32, String> {
            input
                .read(cx)
                .value()
                .parse::<u32>()
                .map_err(|_| format!("{label}必须是非负整数"))
        };
        let result = (|| {
            let listen = Endpoint {
                host: form.listen_host.read(cx).value(),
                port: parse_port(&form.listen_port, "监听端口")?,
            };
            let forward = match form.kind {
                ForwardKind::Local => ForwardSpec::Local {
                    listen,
                    target: Endpoint {
                        host: form.target_host.read(cx).value(),
                        port: parse_port(&form.target_port, "目标端口")?,
                    },
                },
                ForwardKind::Remote => ForwardSpec::Remote {
                    listen,
                    target: Endpoint {
                        host: form.target_host.read(cx).value(),
                        port: parse_port(&form.target_port, "目标端口")?,
                    },
                },
                ForwardKind::Socks5 => ForwardSpec::Socks5 { listen },
            };
            let id = form
                .editing_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let identity = form.identity_file.read(cx).value();
            let description = form.description.read(cx).value();
            let ssh_password = form.ssh_password.read(cx).value();
            let jump_identity = form.jump_identity_file.read(cx).value();
            let jump_password = form.jump_password.read(cx).value();
            let manual_jump = form.jump_enabled && form.jump_host_id.is_none();
            let jump_port = if manual_jump {
                Some(parse_port(&form.jump_port, "跳板机端口")?)
            } else {
                None
            };
            let tunnel = Tunnel {
                id,
                name: form.name.read(cx).value(),
                description: (!description.trim().is_empty()).then_some(description),
                group_id: form.group_id.clone(),
                ssh_host: form.ssh_host.read(cx).value(),
                ssh_port: parse_port(&form.ssh_port, "SSH 端口")?,
                ssh_user: form.ssh_user.read(cx).value(),
                ssh_identity_file: (!identity.trim().is_empty()).then_some(identity),
                ssh_password: (!ssh_password.is_empty()).then_some(ssh_password),
                jump_host_enabled: form.jump_enabled,
                jump_host_id: form
                    .jump_enabled
                    .then(|| form.jump_host_id.clone())
                    .flatten(),
                jump_host: manual_jump.then(|| form.jump_host.read(cx).value()),
                jump_port: manual_jump.then_some(jump_port).flatten(),
                jump_user: manual_jump.then(|| form.jump_user.read(cx).value()),
                jump_identity_file: manual_jump
                    .then_some(jump_identity)
                    .filter(|value| !value.trim().is_empty()),
                jump_password: manual_jump
                    .then_some(jump_password)
                    .filter(|value| !value.is_empty()),
                forward,
                start_with_app: form.start_with_app,
                auto_reconnect: form.auto_reconnect,
                retry_count: parse_u32(&form.retry_count, "重试次数")?,
                retry_interval: parse_u32(&form.retry_interval, "重试间隔")?,
            };
            validate_tunnel(&tunnel)?;
            Ok::<_, String>((tunnel, form.start_after_save))
        })();
        match result {
            Err(error) => self.notice = Some(error.into()),
            Ok((tunnel, start_after_save)) => {
                let unchanged = self
                    .config
                    .tunnels
                    .iter()
                    .find(|existing| existing.id == tunnel.id)
                    .is_some_and(|existing| existing == &tunnel);
                if unchanged {
                    self.form = None;
                    self.notice = None;
                    cx.notify();
                    return;
                }
                if self.is_active(&tunnel.id) {
                    self.save_confirmation = Some(tunnel);
                    cx.notify();
                    return;
                }
                self.persist_tunnel(tunnel, start_after_save, cx);
                return;
            }
        }
        cx.notify();
    }

    fn cancel_save_confirmation(&mut self, cx: &mut Context<Self>) {
        self.save_confirmation = None;
        cx.notify();
    }

    fn confirm_save_and_restart(&mut self, cx: &mut Context<Self>) {
        let Some(tunnel) = self.save_confirmation.take() else {
            return;
        };
        self.persist_tunnel(tunnel, true, cx);
    }

    fn persist_tunnel(&mut self, tunnel: Tunnel, start_after_save: bool, cx: &mut Context<Self>) {
        let name = tunnel.name.clone();
        let id = tunnel.id.clone();
        let mut next_config = self.config.clone();
        if let Some(existing) = next_config
            .tunnels
            .iter_mut()
            .find(|existing| existing.id == tunnel.id)
        {
            *existing = tunnel;
        } else {
            next_config.tunnels.push(tunnel);
        }
        match ConfigStore::new().save_config(&next_config) {
            Ok(()) => {
                self.config = next_config;
                self.statuses
                    .entry(id.clone())
                    .or_insert(TunnelStatus::Stopped);
                self.selected_tunnel = Some(id.clone());
                self.form = None;
                self.notice = Some(format!("已保存“{name}”").into());
                self.refresh_tray();
                if start_after_save {
                    if self.is_active(&id) {
                        self.pending_starts.insert(id.clone());
                        self.notice = Some(
                            self.language
                                .pick(
                                    "正在断开旧连接，随后会使用新配置自动重连",
                                    "Disconnecting the old connection; the updated tunnel will reconnect automatically",
                                )
                                .into(),
                        );
                    }
                    self.request_toggle(id, cx);
                    return;
                }
            }
            Err(error) => self.notice = Some(format!("保存失败：{error}").into()),
        }
        cx.notify();
    }

    fn select_tunnel(&mut self, tunnel_id: String, cx: &mut Context<Self>) {
        self.selected_tunnel = Some(tunnel_id);
        self.notice = None;
        cx.notify();
    }

    fn request_toggle(&mut self, tunnel_id: String, cx: &mut Context<Self>) {
        let Some(tunnel) = self
            .config
            .tunnels
            .iter()
            .find(|tunnel| tunnel.id == tunnel_id)
            .cloned()
        else {
            return;
        };
        let running = self.is_active(&tunnel.id);
        let manager = self.manager.clone();
        let sender = self.messages.clone();
        let tunnel_name = tunnel.name.clone();

        if running {
            self.notice = Some(
                if self.language == Language::Zh {
                    format!("正在停止“{}”…", tunnel.name)
                } else {
                    format!("Stopping “{}”…", tunnel.name)
                }
                .into(),
            );
            self.runtime.spawn(async move {
                if let Err(message) = manager.lock().await.stop_tunnel(&tunnel.id).await {
                    let _ = sender
                        .send(AppMessage::OperationError {
                            tunnel_name,
                            message,
                        })
                        .await;
                }
            });
        } else {
            self.start_tunnel_with_passphrase(tunnel.id, None, cx);
            return;
        }
        cx.notify();
    }

    fn start_tunnel_with_passphrase(
        &mut self,
        tunnel_id: String,
        passphrase: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(tunnel) = self
            .config
            .tunnels
            .iter()
            .find(|tunnel| tunnel.id == tunnel_id)
            .cloned()
        else {
            return;
        };
        self.statuses
            .insert(tunnel.id.clone(), TunnelStatus::Connecting);
        self.notice = Some(
            if self.language == Language::Zh {
                format!("正在连接“{}”…", tunnel.name)
            } else {
                format!("Connecting to “{}”…", tunnel.name)
            }
            .into(),
        );
        let manager = self.manager.clone();
        let sender = self.messages.clone();
        let tunnel_name = tunnel.name.clone();
        let log_sender = sender.clone();
        let log_tunnel_id = tunnel.id.clone();
        let log_sink = LogSink::callback(move |message| {
            let _ = log_sender.try_send(AppMessage::Log {
                tunnel_id: log_tunnel_id.clone(),
                message,
            });
        });
        self.runtime.spawn(async move {
            if let Err(message) =
                TunnelManager::start_tunnel(manager, tunnel, passphrase, log_sink).await
            {
                let _ = sender
                    .send(AppMessage::OperationError {
                        tunnel_name,
                        message,
                    })
                    .await;
            }
        });
        cx.notify();
    }

    fn close_auth_prompt(&mut self, cx: &mut Context<Self>) {
        self.auth_prompt = None;
        cx.notify();
    }

    fn trust_prompted_host(&mut self, cx: &mut Context<Self>) {
        let Some(AuthPrompt::HostKey {
            tunnel_id,
            host,
            port,
            ..
        }) = &self.auth_prompt
        else {
            return;
        };
        let (tunnel_id, host, port) = (tunnel_id.clone(), host.clone(), *port);
        let sender = self.messages.clone();
        self.runtime.spawn(async move {
            match tunnel_core::ssh::engine::SshSession::trust_host_key(&host, port).await {
                Ok(()) => {
                    let _ = sender.send(AppMessage::HostTrusted(tunnel_id)).await;
                }
                Err(error) => {
                    let _ = sender
                        .send(AppMessage::FileOperation(format!(
                            "信任主机密钥失败：{error}"
                        )))
                        .await;
                }
            }
        });
        cx.notify();
    }

    fn submit_passphrase(&mut self, cx: &mut Context<Self>) {
        let Some(AuthPrompt::Passphrase { tunnel_id, input }) = &self.auth_prompt else {
            return;
        };
        let tunnel_id = tunnel_id.clone();
        let passphrase = input.read(cx).value();
        self.auth_prompt = None;
        self.start_tunnel_with_passphrase(tunnel_id, Some(passphrase), cx);
    }

    fn status(&self, tunnel_id: &str) -> TunnelStatus {
        self.statuses
            .get(tunnel_id)
            .cloned()
            .unwrap_or(TunnelStatus::Stopped)
    }

    fn is_active(&self, tunnel_id: &str) -> bool {
        matches!(
            self.status(tunnel_id),
            TunnelStatus::Running | TunnelStatus::Connecting | TunnelStatus::Reconnecting
        )
    }

    fn title(&self) -> SharedString {
        match &self.filter {
            TunnelFilter::All => self.language.pick("隧道", "Tunnels").into(),
            TunnelFilter::Active => self.language.pick("运行中", "Active").into(),
            TunnelFilter::Activity => self.language.pick("活动记录", "Activity").into(),
            TunnelFilter::Group(group_id) => self
                .config
                .groups
                .iter()
                .find(|group| &group.id == group_id)
                .map(|group| group.name.clone().into())
                .unwrap_or_else(|| self.language.pick("分组", "Group").into()),
        }
    }

    fn filtered_tunnels(&self, cx: &Context<Self>) -> Vec<&Tunnel> {
        let query = self.search.read(cx).value().trim().to_lowercase();
        self.config
            .tunnels
            .iter()
            .filter(|tunnel| {
                let in_filter = match &self.filter {
                    TunnelFilter::All => true,
                    TunnelFilter::Active => self.is_active(&tunnel.id),
                    TunnelFilter::Activity => false,
                    TunnelFilter::Group(group_id) => tunnel.group_id.as_ref() == Some(group_id),
                };
                in_filter
                    && (query.is_empty()
                        || tunnel.name.to_lowercase().contains(&query)
                        || tunnel.ssh_host.to_lowercase().contains(&query)
                        || Self::route(tunnel).to_lowercase().contains(&query))
            })
            .collect()
    }

    fn group_name(&self, tunnel: &Tunnel) -> SharedString {
        tunnel
            .group_id
            .as_ref()
            .and_then(|id| self.config.groups.iter().find(|group| &group.id == id))
            .map(|group| group.name.clone().into())
            .unwrap_or_else(|| self.language.pick("未分组", "Ungrouped").into())
    }

    fn route(tunnel: &Tunnel) -> String {
        match &tunnel.forward {
            ForwardSpec::Local { listen, target } => format!(
                "{}:{}  →  {}:{}",
                listen.host, listen.port, target.host, target.port
            ),
            ForwardSpec::Remote { listen, target } => format!(
                "{}:{}  ←  {}:{}",
                listen.host, listen.port, target.host, target.port
            ),
            ForwardSpec::Socks5 { listen } => {
                format!("{}:{}  →  SOCKS5", listen.host, listen.port)
            }
        }
    }

    fn nav_item(
        &self,
        label: impl Into<SharedString>,
        count: Option<usize>,
        filter: TunnelFilter,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.filter == filter;
        let icon = match &filter {
            TunnelFilter::All => "◇",
            TunnelFilter::Active => "▷",
            TunnelFilter::Activity => "↻",
            TunnelFilter::Group(_) => "",
        };
        let icon_color = if selected { TEXT } else { MUTED_DARK };
        let icon_element = if matches!(&filter, TunnelFilter::Group(_)) {
            div()
                .flex()
                .flex_col()
                .items_start()
                .justify_center()
                .w(px(15.0))
                .h(px(14.0))
                .child(
                    div()
                        .ml(px(1.0))
                        .w(px(7.0))
                        .h(px(3.0))
                        .rounded_t(px(2.0))
                        .bg(icon_color),
                )
                .child(
                    div()
                        .w(px(15.0))
                        .h(px(10.0))
                        .rounded(px(2.0))
                        .border_1()
                        .border_color(icon_color),
                )
                .into_any_element()
        } else {
            div()
                .w(px(15.0))
                .text_center()
                .text_size(px(12.0))
                .text_color(icon_color)
                .child(icon)
                .into_any_element()
        };
        let handler_filter = filter.clone();

        div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(42.0))
            .px(px(12.0))
            .rounded(px(7.0))
            .text_size(px(12.0))
            .text_color(if selected { TEXT } else { MUTED })
            .font_weight(if selected {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            })
            .bg(if selected {
                glass(0x123b7a, 0.62)
            } else {
                SIDEBAR_BG
            })
            .cursor_pointer()
            .hover(|style| style.bg(SURFACE_HOVER))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.set_filter(handler_filter.clone(), cx)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(icon_element)
                    .child(label.into()),
            )
            .when_some(count, |item, count| {
                item.child(
                    div()
                        .min_w(px(22.0))
                        .h(px(20.0))
                        .px(px(6.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(10.0))
                        .bg(if selected { color(0x282d32) } else { SURFACE })
                        .text_size(px(10.0))
                        .text_color(if selected { MUTED } else { MUTED_DARK })
                        .child(count.to_string()),
                )
            })
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let total = self.config.tunnels.len();
        let active = self
            .config
            .tunnels
            .iter()
            .filter(|tunnel| self.is_active(&tunnel.id))
            .count();
        let mut sidebar = div()
            .flex()
            .flex_col()
            .w(px(208.0))
            .h_full()
            .px(px(12.0))
            .pb(px(12.0))
            .border_r_1()
            .border_color(BORDER_SOFT)
            .bg(SIDEBAR_BG)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(11.0))
                    .h(px(64.0))
                    .px(px(8.0))
                    .mb(px(12.0))
                    .child(img(self.logo.clone()).size(px(32.0)).rounded(px(8.0)))
                    .child(
                        div()
                            .text_size(px(15.0))
                            .text_color(TEXT)
                            .font_weight(FontWeight::MEDIUM)
                            .child("Tunnel Mate"),
                    ),
            )
            .child(self.nav_item(
                self.language.pick("全部隧道", "All tunnels"),
                Some(total),
                TunnelFilter::All,
                cx,
            ))
            .child(self.nav_item(
                self.language.pick("正在运行", "Active"),
                Some(active),
                TunnelFilter::Active,
                cx,
            ))
            .child(self.nav_item(
                self.language.pick("活动记录", "Activity"),
                None,
                TunnelFilter::Activity,
                cx,
            ))
            .child(
                div()
                    .mt(px(22.0))
                    .mb(px(8.0))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(9.0))
                    .text_color(MUTED_DARK)
                    .child(self.language.pick("分组", "Groups"))
                    .child(
                        div()
                            .size(px(22.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(5.0))
                            .text_size(px(15.0))
                            .cursor_pointer()
                            .hover(|style| style.bg(SURFACE_HOVER))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.open_group_form(cx)),
                            )
                            .child("+"),
                    ),
            );

        for group in &self.config.groups {
            let count = self
                .config
                .tunnels
                .iter()
                .filter(|tunnel| tunnel.group_id.as_ref() == Some(&group.id))
                .count();
            sidebar = sidebar.child(self.nav_item(
                group.name.clone(),
                Some(count),
                TunnelFilter::Group(group.id.clone()),
                cx,
            ));
        }

        sidebar
            .child(div().flex_grow(1.0))
            .child(div().h(px(1.0)).mx(px(4.0)).mb(px(10.0)).bg(BORDER_SOFT))
            .child(
                div()
                    .h(px(42.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .rounded(px(7.0))
                    .text_size(px(12.0))
                    .text_color(MUTED)
                    .cursor_pointer()
                    .hover(|style| style.bg(SURFACE_HOVER))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.open_settings(cx)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .w(px(15.0))
                                    .text_center()
                                    .text_size(px(14.0))
                                    .text_color(MUTED_DARK)
                                    .child("⚙"),
                            )
                            .child(self.language.pick("设置", "Settings")),
                    ),
            )
    }

    fn render_tunnel_row(&self, tunnel: &Tunnel, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected_tunnel.as_deref() == Some(tunnel.id.as_str());
        let status = self.status(&tunnel.id);
        let running = self.is_active(&tunnel.id);
        let select_id = tunnel.id.clone();
        let toggle_id = tunnel.id.clone();
        let diagnose_id = tunnel.id.clone();
        let edit_id = tunnel.id.clone();
        let status_color = match status {
            TunnelStatus::Running => SUCCESS,
            TunnelStatus::Connecting | TunnelStatus::Reconnecting => WARNING,
            TunnelStatus::Failed => DANGER,
            TunnelStatus::Stopped => MUTED_DARK,
        };
        let status_label = match status {
            TunnelStatus::Connecting => self.language.pick("连接中", "Connecting"),
            TunnelStatus::Reconnecting => self.language.pick("重连中", "Reconnecting"),
            TunnelStatus::Running => self.language.pick("已连接", "Connected"),
            TunnelStatus::Failed => self.language.pick("连接失败", "Failed"),
            TunnelStatus::Stopped => self.language.pick("已停止", "Stopped"),
        };
        let kind_label = match &tunnel.forward {
            ForwardSpec::Local { .. } => "LOCAL",
            ForwardSpec::Remote { .. } => "REMOTE",
            ForwardSpec::Socks5 { .. } => "SOCKS5",
        };

        div()
            .flex()
            .items_center()
            .h(px(78.0))
            .mx(px(8.0))
            .mt(px(8.0))
            .px(px(18.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(if selected {
                glass(0x2f76f6, 0.72)
            } else {
                rgba(0x00000000)
            })
            .bg(if selected {
                glass(0x112c58, 0.58)
            } else {
                APP_BG
            })
            .cursor_pointer()
            .hover(|style| style.bg(SURFACE_HOVER))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.select_tunnel(select_id.clone(), cx)),
            )
            .child(
                div()
                    .size(px(9.0))
                    .mr(px(14.0))
                    .rounded(px(4.5))
                    .bg(status_color),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .flex_grow(1.0)
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(TEXT)
                            .child(tunnel.name.clone()),
                    )
                    .child(
                        div()
                            .mt(px(6.0))
                            .text_size(px(10.0))
                            .text_color(MUTED)
                            .child(Self::route(tunnel)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .mr(px(14.0))
                    .child(
                        div()
                            .h(px(30.0))
                            .px(px(11.0))
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(glass(0x2f76f6, 0.68))
                            .bg(glass(0x075bea, 0.10))
                            .text_size(px(10.0))
                            .text_color(color(0xaecbff))
                            .cursor_pointer()
                            .hover(|style| style.bg(glass(0x075bea, 0.24)).text_color(TEXT))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.run_tunnel_diagnostics(diagnose_id.clone(), cx)
                                }),
                            )
                            .child("◇")
                            .child(self.language.pick("诊断", "Diagnose")),
                    )
                    .child(
                        div()
                            .h(px(30.0))
                            .px(px(11.0))
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(BORDER)
                            .bg(glass(0xffffff, 0.025))
                            .text_size(px(10.0))
                            .text_color(MUTED)
                            .cursor_pointer()
                            .hover(|style| style.bg(SURFACE_HOVER).text_color(TEXT))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.edit_tunnel(edit_id.clone(), cx)
                                }),
                            )
                            .child("✎")
                            .child(self.language.pick("编辑", "Edit")),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .w(px(74.0))
                    .child(
                        div()
                            .px(px(7.0))
                            .py(px(3.0))
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(BORDER)
                            .text_size(px(8.0))
                            .text_color(MUTED)
                            .child(kind_label),
                    )
                    .child(
                        div()
                            .mt(px(6.0))
                            .text_size(px(9.0))
                            .text_color(status_color)
                            .child(status_label),
                    ),
            )
            .child(
                div()
                    .w(px(48.0))
                    .h(px(26.0))
                    .p(px(3.0))
                    .rounded(px(13.0))
                    .border_1()
                    .border_color(if running { PRIMARY } else { color(0x3a3f46) })
                    .bg(if running { PRIMARY } else { color(0x202329) })
                    .cursor_pointer()
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.request_toggle(toggle_id.clone(), cx)
                        }),
                    )
                    .child(
                        div()
                            .size(px(18.0))
                            .rounded(px(9.0))
                            .bg(if running {
                                PRIMARY_TEXT
                            } else {
                                color(0xd7d9dc)
                            })
                            .when(running, |dot| dot.ml(px(20.0))),
                    ),
            )
    }

    fn form_field(label: &'static str, input: Entity<TextInput>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(div().text_size(px(11.0)).text_color(MUTED).child(label))
            .child(input)
    }

    fn render_create_sheet(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.form.as_ref().expect("open form");
        let heading = if form.editing_id.is_some() {
            self.language.pick("编辑隧道", "Edit tunnel")
        } else {
            self.language.pick("新建隧道", "New tunnel")
        };
        let group_name = form
            .group_id
            .as_ref()
            .and_then(|id| self.config.groups.iter().find(|group| &group.id == id))
            .map(|group| group.name.as_str())
            .unwrap_or(self.language.pick("未分组", "Ungrouped"));
        let (
            forward_title,
            forward_description,
            forward_route,
            listen_address_label,
            listen_port_label,
            target_address_label,
            target_port_label,
        ) = match form.kind {
            ForwardKind::Local => (
                self.language.pick(
                    "从本机访问远端服务",
                    "Access a remote service locally",
                ),
                self.language.pick(
                    "应用连接本机监听端口，流量通过 SSH 隧道转发到远端目标。",
                    "Apps connect to a local port; traffic travels through SSH to the remote target.",
                ),
                self.language.pick(
                    "本机监听  →  SSH  →  远端目标",
                    "Local listen  →  SSH  →  Remote target",
                ),
                self.language.pick("本机监听地址", "Local listen address"),
                self.language.pick("本机端口", "Local port"),
                self.language.pick("远端目标地址", "Remote target address"),
                self.language.pick("远端目标端口", "Remote target port"),
            ),
            ForwardKind::Remote => (
                self.language.pick(
                    "让远端访问本机服务",
                    "Expose a local service remotely",
                ),
                self.language.pick(
                    "SSH 服务器监听远端端口，收到的流量通过隧道转发到本机目标。",
                    "The SSH server listens remotely and forwards incoming traffic to a local target.",
                ),
                self.language.pick(
                    "远端监听  →  SSH  →  本机目标",
                    "Remote listen  →  SSH  →  Local target",
                ),
                self.language.pick("远端监听地址", "Remote listen address"),
                self.language.pick("远端端口", "Remote port"),
                self.language.pick("本机目标地址", "Local target address"),
                self.language.pick("本机目标端口", "Local target port"),
            ),
            ForwardKind::Socks5 => (
                self.language
                    .pick("创建本机 SOCKS5 代理", "Create a local SOCKS5 proxy"),
                self.language.pick(
                    "应用使用本机 SOCKS5 端口，目标地址由应用决定并通过 SSH 访问。",
                    "Apps use a local SOCKS5 port and choose destinations that are reached through SSH.",
                ),
                self.language.pick(
                    "应用  →  本机 SOCKS5  →  SSH  →  目标地址",
                    "App  →  Local SOCKS5  →  SSH  →  Destination",
                ),
                self.language.pick("SOCKS5 监听地址", "SOCKS5 listen address"),
                self.language.pick("SOCKS5 端口", "SOCKS5 port"),
                self.language.pick("目标地址", "Target address"),
                self.language.pick("目标端口", "Target port"),
            ),
        };
        let kind_button = |label: &'static str, kind: ForwardKind| {
            let selected = form.kind == kind;
            div()
                .flex_grow(1.0)
                .h(px(34.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.0))
                .bg(if selected { PRIMARY } else { APP_BG })
                .text_color(if selected { TEXT } else { MUTED })
                .text_size(px(11.0))
                .cursor_pointer()
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| this.set_form_kind(kind, cx)),
                )
                .child(label)
        };
        let switch = |enabled: bool| {
            div()
                .w(px(42.0))
                .h(px(24.0))
                .rounded(px(12.0))
                .p(px(3.0))
                .bg(if enabled { PRIMARY } else { rgb(0x343840) })
                .child(
                    div()
                        .size(px(18.0))
                        .rounded(px(9.0))
                        .bg(TEXT)
                        .when(enabled, |dot| dot.ml(px(18.0))),
                )
        };
        let selected_group_id = form.group_id.clone();
        let mut group_options = div()
            .flex()
            .flex_col()
            .w(px(220.0))
            .max_h(px(240.0))
            .id("tunnel-group-options")
            .overflow_y_scroll()
            .on_scroll_wheel(stop_scroll_propagation)
            .p(px(4.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(BORDER)
            .bg(color(0x111722))
            .shadow_lg()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if let Some(form) = &mut this.form {
                    form.group_menu_open = false;
                    cx.notify();
                }
            }));
        let ungrouped_selected = selected_group_id.is_none();
        group_options = group_options.child(
            div()
                .id("group-option-ungrouped")
                .h(px(32.0))
                .px(px(9.0))
                .flex()
                .items_center()
                .justify_between()
                .rounded(px(6.0))
                .border_1()
                .border_color(if ungrouped_selected {
                    glass(0x075bea, 0.48)
                } else {
                    rgba(0x00000000)
                })
                .bg(if ungrouped_selected {
                    glass(0x075bea, 0.18)
                } else {
                    rgba(0x00000000)
                })
                .text_size(px(10.0))
                .text_color(if ungrouped_selected { TEXT } else { MUTED })
                .cursor_pointer()
                .hover(|style| {
                    style
                        .bg(if ungrouped_selected {
                            glass(0x075bea, 0.18)
                        } else {
                            glass(0x075bea, 0.10)
                        })
                        .border_color(glass(0x075bea, 0.36))
                        .text_color(TEXT)
                })
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        cx.stop_propagation();
                        this.select_form_group(None, cx);
                    }),
                )
                .child(self.language.pick("未分组", "Ungrouped"))
                .child(if ungrouped_selected { "✓" } else { "" }),
        );
        for (index, group) in self.config.groups.iter().enumerate() {
            let group_id = group.id.clone();
            let selected = selected_group_id.as_ref() == Some(&group.id);
            group_options = group_options.child(
                div()
                    .id(("group-option", index))
                    .h(px(32.0))
                    .px(px(9.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(if selected {
                        glass(0x075bea, 0.48)
                    } else {
                        rgba(0x00000000)
                    })
                    .bg(if selected {
                        glass(0x075bea, 0.18)
                    } else {
                        rgba(0x00000000)
                    })
                    .text_size(px(10.0))
                    .text_color(if selected { TEXT } else { MUTED })
                    .cursor_pointer()
                    .hover(|style| {
                        style
                            .bg(if selected {
                                glass(0x075bea, 0.18)
                            } else {
                                glass(0x075bea, 0.10)
                            })
                            .border_color(glass(0x075bea, 0.36))
                            .text_color(TEXT)
                    })
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.select_form_group(Some(group_id.clone()), cx)
                        }),
                    )
                    .child(group.name.clone())
                    .child(if selected { "✓" } else { "" }),
            );
        }
        let group_dropdown = div()
            .relative()
            .w(px(220.0))
            .h(px(34.0))
            .child(
                div()
                    .w_full()
                    .h_full()
                    .px(px(11.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(px(7.0))
                    .border_1()
                    .border_color(if form.group_menu_open {
                        PRIMARY
                    } else {
                        BORDER
                    })
                    .bg(APP_BG)
                    .text_size(px(10.0))
                    .text_color(TEXT)
                    .cursor_pointer()
                    .hover(|style| style.bg(SURFACE_HOVER))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            cx.stop_propagation();
                            this.toggle_form_group_menu(cx);
                        }),
                    )
                    .child(group_name.to_string())
                    .child(if form.group_menu_open { "▴" } else { "▾" }),
            )
            .when(form.group_menu_open, |container| {
                container.child(
                    deferred(
                        anchored()
                            .anchor(Anchor::TopRight)
                            .position(point(px(220.0), px(4.0)))
                            .position_mode(AnchoredPositionMode::Local)
                            .snap_to_window_with_margin(px(8.0))
                            .child(group_options),
                    )
                    .priority(2),
                )
            });
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0db8))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(620.0))
                    .max_h(px(610.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(SURFACE)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .h(px(58.0))
                            .px(px(18.0))
                            .border_b_1()
                            .border_color(BORDER)
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(TEXT)
                                    .child(heading),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size(px(30.0))
                                    .rounded(px(7.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(SURFACE_HOVER))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.close_create_sheet(cx)),
                                    )
                                    .child("×"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(14.0))
                            .p(px(18.0))
                            .id("tunnel-form-scroll")
                            .overflow_y_scroll()
                            .child(Self::form_field(
                                self.language.pick("名称", "Name"),
                                form.name.clone(),
                            ))
                            .child(Self::form_field(
                                self.language.pick("说明", "Description"),
                                form.description.clone(),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(MUTED)
                                            .child(self.language.pick("分组", "Group")),
                                    )
                                    .child(group_dropdown),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(10.0))
                                    .p(px(12.0))
                                    .rounded(px(10.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .bg(APP_BG)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(3.0))
                                                    .child(
                                                        div()
                                                            .text_size(px(12.0))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(TEXT)
                                                            .child(self.language.pick(
                                                                "SSH 连接",
                                                                "SSH connection",
                                                            )),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(10.0))
                                                            .text_color(MUTED)
                                                            .child(self.language.pick(
                                                                "手动填写，或从 SSH config 自动带入下方信息",
                                                                "Enter details manually or fill them from SSH config",
                                                            )),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .h(px(30.0))
                                                    .px(px(10.0))
                                                    .flex()
                                                    .items_center()
                                                    .rounded(px(7.0))
                                                    .border_1()
                                                    .border_color(glass(0x075bea, 0.45))
                                                    .bg(glass(0x075bea, 0.10))
                                                    .text_size(px(10.0))
                                                    .text_color(PRIMARY_TEXT)
                                                    .cursor_pointer()
                                                    .hover(|style| {
                                                        style.bg(glass(0x075bea, 0.16))
                                                    })
                                                    .on_mouse_up(
                                                        MouseButton::Left,
                                                        cx.listener(|this, _, _, cx| {
                                                            this.open_primary_ssh_hosts(cx)
                                                        }),
                                                    )
                                                    .child(self.language.pick(
                                                        "从 SSH config 选择…",
                                                        "Choose from SSH config…",
                                                    )),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap(px(10.0))
                                            .child(div().flex_grow(1.0).child(Self::form_field(
                                                self.language.pick("主机", "Host"),
                                                form.ssh_host.clone(),
                                            )))
                                            .child(div().w(px(100.0)).child(Self::form_field(
                                                self.language.pick("端口", "Port"),
                                                form.ssh_port.clone(),
                                            )))
                                            .child(div().w(px(150.0)).child(Self::form_field(
                                                self.language.pick("用户", "User"),
                                                form.ssh_user.clone(),
                                            ))),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(6.0))
                                    .p(px(4.0))
                                    .rounded(px(9.0))
                                    .bg(APP_BG)
                                    .child(kind_button(
                                        self.language.pick("本地转发", "Local"),
                                        ForwardKind::Local,
                                    ))
                                    .child(kind_button(
                                        self.language.pick("远程转发", "Remote"),
                                        ForwardKind::Remote,
                                    ))
                                    .child(kind_button("SOCKS5", ForwardKind::Socks5)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(14.0))
                                    .p(px(12.0))
                                    .rounded(px(9.0))
                                    .border_1()
                                    .border_color(glass(0x075bea, 0.32))
                                    .bg(glass(0x075bea, 0.08))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(4.0))
                                            .min_w_0()
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(TEXT)
                                                    .child(forward_title),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.0))
                                                    .line_height(relative(1.4))
                                                    .text_color(MUTED)
                                                    .child(forward_description),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex_none()
                                            .px(px(9.0))
                                            .py(px(6.0))
                                            .rounded(px(6.0))
                                            .bg(APP_BG)
                                            .text_size(px(9.0))
                                            .text_color(PRIMARY_TEXT)
                                            .child(forward_route),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(10.0))
                                    .child(div().flex_grow(1.0).child(Self::form_field(
                                        listen_address_label,
                                        form.listen_host.clone(),
                                    )))
                                    .child(div().w(px(130.0)).child(Self::form_field(
                                        listen_port_label,
                                        form.listen_port.clone(),
                                    ))),
                            )
                            .when(form.kind != ForwardKind::Socks5, |panel| {
                                panel.child(
                                    div()
                                        .flex()
                                        .gap(px(10.0))
                                        .child(div().flex_grow(1.0).child(Self::form_field(
                                            target_address_label,
                                            form.target_host.clone(),
                                        )))
                                        .child(div().w(px(130.0)).child(Self::form_field(
                                            target_port_label,
                                            form.target_port.clone(),
                                        ))),
                                )
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .p(px(12.0))
                                    .rounded(px(9.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .bg(APP_BG)
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(3.0))
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(TEXT)
                                                    .child(self.language.pick(
                                                        "应用启动时自动连接",
                                                        "Connect when app starts",
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.0))
                                                    .text_color(MUTED)
                                                    .child(self.language.pick(
                                                        "打开 Tunnel Mate 后自动建立此隧道",
                                                        "Connect this tunnel automatically when Tunnel Mate opens",
                                                    )),
                                            ),
                                    )
                                    .child(
                                        switch(form.start_with_app)
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    this.toggle_form_start_with_app(cx)
                                                }),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .py(px(9.0))
                                    .border_t_1()
                                    .border_color(BORDER)
                                    .text_size(px(12.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.toggle_form_advanced(cx)),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.0))
                                            .child(
                                                div()
                                                    .w(px(14.0))
                                                    .text_center()
                                                    .text_size(px(14.0))
                                                    .text_color(MUTED)
                                                    .child(if form.advanced { "▾" } else { "▸" }),
                                            )
                                            .child(
                                                self.language.pick("高级设置", "Advanced"),
                                            ),
                                    )
                                    .child(if form.advanced {
                                        self.language.pick("收起", "Collapse")
                                    } else {
                                        self.language.pick(
                                            "认证、重连与跳板机",
                                            "Auth, reconnect & jump host",
                                        )
                                    }),
                            )
                            .when(form.advanced, |panel| {
                                panel
                                    .child(
                                        div()
                                            .flex()
                                            .items_end()
                                            .gap(px(8.0))
                                            .child(div().flex_grow(1.0).child(Self::form_field(
                                                self.language.pick("私钥文件", "Private key"),
                                                form.identity_file.clone(),
                                            )))
                                            .child(
                                                div()
                                                    .h(px(38.0))
                                                    .px(px(11.0))
                                                    .flex()
                                                    .items_center()
                                                    .rounded(px(8.0))
                                                    .border_1()
                                                    .border_color(BORDER)
                                                    .text_size(px(10.0))
                                                    .text_color(MUTED)
                                                    .cursor_pointer()
                                                    .on_mouse_up(
                                                        MouseButton::Left,
                                                        cx.listener(|this, _, _, cx| {
                                                            this.select_private_key(cx)
                                                        }),
                                                    )
                                                    .child(self.language.pick("选择…", "Choose…")),
                                            ),
                                    )
                                    .child(Self::form_field(
                                        self.language.pick("SSH 密码", "SSH password"),
                                        form.ssh_password.clone(),
                                    ))
                                    .child(
                                        div()
                                            .flex()
                                            .gap(px(10.0))
                                            .child(div().flex_grow(1.0).child(Self::form_field(
                                                self.language.pick("重试次数", "Retry count"),
                                                form.retry_count.clone(),
                                            )))
                                            .child(div().flex_grow(1.0).child(Self::form_field(
                                                self.language.pick(
                                                    "重试间隔（秒）",
                                                    "Retry interval (seconds)",
                                                ),
                                                form.retry_interval.clone(),
                                            ))),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .text_size(px(12.0))
                                            .text_color(TEXT)
                                            .child(
                                                self.language.pick(
                                                    "断线后自动重连",
                                                    "Reconnect automatically",
                                                ),
                                            )
                                            .child(
                                                switch(form.auto_reconnect)
                                                    .cursor_pointer()
                                                    .on_mouse_up(
                                                        MouseButton::Left,
                                                        cx.listener(|this, _, _, cx| {
                                                            this.toggle_form_reconnect(cx)
                                                        }),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .pt(px(10.0))
                                            .border_t_1()
                                            .border_color(BORDER)
                                            .text_size(px(12.0))
                                            .text_color(TEXT)
                                            .child(
                                                self.language.pick("使用跳板机", "Use jump host"),
                                            )
                                            .child(
                                                switch(form.jump_enabled)
                                                    .cursor_pointer()
                                                    .on_mouse_up(
                                                        MouseButton::Left,
                                                        cx.listener(|this, _, _, cx| {
                                                            this.toggle_jump_host(cx)
                                                        }),
                                                    ),
                                            ),
                                    )
                                    .when(form.jump_enabled, |advanced| {
                                        advanced.child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .gap(px(10.0))
                                                .p(px(12.0))
                                                .rounded(px(10.0))
                                                .border_1()
                                                .border_color(BORDER)
                                                .bg(APP_BG)
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .justify_between()
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .flex_col()
                                                                .gap(px(3.0))
                                                                .child(
                                                                    div()
                                                                        .text_size(px(12.0))
                                                                        .font_weight(
                                                                            FontWeight::MEDIUM,
                                                                        )
                                                                        .text_color(TEXT)
                                                                        .child(self.language.pick(
                                                                            "跳板机连接",
                                                                            "Jump host connection",
                                                                        )),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_size(px(10.0))
                                                                        .text_color(MUTED)
                                                                        .child(self.language.pick(
                                                                            "手动填写，或从 SSH config 自动带入下方信息",
                                                                            "Enter details manually or fill them from SSH config",
                                                                        )),
                                                                ),
                                                        )
                                                        .child(
                                                            div()
                                                                .h(px(30.0))
                                                                .px(px(10.0))
                                                                .flex()
                                                                .items_center()
                                                                .rounded(px(7.0))
                                                                .border_1()
                                                                .border_color(glass(
                                                                    0x075bea, 0.45,
                                                                ))
                                                                .bg(glass(0x075bea, 0.10))
                                                                .text_size(px(10.0))
                                                                .text_color(PRIMARY_TEXT)
                                                                .cursor_pointer()
                                                                .hover(|style| {
                                                                    style.bg(glass(
                                                                        0x075bea, 0.16,
                                                                    ))
                                                                })
                                                                .on_mouse_up(
                                                                    MouseButton::Left,
                                                                    cx.listener(
                                                                        |this, _, _, cx| {
                                                                            this.open_jump_ssh_hosts(
                                                                                cx,
                                                                            )
                                                                        },
                                                                    ),
                                                                )
                                                                .child(self.language.pick(
                                                                    "从 SSH config 选择…",
                                                                    "Choose from SSH config…",
                                                                )),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .flex()
                                                        .gap(px(10.0))
                                                        .child(div().flex_grow(1.0).child(
                                                            Self::form_field(
                                                                self.language.pick("主机", "Host"),
                                                                form.jump_host.clone(),
                                                            ),
                                                        ))
                                                        .child(div().w(px(100.0)).child(
                                                            Self::form_field(
                                                                self.language.pick("端口", "Port"),
                                                                form.jump_port.clone(),
                                                            ),
                                                        ))
                                                        .child(div().w(px(140.0)).child(
                                                            Self::form_field(
                                                                self.language.pick("用户", "User"),
                                                                form.jump_user.clone(),
                                                            ),
                                                        )),
                                                )
                                                .child(Self::form_field(
                                                    self.language.pick("私钥文件", "Private key"),
                                                    form.jump_identity_file.clone(),
                                                ))
                                                .child(Self::form_field(
                                                    self.language.pick("密码", "Password"),
                                                    form.jump_password.clone(),
                                                )),
                                        )
                                    })
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(10.0))
                            .h(px(64.0))
                            .px(px(18.0))
                            .items_center()
                            .border_t_1()
                            .border_color(BORDER)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .h(px(36.0))
                                    .px(px(14.0))
                                    .rounded(px(8.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(12.0))
                                    .text_color(TEXT)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.close_create_sheet(cx)),
                                    )
                                    .child(self.language.pick("取消", "Cancel")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .mr(px(4.0))
                                    .h(px(36.0))
                                    .px(px(4.0))
                                    .text_size(px(12.0))
                                    .text_color(TEXT)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.toggle_start_after_save(cx)
                                        }),
                                    )
                                    .child(
                                        div()
                                            .size(px(20.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(5.0))
                                            .border_1()
                                            .border_color(if form.start_after_save {
                                                PRIMARY
                                            } else {
                                                color(0x4b5260)
                                            })
                                            .bg(if form.start_after_save {
                                                PRIMARY
                                            } else {
                                                APP_BG
                                            })
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(PRIMARY_TEXT)
                                            .child(if form.start_after_save { "✓" } else { "" }),
                                    )
                                    .child(self.language.pick("保存后启动", "Start after save")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .h(px(36.0))
                                    .px(px(14.0))
                                    .rounded(px(8.0))
                                    .bg(PRIMARY)
                                    .text_size(px(12.0))
                                    .text_color(PRIMARY_TEXT)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.save_form(cx)),
                                    )
                                    .child(self.language.pick("保存", "Save")),
                            ),
                    ),
            )
    }

    #[allow(dead_code)]
    fn render_context_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let shell = div()
            .flex()
            .flex_col()
            .w(px(356.0))
            .my(px(8.0))
            .mr(px(8.0))
            .rounded(px(9.0))
            .border_1()
            .border_color(BORDER)
            .bg(color(0x171a1f))
            .overflow_hidden();
        let Some(tunnel) = self
            .selected_tunnel
            .as_ref()
            .and_then(|id| self.config.tunnels.iter().find(|tunnel| &tunnel.id == id))
        else {
            return shell.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .h_full()
                    .px(px(32.0))
                    .text_center()
                    .child(
                        div()
                            .size(px(38.0))
                            .rounded(px(19.0))
                            .border_1()
                            .border_color(BORDER)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(MUTED_DARK)
                            .child("→"),
                    )
                    .child(
                        div()
                            .mt(px(14.0))
                            .text_size(px(11.0))
                            .text_color(MUTED)
                            .child("选择一个隧道查看连接详情"),
                    ),
            );
        };
        let status = self.status(&tunnel.id);
        let running = self.is_active(&tunnel.id);
        let (status_label, status_color) = match status {
            TunnelStatus::Running => ("已连接", SUCCESS),
            TunnelStatus::Connecting => ("连接中", WARNING),
            TunnelStatus::Reconnecting => ("重连中", WARNING),
            TunnelStatus::Failed => ("连接失败", DANGER),
            TunnelStatus::Stopped => ("已停止", MUTED_DARK),
        };
        let toggle_id = tunnel.id.clone();
        let mut recent_activity = div().flex().flex_col();
        let events = self
            .events
            .iter()
            .rev()
            .filter(|event| event.tunnel_id.as_deref() == Some(tunnel.id.as_str()))
            .take(4)
            .collect::<Vec<_>>();
        if events.is_empty() {
            recent_activity = recent_activity.child(
                div()
                    .py(px(14.0))
                    .text_size(px(10.0))
                    .text_color(MUTED_DARK)
                    .child("暂无活动记录"),
            );
        } else {
            for event in events {
                recent_activity = recent_activity.child(
                    div()
                        .flex()
                        .items_center()
                        .h(px(34.0))
                        .child(div().size(px(6.0)).rounded(px(3.0)).mr(px(9.0)).bg(
                            match event.event_type {
                                tunnel_core::event_logger::EventType::Failed => DANGER,
                                tunnel_core::event_logger::EventType::Started
                                | tunnel_core::event_logger::EventType::Reconnected => SUCCESS,
                                _ => MUTED_DARK,
                            },
                        ))
                        .child(
                            div()
                                .flex_grow(1.0)
                                .min_w_0()
                                .text_size(px(9.0))
                                .text_color(MUTED)
                                .child(event.message.clone()),
                        )
                        .child(
                            div()
                                .ml(px(8.0))
                                .text_size(px(9.0))
                                .text_color(MUTED_DARK)
                                .child(event.timestamp.format("%H:%M").to_string()),
                        ),
                );
            }
        }

        let mut panel = shell
            .child(
                div()
                    .px(px(22.0))
                    .pt(px(24.0))
                    .pb(px(20.0))
                    .border_b_1()
                    .border_color(BORDER_SOFT)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .size(px(9.0))
                                    .rounded(px(4.5))
                                    .mr(px(11.0))
                                    .bg(status_color),
                            )
                            .child(
                                div()
                                    .flex_grow(1.0)
                                    .text_size(px(15.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(TEXT)
                                    .child(tunnel.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(MUTED_DARK)
                                    .child("•••"),
                            ),
                    )
                    .when_some(tunnel.description.clone(), |header, description| {
                        header.child(
                            div()
                                .mt(px(8.0))
                                .text_size(px(10.0))
                                .text_color(MUTED)
                                .child(description),
                        )
                    }),
            )
            .child(
                div()
                    .px(px(22.0))
                    .py(px(20.0))
                    .border_b_1()
                    .border_color(BORDER_SOFT)
                    .child(
                        div()
                            .mb(px(14.0))
                            .text_size(px(10.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(TEXT)
                            .child("连接摘要"),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(MUTED)
                            .child(Self::route(tunnel)),
                    )
                    .child(
                        div()
                            .mt(px(10.0))
                            .text_size(px(10.0))
                            .text_color(MUTED)
                            .child(format!(
                                "{}@{}:{}",
                                tunnel.ssh_user, tunnel.ssh_host, tunnel.ssh_port
                            )),
                    )
                    .child(
                        div()
                            .mt(px(10.0))
                            .text_size(px(9.0))
                            .text_color(MUTED_DARK)
                            .child(self.group_name(tunnel)),
                    )
                    .child(
                        div()
                            .mt(px(14.0))
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .size(px(7.0))
                                    .rounded(px(3.5))
                                    .mr(px(8.0))
                                    .bg(status_color),
                            )
                            .child(
                                div()
                                    .text_size(px(9.0))
                                    .text_color(status_color)
                                    .child(status_label),
                            ),
                    )
                    .child(
                        div()
                            .mt(px(18.0))
                            .h(px(42.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(7.0))
                            .border_1()
                            .border_color(if running { color(0x48695e) } else { PRIMARY })
                            .bg(if running { color(0x1a2b25) } else { PRIMARY })
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if running { PRIMARY } else { PRIMARY_TEXT })
                            .cursor_pointer()
                            .hover(|style| {
                                style.bg(if running {
                                    color(0x20352e)
                                } else {
                                    PRIMARY_HOVER
                                })
                            })
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.request_toggle(toggle_id.clone(), cx)
                                }),
                            )
                            .child(if running {
                                "停止隧道"
                            } else {
                                "启动隧道"
                            }),
                    )
                    .child(
                        div()
                            .mt(px(10.0))
                            .flex()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex_grow(1.0)
                                    .h(px(34.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(6.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(10.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(SURFACE_HOVER))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.run_selected_diagnostics(cx)
                                        }),
                                    )
                                    .child("连接诊断"),
                            )
                            .child(
                                div()
                                    .flex_grow(1.0)
                                    .h(px(34.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(6.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(10.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(SURFACE_HOVER))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.edit_selected(cx)),
                                    )
                                    .child("编辑"),
                            ),
                    ),
            )
            .child(
                div()
                    .px(px(22.0))
                    .py(px(18.0))
                    .border_b_1()
                    .border_color(BORDER_SOFT)
                    .child(
                        div()
                            .mb(px(8.0))
                            .text_size(px(10.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(TEXT)
                            .child("最近活动"),
                    )
                    .child(recent_activity),
            );

        if let Some(logs) = self.logs.get(&tunnel.id).filter(|logs| !logs.is_empty()) {
            let mut log_panel = div()
                .px(px(22.0))
                .py(px(18.0))
                .border_b_1()
                .border_color(BORDER_SOFT)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .mb(px(8.0))
                        .child(
                            div()
                                .text_size(px(10.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(TEXT)
                                .child("实时日志"),
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(10.0))
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(MUTED)
                                        .cursor_pointer()
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.copy_selected_logs(cx)
                                            }),
                                        )
                                        .child("复制"),
                                )
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(MUTED)
                                        .cursor_pointer()
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.export_selected_logs(cx)
                                            }),
                                        )
                                        .child("导出"),
                                )
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(DANGER)
                                        .cursor_pointer()
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.clear_selected_logs(cx)
                                            }),
                                        )
                                        .child("清空"),
                                ),
                        ),
                );
            for line in logs.iter().rev().take(3).rev() {
                log_panel = log_panel.child(
                    div()
                        .mt(px(5.0))
                        .text_size(px(9.0))
                        .text_color(MUTED_DARK)
                        .child(line.clone()),
                );
            }
            panel = panel.child(log_panel);
        }

        panel.child(div().flex_grow(1.0)).child(
            div()
                .h(px(52.0))
                .px(px(22.0))
                .flex()
                .items_center()
                .border_t_1()
                .border_color(BORDER_SOFT)
                .child(
                    div()
                        .flex_grow(1.0)
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(10.0))
                        .text_color(MUTED)
                        .cursor_pointer()
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.open_advanced_selected(cx)),
                        )
                        .child("高级设置")
                        .child("›"),
                )
                .child(
                    div()
                        .ml(px(16.0))
                        .text_size(px(9.0))
                        .text_color(DANGER)
                        .cursor_pointer()
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.delete_selected(cx)),
                        )
                        .child("删除"),
                ),
        )
    }

    fn render_workspace(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tunnels = self.filtered_tunnels(cx);
        let title = self.title();
        let mut list = div()
            .flex()
            .flex_col()
            .flex_grow(1.0)
            .mx(px(10.0))
            .mb(px(10.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(BORDER)
            .bg(APP_BG)
            .overflow_hidden();
        if self.filter == TunnelFilter::Activity && !self.events.is_empty() {
            for event in self.events.iter().rev() {
                list = list.child(
                    div()
                        .flex()
                        .items_center()
                        .min_h(px(70.0))
                        .px(px(22.0))
                        .border_b_1()
                        .border_color(BORDER_SOFT)
                        .child(div().size(px(7.0)).rounded(px(3.5)).mr(px(12.0)).bg(
                            match event.event_type {
                                tunnel_core::event_logger::EventType::Failed => DANGER,
                                tunnel_core::event_logger::EventType::Started
                                | tunnel_core::event_logger::EventType::Reconnected => SUCCESS,
                                _ => MUTED_DARK,
                            },
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_grow(1.0)
                                .child(
                                    div().text_size(px(11.0)).text_color(TEXT).child(
                                        event
                                            .tunnel_name
                                            .clone()
                                            .unwrap_or_else(|| "Tunnel Mate".into()),
                                    ),
                                )
                                .child(
                                    div()
                                        .mt(px(5.0))
                                        .text_size(px(10.0))
                                        .text_color(MUTED)
                                        .child(event.message.clone()),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(9.0))
                                .text_color(MUTED_DARK)
                                .child(event.timestamp.format("%m-%d %H:%M").to_string()),
                        ),
                );
            }
        } else if tunnels.is_empty() {
            list = list.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .h(px(260.0))
                    .child(div().text_size(px(11.0)).text_color(MUTED).child(
                        if self.filter == TunnelFilter::Activity {
                            self.language.pick("暂无活动记录", "No activity yet")
                        } else if self.config.tunnels.is_empty() {
                            self.language.pick("还没有隧道", "No tunnels yet")
                        } else {
                            self.language
                                .pick("当前筛选下没有隧道", "No matching tunnels")
                        },
                    ))
                    .when(self.config.tunnels.is_empty(), |empty| {
                        empty.child(
                            div()
                                .mt(px(7.0))
                                .text_size(px(9.0))
                                .text_color(MUTED_DARK)
                                .child(self.language.pick(
                                    "创建第一个连接，常用设置只需一分钟",
                                    "Create your first connection in about a minute",
                                )),
                        )
                    }),
            );
        } else {
            for tunnel in tunnels {
                list = list.child(self.render_tunnel_row(tunnel, cx));
            }
        }

        let mut center = div()
            .flex()
            .flex_col()
            .flex_grow(1.0)
            .min_w_0()
            .h_full()
            .bg(APP_BG)
            .child(
                div()
                    .h(px(76.0))
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_grow(1.0)
                            .text_size(px(18.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(TEXT)
                            .child(title),
                    )
                    .when(
                        self.filter == TunnelFilter::Activity && !self.events.is_empty(),
                        |header| {
                            header.child(
                                div()
                                    .h(px(34.0))
                                    .px(px(11.0))
                                    .mr(px(8.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(6.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(10.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(SURFACE_HOVER))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.clear_activity(cx)),
                                    )
                                    .child(self.language.pick("清空记录", "Clear")),
                            )
                        },
                    )
                    .when(matches!(self.filter, TunnelFilter::Group(_)), |header| {
                        header
                            .child(
                                div()
                                    .h(px(34.0))
                                    .px(px(11.0))
                                    .mr(px(8.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(6.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(10.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.edit_current_group(cx)),
                                    )
                                    .child(self.language.pick("编辑分组", "Edit group")),
                            )
                            .child(
                                div()
                                    .h(px(34.0))
                                    .px(px(10.0))
                                    .mr(px(8.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(6.0))
                                    .text_size(px(10.0))
                                    .text_color(DANGER)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.delete_current_group(cx)),
                                    )
                                    .child(self.language.pick("删除", "Delete")),
                            )
                    })
                    .when(self.filter != TunnelFilter::Activity, |header| {
                        header
                            .child(div().w(px(246.0)).mr(px(10.0)).child(self.search.clone()))
                            .child(
                                div()
                                    .h(px(40.0))
                                    .px(px(14.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(7.0))
                                    .border_1()
                                    .border_color(PRIMARY)
                                    .bg(PRIMARY)
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(PRIMARY_TEXT)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(PRIMARY_HOVER))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.open_create_sheet(cx)),
                                    )
                                    .child(self.language.pick("新建隧道", "New tunnel")),
                            )
                    }),
            );

        if let Some(error) = &self.load_error {
            center = center.child(
                div()
                    .mx(px(18.0))
                    .mt(px(14.0))
                    .px(px(12.0))
                    .py(px(9.0))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(color(0x65383d))
                    .text_size(px(10.0))
                    .text_color(color(0xee9ba2))
                    .child(error.clone()),
            );
        }
        center = center.child(list);
        div().flex().flex_grow(1.0).min_w_0().h_full().child(center)
    }

    fn render_notice(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .right(px(18.0))
            .bottom(px(18.0))
            .max_w(px(420.0))
            .px(px(13.0))
            .py(px(11.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .rounded(px(9.0))
            .border_1()
            .border_color(BORDER)
            .bg(color(0x171d27))
            .shadow_lg()
            .text_size(px(11.0))
            .text_color(TEXT)
            .child(div().size(px(7.0)).rounded(px(4.0)).bg(PRIMARY))
            .child(
                div()
                    .flex_grow(1.0)
                    .min_w_0()
                    .whitespace_normal()
                    .child(self.notice.clone().unwrap_or_default()),
            )
            .child(
                div()
                    .size(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.0))
                    .text_color(MUTED)
                    .cursor_pointer()
                    .hover(|style| style.bg(SURFACE_HOVER).text_color(TEXT))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.dismiss_notice(cx)),
                    )
                    .child("×"),
            )
    }

    fn render_ssh_host_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.form.as_ref().expect("open tunnel form");
        let picker_target = form.ssh_picker_target.unwrap_or(SshPickerTarget::Primary);
        let (title, description) = match picker_target {
            SshPickerTarget::Primary => (
                self.language.pick("选择 SSH 主机", "Choose an SSH host"),
                self.language.pick(
                    "选择后会填入 SSH 连接的主机、端口、用户和私钥",
                    "Fills the SSH connection host, port, user, and identity",
                ),
            ),
            SshPickerTarget::JumpHost => (
                self.language.pick("选择跳板机", "Choose a jump host"),
                self.language.pick(
                    "选择后会填入跳板机的主机、端口、用户和私钥",
                    "Fills the jump host, port, user, and identity",
                ),
            ),
        };
        let current_host = match picker_target {
            SshPickerTarget::Primary => form.ssh_host.read(cx).value(),
            SshPickerTarget::JumpHost => form.jump_host.read(cx).value(),
        };
        let matched_index = form
            .ssh_hosts
            .iter()
            .position(|host| ssh_host_matches(host, &current_host));
        let mut host_indices = (0..form.ssh_hosts.len()).collect::<Vec<_>>();
        if let Some(index) = matched_index {
            host_indices.remove(index);
            host_indices.insert(0, index);
        }
        let mut hosts = div()
            .id("ssh-host-picker-scroll")
            .flex()
            .flex_col()
            .gap(px(7.0))
            .max_h(px(410.0))
            .overflow_y_scroll()
            .p(px(14.0));

        if form.ssh_hosts.is_empty() {
            hosts = hosts.child(
                div()
                    .h(px(140.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(7.0))
                    .rounded(px(9.0))
                    .border_1()
                    .border_color(BORDER)
                    .text_color(MUTED)
                    .child(div().text_size(px(20.0)).child("⌁"))
                    .child(
                        div().text_size(px(11.0)).child(
                            self.language
                                .pick("SSH config 中没有可用主机", "No hosts found in SSH config"),
                        ),
                    ),
            );
        } else {
            for index in host_indices {
                let host = &form.ssh_hosts[index];
                let selected = matched_index == Some(index);
                let endpoint = format!(
                    "{}@{}:{}",
                    host.user
                        .as_deref()
                        .unwrap_or(self.language.pick("默认用户", "default user")),
                    host.host_name.as_deref().unwrap_or(&host.host),
                    host.port.unwrap_or(22)
                );
                let identity = host.identity_file.as_deref().map(|path| {
                    Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(path)
                        .to_string()
                });
                hosts = hosts.child(
                    div()
                        .id(("ssh-host-option", index))
                        .px(px(12.0))
                        .py(px(10.0))
                        .flex()
                        .items_center()
                        .gap(px(11.0))
                        .rounded(px(9.0))
                        .border_1()
                        .border_color(if selected {
                            glass(0x075bea, 0.52)
                        } else {
                            BORDER
                        })
                        .bg(if selected {
                            glass(0x075bea, 0.18)
                        } else {
                            APP_BG
                        })
                        .cursor_pointer()
                        .hover(move |style| {
                            style
                                .bg(if selected {
                                    glass(0x075bea, 0.18)
                                } else {
                                    glass(0x075bea, 0.10)
                                })
                                .border_color(glass(0x075bea, 0.42))
                        })
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| this.apply_ssh_host(index, cx)),
                        )
                        .child(
                            div()
                                .size(px(32.0))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(8.0))
                                .bg(glass(0x075bea, if selected { 0.28 } else { 0.16 }))
                                .text_color(PRIMARY)
                                .child("⌁"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_grow(1.0)
                                .min_w_0()
                                .gap(px(3.0))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(TEXT)
                                        .child(host.host.clone()),
                                )
                                .child(div().text_size(px(10.0)).text_color(MUTED).child(endpoint)),
                        )
                        .when_some(identity, |row, identity| {
                            row.child(
                                div()
                                    .px(px(7.0))
                                    .py(px(4.0))
                                    .rounded(px(5.0))
                                    .bg(SURFACE)
                                    .text_size(px(9.0))
                                    .text_color(MUTED)
                                    .child(identity),
                            )
                        })
                        .child(
                            div()
                                .w(px(24.0))
                                .text_center()
                                .text_color(PRIMARY)
                                .child(if selected { "✓" } else { "›" }),
                        ),
                );
            }
        }

        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0dd4))
            .child(
                div()
                    .w(px(520.0))
                    .max_h(px(520.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(SURFACE)
                    .shadow_lg()
                    .overflow_hidden()
                    .child(
                        div()
                            .h(px(62.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(BORDER)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(3.0))
                                    .child(div().font_weight(FontWeight::MEDIUM).child(title))
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .text_color(MUTED)
                                            .child(description),
                                    ),
                            )
                            .child(
                                div()
                                    .size(px(30.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(7.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(SURFACE_HOVER).text_color(TEXT))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.close_ssh_hosts(cx)),
                                    )
                                    .child("×"),
                            ),
                    )
                    .child(hosts),
            )
    }

    fn render_diagnostics(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let loading = self.diagnostics.as_ref().is_some_and(Vec::is_empty);
        let mut steps = div()
            .flex()
            .flex_col()
            .gap(px(9.0))
            .w_full()
            .max_h(px(562.0))
            .id("diagnostics-scroll")
            .overflow_y_scroll()
            .p(px(18.0));
        if loading {
            steps = steps.child(
                div()
                    .h(px(160.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(10.0))
                    .text_color(MUTED)
                    .child(
                        div()
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(14.0))
                            .border_1()
                            .border_color(PRIMARY)
                            .text_color(PRIMARY)
                            .child("•••"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .child(self.language.pick("正在检查连接…", "Checking connection…")),
                    ),
            );
        }
        for step in self.diagnostics.as_ref().into_iter().flatten() {
            let good = step.status == "success";
            let warning = step.status == "warning";
            steps = steps.child(
                div()
                    .flex()
                    .w_full()
                    .min_w_0()
                    .gap(px(10.0))
                    .p(px(11.0))
                    .rounded(px(8.0))
                    .bg(APP_BG)
                    .child(
                        div()
                            .mt(px(3.0))
                            .size(px(8.0))
                            .flex_none()
                            .rounded(px(4.0))
                            .bg(if good {
                                SUCCESS
                            } else if warning {
                                color(0xd8a94a)
                            } else {
                                color(0xc45b5b)
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_grow(1.0)
                            .min_w_0()
                            .whitespace_normal()
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .whitespace_normal()
                                    .text_size(px(12.0))
                                    .text_color(TEXT)
                                    .child(step.name.clone()),
                            )
                            .child(
                                div()
                                    .mt(px(3.0))
                                    .w_full()
                                    .min_w_0()
                                    .whitespace_normal()
                                    .text_size(px(10.0))
                                    .text_color(MUTED)
                                    .child(step.message.clone()),
                            ),
                    ),
            );
        }
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0db8))
            .child(
                div()
                    .w(px(600.0))
                    .max_h(px(620.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(SURFACE)
                    .overflow_hidden()
                    .child(
                        div()
                            .h(px(58.0))
                            .px(px(18.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(BORDER)
                            .child(
                                div().font_weight(FontWeight::MEDIUM).child(
                                    self.language.pick("连接诊断", "Connection diagnostics"),
                                ),
                            )
                            .child(
                                div()
                                    .size(px(30.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(7.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.close_diagnostics(cx)),
                                    )
                                    .child("×"),
                            ),
                    )
                    .child(steps),
            )
    }

    fn render_save_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tunnel = self.save_confirmation.as_ref().expect("save confirmation");
        let message = if self.language == Language::Zh {
            format!(
                "“{}”当前正在运行。保存修改会先断开现有连接，然后立即使用新配置重新连接。",
                tunnel.name
            )
        } else {
            format!(
                "“{}” is currently running. Saving will disconnect it and immediately reconnect with the updated configuration.",
                tunnel.name
            )
        };
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0dd6))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(440.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(color(0x171d29))
                    .p(px(22.0))
                    .child(
                        div()
                            .text_size(px(16.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(TEXT)
                            .child(self.language.pick("断开并重新连接？", "Reconnect tunnel?")),
                    )
                    .child(
                        div()
                            .mt(px(10.0))
                            .text_size(px(12.0))
                            .line_height(relative(1.55))
                            .text_color(MUTED)
                            .child(message),
                    )
                    .child(
                        div()
                            .mt(px(22.0))
                            .flex()
                            .justify_end()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .h(px(36.0))
                                    .px(px(15.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(12.0))
                                    .text_color(TEXT)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.cancel_save_confirmation(cx)
                                        }),
                                    )
                                    .child(self.language.pick("取消", "Cancel")),
                            )
                            .child(
                                div()
                                    .h(px(36.0))
                                    .px(px(15.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.0))
                                    .bg(PRIMARY)
                                    .text_size(px(12.0))
                                    .text_color(PRIMARY_TEXT)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.confirm_save_and_restart(cx)
                                        }),
                                    )
                                    .child(self.language.pick("保存并重连", "Save and reconnect")),
                            ),
                    ),
            )
    }

    fn render_import_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0dd6))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(440.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(color(0x171d29))
                    .p(px(22.0))
                    .child(
                        div()
                            .text_size(px(16.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(TEXT)
                            .child(self.language.pick(
                                "停止隧道并导入？",
                                "Stop tunnels and import?",
                            )),
                    )
                    .child(
                        div()
                            .mt(px(10.0))
                            .text_size(px(12.0))
                            .line_height(relative(1.55))
                            .text_color(MUTED)
                            .child(self.language.pick(
                                "导入备份会停止当前运行中的隧道，然后用所选备份替换现有配置。",
                                "Importing a backup stops active tunnels and replaces the current configuration with the selected backup.",
                            )),
                    )
                    .child(
                        div()
                            .mt(px(22.0))
                            .flex()
                            .justify_end()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .h(px(36.0))
                                    .px(px(15.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(12.0))
                                    .text_color(TEXT)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.cancel_import_backup(cx)
                                        }),
                                    )
                                    .child(self.language.pick("取消", "Cancel")),
                            )
                            .child(
                                div()
                                    .h(px(36.0))
                                    .px(px(15.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.0))
                                    .bg(PRIMARY)
                                    .text_size(px(12.0))
                                    .text_color(PRIMARY_TEXT)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.confirm_import_backup(cx)
                                        }),
                                    )
                                    .child(self.language.pick("继续导入", "Continue")),
                            ),
                    ),
            )
    }

    fn render_group_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.group_form.as_ref().expect("group form");
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0db8))
            .child(
                div()
                    .w(px(430.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(SURFACE)
                    .child(
                        div()
                            .h(px(56.0))
                            .px(px(18.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(BORDER)
                            .child(div().font_weight(FontWeight::MEDIUM).child(
                                if form.editing_id.is_some() {
                                    self.language.pick("编辑分组", "Edit group")
                                } else {
                                    self.language.pick("新建分组", "New group")
                                },
                            ))
                            .child(
                                div()
                                    .size(px(30.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(7.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.close_group_form(cx)),
                                    )
                                    .child("×"),
                            ),
                    )
                    .child(
                        div()
                            .p(px(18.0))
                            .flex()
                            .flex_col()
                            .gap(px(13.0))
                            .child(Self::form_field(
                                self.language.pick("名称", "Name"),
                                form.name.clone(),
                            ))
                            .child(Self::form_field(
                                self.language.pick("说明", "Description"),
                                form.description.clone(),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap(px(8.0))
                                    .mt(px(5.0))
                                    .child(
                                        div()
                                            .h(px(34.0))
                                            .px(px(12.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(8.0))
                                            .border_1()
                                            .border_color(BORDER)
                                            .text_size(px(11.0))
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    this.close_group_form(cx)
                                                }),
                                            )
                                            .child(self.language.pick("取消", "Cancel")),
                                    )
                                    .child(
                                        div()
                                            .h(px(34.0))
                                            .px(px(13.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(8.0))
                                            .bg(PRIMARY)
                                            .text_size(px(11.0))
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| this.save_group(cx)),
                                            )
                                            .child(self.language.pick("保存", "Save")),
                                    ),
                            ),
                    ),
            )
    }

    fn render_auth_prompt(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (title, body) = match self.auth_prompt.as_ref().expect("auth prompt") {
            AuthPrompt::HostKey {
                host,
                port,
                fingerprint,
                ..
            } => (
                self.language.pick("信任 SSH 主机密钥", "Trust SSH host key"),
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(MUTED)
                            .child(if self.language == Language::Zh {
                                format!("首次连接 {host}:{port}，请核对指纹后再信任。")
                            } else {
                                format!(
                                    "First connection to {host}:{port}. Verify the fingerprint before trusting it."
                                )
                            }),
                    )
                    .child(
                        div()
                            .p(px(10.0))
                            .rounded(px(8.0))
                            .bg(APP_BG)
                            .text_size(px(10.0))
                            .text_color(TEXT)
                            .child(fingerprint.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .h(px(34.0))
                                    .px(px(12.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(11.0))
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.close_auth_prompt(cx)),
                                    )
                                    .child(self.language.pick("取消", "Cancel")),
                            )
                            .child(
                                div()
                                    .h(px(34.0))
                                    .px(px(13.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.0))
                                    .bg(PRIMARY)
                                    .text_size(px(11.0))
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.trust_prompted_host(cx)),
                                    )
                                    .child(self.language.pick("信任并重试", "Trust and retry")),
                            ),
                    ),
            ),
            AuthPrompt::Passphrase { input, .. } => (
                self.language.pick("输入私钥口令", "Enter private key passphrase"),
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(MUTED)
                            .child(self.language.pick(
                                "该私钥已加密，口令只用于本次连接，不会写入配置文件。",
                                "This key is encrypted. The passphrase is used only for this connection and is never saved.",
                            )),
                    )
                    .child(Self::form_field(
                        self.language.pick("私钥口令", "Private key passphrase"),
                        input.clone(),
                    ))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .h(px(34.0))
                                    .px(px(12.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(11.0))
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.close_auth_prompt(cx)),
                                    )
                                    .child(self.language.pick("取消", "Cancel")),
                            )
                            .child(
                                div()
                                    .h(px(34.0))
                                    .px(px(13.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.0))
                                    .bg(PRIMARY)
                                    .text_size(px(11.0))
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.submit_passphrase(cx)),
                                    )
                                    .child(self.language.pick("连接", "Connect")),
                            ),
                    ),
            ),
        };
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0dd4))
            .child(
                div()
                    .w(px(470.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(SURFACE)
                    .child(
                        div()
                            .h(px(56.0))
                            .px(px(18.0))
                            .flex()
                            .items_center()
                            .border_b_1()
                            .border_color(BORDER)
                            .font_weight(FontWeight::MEDIUM)
                            .child(title),
                    )
                    .child(div().p(px(18.0)).child(body)),
            )
    }

    fn render_about(&self, cx: &mut Context<Self>) -> impl IntoElement {
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0dd4))
            .child(
                div()
                    .w(px(360.0))
                    .p(px(26.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .rounded(px(16.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(SURFACE)
                    .shadow_lg()
                    .child(img(self.logo.clone()).size(px(72.0)).rounded(px(17.0)))
                    .child(
                        div()
                            .mt(px(16.0))
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(TEXT)
                            .child("Tunnel Mate"),
                    )
                    .child(
                        div()
                            .mt(px(5.0))
                            .text_size(px(11.0))
                            .text_color(MUTED)
                            .child(format!("Version {}", env!("CARGO_PKG_VERSION"))),
                    )
                    .child(
                        div()
                            .mt(px(13.0))
                            .text_size(px(11.0))
                            .text_color(MUTED)
                            .child(self.language.pick(
                                "简洁、可靠的 SSH 隧道管理工具",
                                "A focused, reliable SSH tunnel manager",
                            )),
                    )
                    .child(
                        div()
                            .mt(px(22.0))
                            .h(px(34.0))
                            .px(px(18.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(8.0))
                            .bg(PRIMARY)
                            .text_size(px(11.0))
                            .text_color(PRIMARY_TEXT)
                            .cursor_pointer()
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.close_about(cx)),
                            )
                            .child(self.language.pick("好", "OK")),
                    ),
            )
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.settings_form.as_ref().expect("settings form");
        let row = |label: &'static str,
                   description: &'static str,
                   enabled: bool,
                   toggle: SettingToggle| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .py(px(13.0))
                .border_b_1()
                .border_color(BORDER)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(div().text_size(px(12.0)).text_color(TEXT).child(label))
                        .child(
                            div()
                                .mt(px(3.0))
                                .text_size(px(10.0))
                                .text_color(MUTED)
                                .child(description),
                        ),
                )
                .child(
                    div()
                        .w(px(42.0))
                        .h(px(24.0))
                        .rounded(px(12.0))
                        .p(px(3.0))
                        .bg(if enabled { PRIMARY } else { rgb(0x343840) })
                        .cursor_pointer()
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| this.toggle_setting(toggle, cx)),
                        )
                        .child(
                            div()
                                .size(px(18.0))
                                .rounded(px(9.0))
                                .bg(TEXT)
                                .when(enabled, |dot| dot.ml(px(18.0))),
                        ),
                )
        };
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0db8))
            .child(
                div()
                    .w(px(520.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(SURFACE)
                    .child(
                        div()
                            .h(px(58.0))
                            .px(px(18.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(BORDER)
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(self.language.pick("设置", "Settings")),
                            )
                            .child(
                                div()
                                    .size(px(30.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(7.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.cancel_settings(cx)),
                                    )
                                    .child("×"),
                            ),
                    )
                    .child(
                        div()
                            .px(px(18.0))
                            .pb(px(18.0))
                            .child(row(
                                self.language.pick("开机启动", "Launch at login"),
                                self.language.pick(
                                    "登录系统后自动运行 Tunnel Mate",
                                    "Run Tunnel Mate after signing in",
                                ),
                                form.launch_on_startup,
                                SettingToggle::Launch,
                            ))
                            .child(row(
                                self.language.pick(
                                    "登录启动时隐藏窗口",
                                    "Hide window at login",
                                ),
                                self.language.pick(
                                    "仅影响登录系统后的自动启动，手动打开始终显示窗口",
                                    "Only affects login launch; manual launch always shows the window",
                                ),
                                form.start_minimized,
                                SettingToggle::Minimized,
                            ))
                            .child(row(
                                self.language.pick("关闭到托盘", "Close to tray"),
                                self.language.pick(
                                    "点关闭按钮只隐藏窗口，隧道继续运行；可从 Dock 或菜单栏恢复",
                                    "Hide the window without stopping tunnels; restore from Dock or menu bar",
                                ),
                                form.close_to_tray,
                                SettingToggle::CloseToTray,
                            ))
                            .child(
                                div()
                                    .mt(px(14.0))
                                    .flex()
                                    .gap(px(10.0))
                                    .child(
                                        div().flex_grow(1.0).child(Self::form_field(
                                            self.language
                                                .pick("保活间隔（秒）", "Keep-alive (seconds)"),
                                            form.keep_alive.clone(),
                                        )),
                                    )
                                    .child(div().flex_grow(1.0).child(
                                        Self::form_field(
                                            self.language.pick(
                                                "连接超时（秒）",
                                                "Connection timeout (seconds)",
                                            ),
                                            form.connect_timeout.clone(),
                                        ),
                                    )),
                            )
                            .child(div().mt(px(12.0)).child(Self::form_field(
                                self.language.pick("SSH config 路径", "SSH config path"),
                                form.ssh_config_path.clone(),
                            )))
                            .child(
                                div()
                                    .mt(px(16.0))
                                    .flex()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .h(px(34.0))
                                            .px(px(12.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(8.0))
                                            .border_1()
                                            .border_color(BORDER)
                                            .text_size(px(11.0))
                                            .text_color(TEXT)
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    this.export_backup(cx)
                                                }),
                                            )
                                            .child(self.language.pick("导出备份", "Export backup")),
                                    )
                                    .child(
                                        div()
                                            .h(px(34.0))
                                            .px(px(12.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(8.0))
                                            .border_1()
                                            .border_color(BORDER)
                                            .text_size(px(11.0))
                                            .text_color(TEXT)
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    this.import_backup(cx)
                                                }),
                                            )
                                            .child(self.language.pick("导入备份", "Import backup")),
                                    )
                                    .child(div().flex_grow(1.0))
                                    .child(
                                        div()
                                            .h(px(34.0))
                                            .px(px(13.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(8.0))
                                            .bg(PRIMARY)
                                            .text_size(px(11.0))
                                            .text_color(PRIMARY_TEXT)
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    this.save_settings(cx)
                                                }),
                                            )
                                            .child(self.language.pick("保存", "Save")),
                                    ),
                            ),
                    ),
            )
    }
}

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
            .pt(px(38.0))
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

fn parse_host_key_prompt(message: &str) -> Option<(String, u16, String)> {
    let value = message.strip_prefix("HOST_KEY_NOT_TRUSTED|")?;
    let mut parts = value.splitn(3, '|');
    let host = parts.next()?.to_string();
    let port = parts.next()?.parse().ok()?;
    let fingerprint = parts.next()?.to_string();
    (!host.is_empty() && !fingerprint.is_empty()).then_some((host, port, fingerprint))
}

fn install_native_macos_behavior(cx: &mut App, language: Language) {
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
        let window_size = size(px(1050.0), px(680.0));
        let bounds = cx
            .primary_display()
            .map(|display| Bounds::centered_at(display.visible_bounds().center(), window_size))
            .unwrap_or_else(|| Bounds::centered(None, window_size, cx));
        let window_handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_background: WindowBackgroundAppearance::Blurred,
                    titlebar: Some(gpui::TitlebarOptions {
                        title: None,
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(16.0), px(16.0))),
                    }),
                    ..Default::default()
                },
                |window, cx| {
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
                },
            )
            .expect("failed to open Tunnel Mate window");
        let app = window_handle
            .entity(cx)
            .expect("failed to access Tunnel Mate root view");
        register_global_actions(cx, window_handle, app);
        // Tray initialization uses a native menu backend that replaces NSApp's main menu.
        // Install the application menus afterwards so the standard macOS menus remain intact.
        install_native_macos_behavior(cx, Language::system());
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
    use super::{parse_host_key_prompt, ssh_host_matches, Language, SshHostConfig};

    #[test]
    fn parses_host_key_intervention_message() {
        assert_eq!(
            parse_host_key_prompt("HOST_KEY_NOT_TRUSTED|example.com|2222|SHA256:abc"),
            Some(("example.com".to_string(), 2222, "SHA256:abc".to_string()))
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
