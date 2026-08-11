use super::*;

impl TunnelMateApp {
    pub(super) fn set_filter(&mut self, filter: TunnelFilter, cx: &mut Context<Self>) {
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

    pub(super) fn open_create_sheet(&mut self, cx: &mut Context<Self>) {
        let mut form = TunnelForm::new(None, self.language, cx);
        form.ssh_hosts = parse_ssh_config(self.config.settings.ssh_config_path.as_deref());
        self.form = Some(form);
        self.notice = None;
        cx.notify();
    }

    pub(super) fn close_create_sheet(&mut self, cx: &mut Context<Self>) {
        self.form = None;
        cx.notify();
    }

    pub(super) fn edit_selected(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_tunnel.clone() else {
            return;
        };
        self.edit_tunnel(id, cx);
    }

    pub(super) fn edit_tunnel(&mut self, id: String, cx: &mut Context<Self>) {
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

    pub(super) fn open_advanced_selected(&mut self, cx: &mut Context<Self>) {
        self.edit_selected(cx);
        if let Some(form) = &mut self.form {
            form.advanced = true;
        }
        cx.notify();
    }

    pub(super) fn delete_selected(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn run_selected_diagnostics(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_tunnel.clone() else {
            return;
        };
        self.run_tunnel_diagnostics(id, cx);
    }

    pub(super) fn run_tunnel_diagnostics(&mut self, id: String, cx: &mut Context<Self>) {
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

    pub(super) fn close_diagnostics(&mut self, cx: &mut Context<Self>) {
        self.diagnostics = None;
        cx.notify();
    }

    pub(super) fn show_about(&mut self, cx: &mut Context<Self>) {
        self.about_open = true;
        cx.notify();
    }

    pub(super) fn close_about(&mut self, cx: &mut Context<Self>) {
        self.about_open = false;
        cx.notify();
    }

    pub(super) fn request_quit(&mut self) {
        let manager = self.manager.clone();
        let sender = self.messages.clone();
        self.runtime.spawn(async move {
            TunnelManager::stop_all(manager).await;
            let _ = sender.send(AppMessage::QuitReady).await;
        });
    }

    pub(super) fn request_close(&mut self, cx: &mut Context<Self>) {
        if self.config.settings.close_to_tray && self._tray.is_some() {
            cx.hide();
        } else {
            self.request_quit();
        }
    }

    pub(super) fn dismiss(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn submit_primary(&mut self, cx: &mut Context<Self>) {
        if self
            .form
            .as_ref()
            .is_some_and(|form| form.ssh_picker_target.is_some() || form.group_menu_open)
        {
            return;
        }
        if self.pending_import.is_some() {
            self.confirm_import_backup(cx);
        } else if self.save_confirmation.is_some() {
            self.confirm_save_and_restart(cx);
        } else if matches!(
            self.auth_prompt,
            Some(AuthPrompt::HostKey {
                issue: HostKeyIssue::Unknown,
                ..
            })
        ) {
            self.trust_prompted_host(cx);
        } else if matches!(self.auth_prompt, Some(AuthPrompt::HostKey { .. })) {
            self.close_auth_prompt(cx);
        } else if matches!(self.auth_prompt, Some(AuthPrompt::Passphrase { .. })) {
            self.submit_passphrase(cx);
        } else if self.diagnostics.is_some() {
            self.diagnostics = None;
            cx.notify();
        } else if self.group_form.is_some() {
            self.save_group(cx);
        } else if self.settings_form.is_some() {
            self.save_settings(cx);
        } else if self.form.is_some() {
            self.save_form(cx);
        }
    }

    pub(super) fn clear_activity(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn clear_selected_logs(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn copy_selected_logs(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn export_selected_logs(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn open_settings(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn cancel_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_form = None;
        cx.notify();
    }

    pub(super) fn save_settings(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn toggle_setting(&mut self, setting: SettingToggle, cx: &mut Context<Self>) {
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

    pub(super) fn set_form_kind(&mut self, kind: ForwardKind, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.kind = kind;
            cx.notify();
        }
    }

    pub(super) fn toggle_form_advanced(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.advanced = !form.advanced;
            cx.notify();
        }
    }

    pub(super) fn toggle_form_reconnect(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.auto_reconnect = !form.auto_reconnect;
            cx.notify();
        }
    }

    pub(super) fn toggle_form_start_with_app(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.start_with_app = !form.start_with_app;
            cx.notify();
        }
    }

    pub(super) fn toggle_jump_host(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.jump_enabled = !form.jump_enabled;
            cx.notify();
        }
    }

    pub(super) fn toggle_form_group_menu(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &mut self.form else { return };
        form.group_menu_open = !form.group_menu_open;
        cx.notify();
    }

    pub(super) fn select_form_group(&mut self, group_id: Option<String>, cx: &mut Context<Self>) {
        let Some(form) = &mut self.form else { return };
        form.group_id = group_id;
        form.group_menu_open = false;
        cx.notify();
    }

    pub(super) fn open_group_form(&mut self, cx: &mut Context<Self>) {
        let name_placeholder = self.language.pick("例如：生产环境", "e.g. Production");
        let description_placeholder = self.language.pick("说明（可选）", "Description (optional)");
        self.group_form = Some(GroupForm {
            editing_id: None,
            name: cx.new(|cx| TextInput::new(cx, name_placeholder, "")),
            description: cx.new(|cx| TextInput::new(cx, description_placeholder, "")),
        });
        cx.notify();
    }

    pub(super) fn edit_current_group(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn close_group_form(&mut self, cx: &mut Context<Self>) {
        self.group_form = None;
        cx.notify();
    }

    pub(super) fn save_group(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn delete_current_group(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn toggle_start_after_save(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.start_after_save = !form.start_after_save;
            cx.notify();
        }
    }

    pub(super) fn open_primary_ssh_hosts(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.ssh_picker_target = Some(SshPickerTarget::Primary);
            cx.notify();
        }
    }

    pub(super) fn open_jump_ssh_hosts(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.ssh_picker_target = Some(SshPickerTarget::JumpHost);
            cx.notify();
        }
    }

    pub(super) fn close_ssh_hosts(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.ssh_picker_target = None;
            cx.notify();
        }
    }

    pub(super) fn dismiss_notice(&mut self, cx: &mut Context<Self>) {
        self.notice = None;
        cx.notify();
    }

    pub(super) fn apply_ssh_host(&mut self, index: usize, cx: &mut Context<Self>) {
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

    pub(super) fn select_private_key(&mut self, cx: &mut Context<Self>) {
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
}
