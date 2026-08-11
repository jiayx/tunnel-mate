use super::super::*;

impl TunnelMateApp {
    pub(crate) fn set_form_kind(&mut self, kind: ForwardKind, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.kind = kind;
            cx.notify();
        }
    }

    pub(crate) fn toggle_form_advanced(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.advanced = !form.advanced;
            cx.notify();
        }
    }

    pub(crate) fn toggle_form_reconnect(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.auto_reconnect = !form.auto_reconnect;
            cx.notify();
        }
    }

    pub(crate) fn toggle_form_start_with_app(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.start_with_app = !form.start_with_app;
            cx.notify();
        }
    }

    pub(crate) fn toggle_jump_host(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.jump_enabled = !form.jump_enabled;
            cx.notify();
        }
    }

    pub(crate) fn toggle_form_group_menu(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &mut self.form else { return };
        form.group_menu_open = !form.group_menu_open;
        cx.notify();
    }

    pub(crate) fn close_form_group_menu(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &mut self.form else { return };
        if form.group_menu_open {
            form.group_menu_open = false;
            cx.notify();
        }
    }

    pub(crate) fn select_form_group(&mut self, group_id: Option<String>, cx: &mut Context<Self>) {
        let Some(form) = &mut self.form else { return };
        form.group_id = group_id;
        form.group_menu_open = false;
        cx.notify();
    }

    pub(crate) fn open_group_form(&mut self, cx: &mut Context<Self>) {
        let name_placeholder = self.language.pick("例如：生产环境", "e.g. Production");
        let description_placeholder = self.language.pick("说明（可选）", "Description (optional)");
        self.group_form = Some(GroupForm {
            editing_id: None,
            name: cx.new(|cx| TextInput::new(cx, name_placeholder, "")),
            description: cx.new(|cx| TextInput::new(cx, description_placeholder, "")),
        });
        cx.notify();
    }

    pub(crate) fn edit_current_group(&mut self, cx: &mut Context<Self>) {
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

    pub(crate) fn close_group_form(&mut self, cx: &mut Context<Self>) {
        self.group_form = None;
        cx.notify();
    }

    pub(crate) fn save_group(&mut self, cx: &mut Context<Self>) {
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

    pub(crate) fn delete_current_group(&mut self, cx: &mut Context<Self>) {
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

    pub(crate) fn toggle_start_after_save(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.start_after_save = !form.start_after_save;
            cx.notify();
        }
    }

    pub(crate) fn open_primary_ssh_hosts(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.ssh_picker_target = Some(SshPickerTarget::Primary);
            cx.notify();
        }
    }

    pub(crate) fn open_jump_ssh_hosts(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.ssh_picker_target = Some(SshPickerTarget::JumpHost);
            cx.notify();
        }
    }

    pub(crate) fn close_ssh_hosts(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            form.ssh_picker_target = None;
            cx.notify();
        }
    }

    pub(crate) fn dismiss_notice(&mut self, cx: &mut Context<Self>) {
        self.notice = None;
        cx.notify();
    }

    pub(crate) fn apply_ssh_host(&mut self, index: usize, cx: &mut Context<Self>) {
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

    pub(crate) fn select_private_key(&mut self, target: PrivateKeyTarget, cx: &mut Context<Self>) {
        let mut dialog = rfd::AsyncFileDialog::new().set_title(
            self.language
                .pick("选择 SSH 私钥", "Choose an SSH private key"),
        );
        if let Some(directory) = default_ssh_directory() {
            dialog = dialog.set_directory(directory);
        }
        let sender = self.messages.clone();
        cx.spawn(async move |_, _| {
            if let Some(file) = dialog.pick_file().await {
                let _ = sender
                    .send(AppMessage::PrivateKeySelected {
                        target,
                        path: file.path().to_string_lossy().into_owned(),
                    })
                    .await;
            }
        })
        .detach();
    }
}

fn default_ssh_directory() -> Option<std::path::PathBuf> {
    // dirs::home_dir maps to USERPROFILE on Windows and the user's home
    // directory on macOS/Linux. Open ~/.ssh when present, otherwise fall back
    // to the home directory so every platform receives a valid initial path.
    let home = dirs::home_dir()?;
    let ssh = home.join(".ssh");
    Some(if ssh.is_dir() { ssh } else { home })
}
