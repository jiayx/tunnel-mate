use super::*;

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum TunnelFilter {
    All,
    Active,
    Activity,
    Group(String),
}

pub(crate) enum AppMessage {
    Runtime(RuntimeEvent),
    #[cfg(target_os = "macos")]
    WindowContentTopInset(f32),
    OperationError {
        tunnel_name: String,
        message: String,
    },
    Diagnostics {
        request_id: u64,
        tunnel_id: String,
        steps: Vec<DiagnosticStep>,
    },
    ImportSelected(AppConfig),
    ImportFailed(String),
    ConfigImported(AppConfig),
    DeleteReady(String),
    DeleteFailed {
        tunnel_name: String,
        message: String,
    },
    FileOperation {
        message: String,
        transient: bool,
    },
    PrivateKeySelected {
        target: PrivateKeyTarget,
        path: String,
    },
    Tray(String),
    QuitReady,
    HostTrusted(String),
}

pub(crate) struct ActivityEvents {
    events: RwLock<VecDeque<LogEvent>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoticeKind {
    Transient,
    Progress,
    Persistent,
}

pub(crate) struct AppNotice {
    pub(crate) id: u64,
    pub(crate) message: SharedString,
    pub(crate) kind: NoticeKind,
    pub(crate) tunnel_id: Option<String>,
}

pub(crate) struct DiagnosticState {
    pub(crate) scroll: gpui::ScrollHandle,
    pub(crate) request_id: u64,
    pub(crate) tunnel_id: String,
    pub(crate) tunnel_name: String,
    pub(crate) address: String,
    pub(crate) steps: Option<Vec<DiagnosticStep>>,
    pub(crate) copied: bool,
    pub(crate) task: Option<tokio::task::JoinHandle<()>>,
}

impl DiagnosticState {
    pub(crate) fn complete(
        &mut self,
        request_id: u64,
        tunnel_id: &str,
        steps: Vec<DiagnosticStep>,
    ) -> bool {
        if self.request_id != request_id || self.tunnel_id != tunnel_id {
            return false;
        }
        self.steps = Some(steps);
        self.task.take();
        true
    }
}

impl Drop for DiagnosticState {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

impl ActivityEvents {
    pub(crate) fn new(events: Vec<LogEvent>) -> Self {
        Self {
            events: RwLock::new(events.into()),
        }
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, VecDeque<LogEvent>> {
        self.events
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, VecDeque<LogEvent>> {
        self.events
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.read().is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.read().len()
    }

    pub(crate) fn newest_in(&self, range: Range<usize>) -> Vec<LogEvent> {
        let events = self.read();
        let event_count = events.len();
        range
            .filter_map(|index| {
                event_count
                    .checked_sub(index + 1)
                    .and_then(|index| events.get(index))
                    .cloned()
            })
            .collect()
    }

    pub(crate) fn push(&self, event: LogEvent) {
        let mut events = self.write();
        events.push_back(event);
        if events.len() > 1_000 {
            events.pop_front();
        }
    }

    pub(crate) fn clear(&self) {
        self.write().clear();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ForwardKind {
    Local,
    Remote,
    Socks5,
}

#[derive(Clone, Copy)]
pub(crate) enum SettingToggle {
    Launch,
    Minimized,
    CloseToTray,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SshPickerTarget {
    Primary,
    JumpHost,
}

#[derive(Clone, Copy)]
pub(crate) enum PrivateKeyTarget {
    Primary,
    JumpHost,
}

pub(crate) struct TunnelForm {
    pub(crate) editing_id: Option<String>,
    pub(crate) validation_error: Option<SharedString>,
    pub(crate) kind: ForwardKind,
    pub(crate) advanced: bool,
    pub(crate) authentication_expanded: bool,
    pub(crate) scroll: gpui::ScrollHandle,
    pub(crate) start_after_save: bool,
    pub(crate) auto_reconnect: bool,
    pub(crate) start_with_app: bool,
    pub(crate) jump_enabled: bool,
    pub(crate) jump_host_id: Option<String>,
    pub(crate) group_id: Option<String>,
    pub(crate) group_menu_open: bool,
    pub(crate) group_menu_scroll: gpui::ScrollHandle,
    pub(crate) ssh_picker_scroll: gpui::ScrollHandle,
    pub(crate) name: Entity<TextInput>,
    pub(crate) description: Entity<TextInput>,
    pub(crate) ssh_host: Entity<TextInput>,
    pub(crate) ssh_port: Entity<TextInput>,
    pub(crate) ssh_user: Entity<TextInput>,
    pub(crate) identity_file: Entity<TextInput>,
    pub(crate) ssh_password: Entity<TextInput>,
    pub(crate) jump_host: Entity<TextInput>,
    pub(crate) jump_port: Entity<TextInput>,
    pub(crate) jump_user: Entity<TextInput>,
    pub(crate) jump_identity_file: Entity<TextInput>,
    pub(crate) jump_password: Entity<TextInput>,
    pub(crate) listen_host: Entity<TextInput>,
    pub(crate) listen_port: Entity<TextInput>,
    pub(crate) target_host: Entity<TextInput>,
    pub(crate) target_port: Entity<TextInput>,
    pub(crate) retry_count: Entity<TextInput>,
    pub(crate) retry_interval: Entity<TextInput>,
    pub(crate) ssh_hosts: Vec<SshHostConfig>,
    pub(crate) ssh_picker_target: Option<SshPickerTarget>,
}

pub(crate) struct SettingsForm {
    pub(crate) scroll: gpui::ScrollHandle,
    pub(crate) validation_error: Option<SharedString>,
    pub(crate) launch_on_startup: bool,
    pub(crate) start_minimized: bool,
    pub(crate) close_to_tray: bool,
    pub(crate) keep_alive: Entity<TextInput>,
    pub(crate) connect_timeout: Entity<TextInput>,
    pub(crate) ssh_config_path: Entity<TextInput>,
}

pub(crate) struct GroupForm {
    pub(crate) validation_error: Option<SharedString>,
    pub(crate) editing_id: Option<String>,
    pub(crate) name: Entity<TextInput>,
    pub(crate) description: Entity<TextInput>,
}

pub(crate) enum AuthPrompt {
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
pub(crate) enum HostKeyIssue {
    Unknown,
    Changed,
    Revoked,
}

impl TunnelForm {
    pub(crate) fn new(
        tunnel: Option<&Tunnel>,
        language: Language,
        cx: &mut Context<TunnelMateApp>,
    ) -> Self {
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
        let form = Self {
            editing_id: tunnel.map(|tunnel| tunnel.id.clone()),
            validation_error: None,
            kind,
            advanced: false,
            authentication_expanded: tunnel
                .is_some_and(|t| t.ssh_identity_file.is_some() || t.ssh_password.is_some()),
            scroll: gpui::ScrollHandle::new(),
            start_after_save: tunnel.is_none(),
            auto_reconnect: tunnel.map(|tunnel| tunnel.auto_reconnect).unwrap_or(true),
            start_with_app: tunnel.map(|tunnel| tunnel.start_with_app).unwrap_or(true),
            jump_enabled: tunnel
                .map(|tunnel| tunnel.jump_host_enabled)
                .unwrap_or(false),
            jump_host_id: tunnel.and_then(|tunnel| tunnel.jump_host_id.clone()),
            group_id: tunnel.and_then(|tunnel| tunnel.group_id.clone()),
            group_menu_open: false,
            group_menu_scroll: gpui::ScrollHandle::new(),
            ssh_picker_scroll: gpui::ScrollHandle::new(),
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
                        "密码（可选，安全保存到系统凭据库）",
                        "Password (optional, stored securely by the system)",
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
                language.pick(
                    "~/.ssh/id_ed25519（留空则自动选择）",
                    "~/.ssh/id_ed25519 (blank for automatic)",
                ),
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
        };
        for input in [
            &form.name,
            &form.description,
            &form.ssh_host,
            &form.ssh_port,
            &form.ssh_user,
            &form.identity_file,
            &form.ssh_password,
            &form.listen_host,
            &form.listen_port,
            &form.target_host,
            &form.target_port,
            &form.retry_count,
            &form.retry_interval,
            &form.jump_host,
            &form.jump_port,
            &form.jump_user,
            &form.jump_identity_file,
            &form.jump_password,
        ] {
            input.update(cx, |input, _| input.set_scroll_parent(form.scroll.clone()));
            cx.subscribe(input, |this, _, _: &text_input::InputChanged, cx| {
                if let Some(form) = &mut this.form {
                    form.validation_error = None;
                }
                cx.notify();
            })
            .detach();
        }
        form
    }
}

pub(crate) struct TunnelMateApp {
    pub(crate) theme: Theme,
    pub(crate) language: Language,
    pub(crate) logo: Arc<RenderImage>,
    pub(crate) config: AppConfig,
    pub(crate) search: Entity<TextInput>,
    pub(crate) filter: TunnelFilter,
    pub(crate) selected_tunnel: Option<String>,
    pub(crate) form: Option<TunnelForm>,
    pub(crate) notice: Option<AppNotice>,
    pub(crate) next_notice_id: u64,
    pub(crate) notice_task: Option<Task<()>>,
    pub(crate) load_error: Option<SharedString>,
    pub(crate) statuses: HashMap<String, TunnelStatus>,
    pub(crate) pending_starts: HashSet<String>,
    pub(crate) events: Arc<ActivityEvents>,
    pub(crate) activity_scroll: UniformListScrollHandle,
    pub(crate) diagnostics: Option<DiagnosticState>,
    pub(crate) next_diagnostic_id: u64,
    pub(crate) tunnel_scroll: UniformListScrollHandle,
    pub(crate) groups_scroll: gpui::ScrollHandle,
    pub(crate) auth_prompt_scroll: gpui::ScrollHandle,
    pub(crate) root_focus: gpui::FocusHandle,
    pub(crate) modal_focus: gpui::FocusHandle,
    pub(crate) return_focus: Option<gpui::FocusHandle>,
    pub(crate) active_modal: Option<ModalLayer>,
    pub(crate) pending_field_focus: Option<Entity<TextInput>>,
    pub(crate) settings_form: Option<SettingsForm>,
    pub(crate) pending_import: Option<AppConfig>,
    pub(crate) group_form: Option<GroupForm>,
    pub(crate) save_confirmation: Option<Tunnel>,
    pub(crate) delete_confirmation: Option<String>,
    pub(crate) group_delete_confirmation: Option<String>,
    pub(crate) pending_delete: Option<String>,
    pub(crate) auth_prompt: Option<AuthPrompt>,
    pub(crate) pending_passphrases: HashMap<String, String>,
    pub(crate) about_open: bool,
    pub(crate) manager: Arc<Mutex<TunnelManager>>,
    pub(crate) runtime: Arc<Runtime>,
    pub(crate) messages: async_channel::Sender<AppMessage>,
    #[cfg(target_os = "macos")]
    pub(crate) window_content_top_inset: f32,
    #[cfg(target_os = "macos")]
    pub(crate) _window_layout_observer: Option<WindowLayoutObserver>,
    pub(crate) _event_task: Task<()>,
    pub(crate) _tray: Option<tray_icon::TrayIcon>,
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;

    fn running(request_id: u64, tunnel_id: &str) -> DiagnosticState {
        DiagnosticState {
            scroll: gpui::ScrollHandle::new(),
            request_id,
            tunnel_id: tunnel_id.into(),
            tunnel_name: "Test tunnel".into(),
            address: "127.0.0.1:22".into(),
            steps: None,
            copied: false,
            task: None,
        }
    }

    #[test]
    fn earlier_diagnostic_cannot_replace_a_new_check() {
        let mut current = running(2, "tunnel-b");
        assert!(!current.complete(1, "tunnel-b", vec![]));
        assert!(!current.complete(2, "tunnel-a", vec![]));
        assert!(current.steps.is_none());
        assert!(current.complete(2, "tunnel-b", vec![]));
        assert!(current.steps.is_some());
    }

    #[test]
    fn closing_diagnostics_cancels_the_in_flight_check() {
        struct SignalOnDrop(Option<tokio::sync::oneshot::Sender<()>>);
        impl Drop for SignalOnDrop {
            fn drop(&mut self) {
                if let Some(sender) = self.0.take() {
                    let _ = sender.send(());
                }
            }
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let (started, ready) = tokio::sync::oneshot::channel();
            let (cancelled, finished) = tokio::sync::oneshot::channel();
            let mut state = running(1, "tunnel-a");
            state.task = Some(tokio::spawn(async move {
                let _guard = SignalOnDrop(Some(cancelled));
                let _ = started.send(());
                std::future::pending::<()>().await;
            }));
            ready.await.unwrap();
            drop(state);
            tokio::time::timeout(Duration::from_secs(1), finished)
                .await
                .unwrap()
                .unwrap();
        });
    }
}
