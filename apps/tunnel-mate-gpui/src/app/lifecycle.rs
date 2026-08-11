use super::*;

impl TunnelMateApp {
    pub(super) fn load(cx: &mut Context<Self>) -> Self {
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
        let mut logo =
            image::load_from_memory(include_bytes!("../../../../assets/icons/128x128.png"))
                .expect("embedded app icon must be valid")
                .into_rgba8();
        for pixel in logo.pixels_mut() {
            pixel.0.swap(0, 2);
        }
        let keystroke_subscription = cx.observe_keystrokes(|this, event, window, cx| {
            if event.keystroke.key == "escape" {
                this.dismiss(cx);
            } else if event.keystroke.key == "tab" {
                if event.keystroke.modifiers.shift {
                    window.focus_prev(cx);
                } else {
                    window.focus_next(cx);
                }
                cx.stop_propagation();
            } else if event.keystroke.key == "enter" {
                this.submit_primary(cx);
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
            delete_confirmation: None,
            pending_delete: None,
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

    pub(super) fn handle_message(&mut self, message: AppMessage, cx: &mut Context<Self>) {
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
                    } else if let Some((issue, host, port, fingerprint)) =
                        parse_host_key_prompt(message)
                    {
                        intervention = true;
                        self.auth_prompt = Some(AuthPrompt::HostKey {
                            tunnel_id: payload.tunnel_id.clone(),
                            issue,
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
                            self.notice = Some(self.language.runtime_message(&message).into());
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
                        format!(
                            "“{tunnel_name}”操作失败：{}",
                            self.language.runtime_message(&message)
                        )
                    } else {
                        format!("Operation failed for “{tunnel_name}”: {message}")
                    }
                    .into(),
                );
            }
            AppMessage::DeleteReady(tunnel_id) => {
                if self.pending_delete.as_deref() == Some(tunnel_id.as_str()) {
                    self.pending_delete = None;
                    self.delete_tunnel(tunnel_id, cx);
                }
            }
            AppMessage::DeleteFailed {
                tunnel_name,
                message,
            } => {
                self.pending_delete = None;
                self.notice = Some(
                    if self.language == Language::Zh {
                        format!(
                            "无法删除“{tunnel_name}”：停止连接失败：{}",
                            self.language.runtime_message(&message)
                        )
                    } else {
                        format!(
                            "Could not delete “{tunnel_name}” because stopping it failed: {message}"
                        )
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
                    TunnelManager::stop_all(manager).await;
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

    pub(super) fn refresh_tray(&mut self) {
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
}
