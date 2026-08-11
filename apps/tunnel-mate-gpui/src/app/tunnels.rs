use super::*;

impl TunnelMateApp {
    pub(super) fn save_form(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &self.form else { return };
        let language = self.language;
        let manual_jump = form.jump_enabled && form.jump_host_id.is_none();
        let advanced_has_error = form
            .retry_count
            .read(cx)
            .value()
            .trim()
            .parse::<u32>()
            .is_err()
            || form
                .retry_interval
                .read(cx)
                .value()
                .trim()
                .parse::<u32>()
                .is_err()
            || (manual_jump
                && form
                    .jump_port
                    .read(cx)
                    .value()
                    .trim()
                    .parse::<u16>()
                    .map_or(true, |port| port == 0));
        let mut missing = Vec::new();
        let mut require = |input: &Entity<TextInput>, label: &'static str| {
            if input.read(cx).value().trim().is_empty() {
                missing.push(label);
            }
        };
        require(&form.name, language.pick("名称", "Name"));
        require(&form.ssh_host, language.pick("SSH 主机", "SSH host"));
        require(&form.ssh_port, language.pick("SSH 端口", "SSH port"));
        require(&form.ssh_user, language.pick("SSH 用户", "SSH user"));
        let (listen_host_label, listen_port_label) = match form.kind {
            ForwardKind::Local => (
                language.pick("本机监听地址", "Local listen address"),
                language.pick("本机端口", "Local port"),
            ),
            ForwardKind::Remote => (
                language.pick("远端监听地址", "Remote listen address"),
                language.pick("远端端口", "Remote port"),
            ),
            ForwardKind::Socks5 => (
                language.pick("SOCKS5 监听地址", "SOCKS5 listen address"),
                language.pick("SOCKS5 端口", "SOCKS5 port"),
            ),
        };
        require(&form.listen_host, listen_host_label);
        require(&form.listen_port, listen_port_label);
        if form.kind != ForwardKind::Socks5 {
            let (target_host_label, target_port_label) = match form.kind {
                ForwardKind::Local => (
                    language.pick("远端目标地址", "Remote target address"),
                    language.pick("远端目标端口", "Remote target port"),
                ),
                ForwardKind::Remote => (
                    language.pick("本机目标地址", "Local target address"),
                    language.pick("本机目标端口", "Local target port"),
                ),
                ForwardKind::Socks5 => unreachable!(),
            };
            require(&form.target_host, target_host_label);
            require(&form.target_port, target_port_label);
        }
        require(&form.retry_count, language.pick("重试次数", "Retry count"));
        require(
            &form.retry_interval,
            language.pick("重试间隔", "Retry interval"),
        );
        if manual_jump {
            require(&form.jump_host, language.pick("跳板机主机", "Jump host"));
            require(
                &form.jump_port,
                language.pick("跳板机端口", "Jump host port"),
            );
            require(
                &form.jump_user,
                language.pick("跳板机用户", "Jump host user"),
            );
        }
        if !missing.is_empty() {
            let message = if language == Language::Zh {
                format!("请填写以下必填项：{}", missing.join("、"))
            } else {
                format!("Complete the required fields: {}", missing.join(", "))
            };
            if let Some(form) = &mut self.form {
                form.validation_error = Some(message.into());
                form.advanced |= advanced_has_error;
            }
            self.notice = None;
            cx.notify();
            return;
        }

        let parse_port = |input: &Entity<TextInput>, label: &str| -> Result<u16, String> {
            input
                .read(cx)
                .value()
                .trim()
                .parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .ok_or_else(|| {
                    if language == Language::Zh {
                        format!("{label}必须是 1–65535 的端口")
                    } else {
                        format!("{label} must be a port from 1 to 65535")
                    }
                })
        };
        let parse_u32 = |input: &Entity<TextInput>, label: &str| -> Result<u32, String> {
            input.read(cx).value().trim().parse::<u32>().map_err(|_| {
                if language == Language::Zh {
                    format!("{label}必须是非负整数")
                } else {
                    format!("{label} must be a non-negative integer")
                }
            })
        };
        let result = (|| {
            let listen = Endpoint {
                host: form.listen_host.read(cx).value(),
                port: parse_port(&form.listen_port, language.pick("监听端口", "Listen port"))?,
            };
            let forward = match form.kind {
                ForwardKind::Local => ForwardSpec::Local {
                    listen,
                    target: Endpoint {
                        host: form.target_host.read(cx).value(),
                        port: parse_port(
                            &form.target_port,
                            language.pick("目标端口", "Target port"),
                        )?,
                    },
                },
                ForwardKind::Remote => ForwardSpec::Remote {
                    listen,
                    target: Endpoint {
                        host: form.target_host.read(cx).value(),
                        port: parse_port(
                            &form.target_port,
                            language.pick("目标端口", "Target port"),
                        )?,
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
                Some(parse_port(
                    &form.jump_port,
                    language.pick("跳板机端口", "Jump host port"),
                )?)
            } else {
                None
            };
            let tunnel = Tunnel {
                id,
                name: form.name.read(cx).value(),
                description: (!description.trim().is_empty()).then_some(description),
                group_id: form.group_id.clone(),
                ssh_host: form.ssh_host.read(cx).value(),
                ssh_port: parse_port(&form.ssh_port, language.pick("SSH 端口", "SSH port"))?,
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
                retry_count: parse_u32(
                    &form.retry_count,
                    language.pick("重试次数", "Retry count"),
                )?,
                retry_interval: parse_u32(
                    &form.retry_interval,
                    language.pick("重试间隔", "Retry interval"),
                )?,
            };
            validate_tunnel(&tunnel)?;
            Ok::<_, String>((tunnel, form.start_after_save))
        })();
        match result {
            Err(error) => {
                if let Some(form) = &mut self.form {
                    form.validation_error = Some(error.into());
                    form.advanced |= advanced_has_error;
                }
                self.notice = None;
            }
            Ok((tunnel, start_after_save)) => {
                if let Some(form) = &mut self.form {
                    form.validation_error = None;
                }
                let existing = self
                    .config
                    .tunnels
                    .iter()
                    .find(|existing| existing.id == tunnel.id);
                if existing.is_some_and(|existing| existing == &tunnel) {
                    self.form = None;
                    self.notice = None;
                    cx.notify();
                    return;
                }
                let requires_reconnect =
                    existing.is_some_and(|existing| existing.connection_settings_differ(&tunnel));
                if self.is_active(&tunnel.id) && requires_reconnect {
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

    pub(super) fn cancel_save_confirmation(&mut self, cx: &mut Context<Self>) {
        self.save_confirmation = None;
        cx.notify();
    }

    pub(super) fn confirm_save_and_restart(&mut self, cx: &mut Context<Self>) {
        let Some(tunnel) = self.save_confirmation.take() else {
            return;
        };
        self.persist_tunnel(tunnel, true, cx);
    }

    pub(super) fn persist_tunnel(
        &mut self,
        tunnel: Tunnel,
        start_after_save: bool,
        cx: &mut Context<Self>,
    ) {
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

    pub(super) fn select_tunnel(&mut self, tunnel_id: String, cx: &mut Context<Self>) {
        self.selected_tunnel = Some(tunnel_id);
        self.notice = None;
        cx.notify();
    }

    pub(super) fn request_toggle(&mut self, tunnel_id: String, cx: &mut Context<Self>) {
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
                if let Err(message) = TunnelManager::stop_tunnel(manager, &tunnel.id).await {
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

    pub(super) fn start_tunnel_with_passphrase(
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

    pub(super) fn close_auth_prompt(&mut self, cx: &mut Context<Self>) {
        self.auth_prompt = None;
        cx.notify();
    }

    pub(super) fn copy_prompted_fingerprint(&self, cx: &mut Context<Self>) {
        if let Some(AuthPrompt::HostKey { fingerprint, .. }) = &self.auth_prompt {
            cx.write_to_clipboard(ClipboardItem::new_string(fingerprint.clone()));
        }
    }

    pub(super) fn copy_known_host_cleanup(&self, cx: &mut Context<Self>) {
        if let Some(AuthPrompt::HostKey { host, port, .. }) = &self.auth_prompt {
            let target = if *port == 22 {
                host.clone()
            } else {
                format!("[{host}]:{port}")
            };
            cx.write_to_clipboard(ClipboardItem::new_string(format!(
                "ssh-keygen -R '{target}'"
            )));
        }
    }

    pub(super) fn trust_prompted_host(&mut self, cx: &mut Context<Self>) {
        let Some(AuthPrompt::HostKey {
            tunnel_id,
            issue: HostKeyIssue::Unknown,
            host,
            port,
            fingerprint,
            ..
        }) = &self.auth_prompt
        else {
            return;
        };
        let (tunnel_id, host, port, fingerprint) =
            (tunnel_id.clone(), host.clone(), *port, fingerprint.clone());
        let error_prefix = self
            .language
            .pick("信任主机密钥失败", "Failed to trust host key")
            .to_string();
        let sender = self.messages.clone();
        self.runtime.spawn(async move {
            match tunnel_core::ssh::engine::SshSession::trust_host_key(&host, port, &fingerprint)
                .await
            {
                Ok(()) => {
                    let _ = sender.send(AppMessage::HostTrusted(tunnel_id)).await;
                }
                Err(error) => {
                    let _ = sender
                        .send(AppMessage::FileOperation(format!(
                            "{error_prefix}: {error}"
                        )))
                        .await;
                }
            }
        });
        cx.notify();
    }

    pub(super) fn begin_host_key_replacement(&mut self, cx: &mut Context<Self>) {
        if let Some(AuthPrompt::HostKey {
            issue: HostKeyIssue::Changed,
            confirm_replace,
            ..
        }) = &mut self.auth_prompt
        {
            *confirm_replace = true;
            cx.notify();
        }
    }

    pub(super) fn cancel_host_key_replacement(&mut self, cx: &mut Context<Self>) {
        if let Some(AuthPrompt::HostKey {
            confirm_replace, ..
        }) = &mut self.auth_prompt
        {
            *confirm_replace = false;
            cx.notify();
        }
    }

    pub(super) fn replace_prompted_host_key(&mut self, cx: &mut Context<Self>) {
        let Some(AuthPrompt::HostKey {
            tunnel_id,
            issue: HostKeyIssue::Changed,
            host,
            port,
            fingerprint,
            confirm_replace: true,
            ..
        }) = &self.auth_prompt
        else {
            return;
        };
        let (tunnel_id, host, port, fingerprint) =
            (tunnel_id.clone(), host.clone(), *port, fingerprint.clone());
        let error_prefix = self
            .language
            .pick("更新主机密钥失败", "Failed to update host key")
            .to_string();
        let sender = self.messages.clone();
        self.runtime.spawn(async move {
            match tunnel_core::ssh::engine::SshSession::replace_host_key(&host, port, &fingerprint)
                .await
            {
                Ok(()) => {
                    let _ = sender.send(AppMessage::HostTrusted(tunnel_id)).await;
                }
                Err(error) => {
                    let _ = sender
                        .send(AppMessage::FileOperation(format!(
                            "{error_prefix}: {error}"
                        )))
                        .await;
                }
            }
        });
        cx.notify();
    }

    pub(super) fn submit_passphrase(&mut self, cx: &mut Context<Self>) {
        let Some(AuthPrompt::Passphrase { tunnel_id, input }) = &self.auth_prompt else {
            return;
        };
        let tunnel_id = tunnel_id.clone();
        let passphrase = input.read(cx).value();
        self.auth_prompt = None;
        self.start_tunnel_with_passphrase(tunnel_id, Some(passphrase), cx);
    }
}
