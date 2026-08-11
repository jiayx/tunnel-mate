use super::*;

impl TunnelMateApp {
    pub(super) fn render_notice(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

    pub(super) fn render_ssh_host_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

    pub(super) fn render_diagnostics(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

    pub(super) fn render_save_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

    pub(super) fn render_delete_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let id = self
            .delete_confirmation
            .as_ref()
            .expect("delete confirmation");
        let tunnel = self
            .config
            .tunnels
            .iter()
            .find(|tunnel| &tunnel.id == id)
            .expect("tunnel being deleted");
        let running = self.is_active(id);
        let message = match (running, self.language) {
            (true, Language::Zh) => format!(
                "“{}”当前正在运行。删除后会立即断开连接，并且无法恢复。",
                tunnel.name
            ),
            (true, Language::En) => format!(
                "“{}” is running. Deleting it will disconnect immediately and cannot be undone.",
                tunnel.name
            ),
            (false, Language::Zh) => {
                format!("确定删除“{}”吗？此操作无法恢复。", tunnel.name)
            }
            (false, Language::En) => {
                format!("Delete “{}”? This action cannot be undone.", tunnel.name)
            }
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
                            .child(self.language.pick("删除隧道？", "Delete tunnel?")),
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
                                            this.cancel_delete_confirmation(cx)
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
                                    .border_1()
                                    .border_color(DANGER)
                                    .bg(glass(0xdc747c, 0.16))
                                    .text_size(px(12.0))
                                    .text_color(DANGER)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(glass(0xdc747c, 0.24)))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.confirm_delete_tunnel(cx)
                                        }),
                                    )
                                    .child(if running {
                                        self.language.pick("停止并删除", "Stop and delete")
                                    } else {
                                        self.language.pick("删除", "Delete")
                                    }),
                            ),
                    ),
            )
    }

    pub(super) fn render_import_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

    pub(super) fn render_group_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                            .child(Self::required_form_field(
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

    pub(super) fn render_auth_prompt(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (title, body) = match self.auth_prompt.as_ref().expect("auth prompt") {
            AuthPrompt::HostKey {
                issue,
                host,
                port,
                fingerprint,
                ..
            } => (
                match issue {
                    HostKeyIssue::Unknown => self.language.pick("信任 SSH 主机密钥", "Trust SSH host key"),
                    HostKeyIssue::Changed => self.language.pick("SSH 主机密钥已变化", "SSH host key changed"),
                    HostKeyIssue::Revoked => self.language.pick("SSH 主机密钥已撤销", "SSH host key revoked"),
                },
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(MUTED)
                            .child(match (issue, self.language) {
                                (HostKeyIssue::Unknown, Language::Zh) => format!("首次连接 {host}:{port}，请核对指纹后再信任。"),
                                (HostKeyIssue::Unknown, Language::En) => format!("First connection to {host}:{port}. Verify the fingerprint before trusting it."),
                                (HostKeyIssue::Changed, Language::Zh) => format!("{host}:{port} 的主机密钥与已保存记录不一致。为防止中间人攻击，应用不会自动覆盖；确认服务器确实更换密钥后，请先运行 ssh-keygen -R {host}。"),
                                (HostKeyIssue::Changed, Language::En) => format!("The key for {host}:{port} differs from the saved key. It will not be overwritten automatically. After independently verifying the change, run ssh-keygen -R {host}."),
                                (HostKeyIssue::Revoked, Language::Zh) => format!("{host}:{port} 的密钥在 known_hosts 中被标记为已撤销。请联系服务器管理员核实，应用已阻止连接。"),
                                (HostKeyIssue::Revoked, Language::En) => format!("The key for {host}:{port} is marked as revoked in known_hosts. Contact the server administrator; the connection was blocked."),
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
                            .flex_wrap()
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
                                        cx.listener(|this, _, _, cx| this.copy_prompted_fingerprint(cx)),
                                    )
                                    .child(self.language.pick("复制指纹", "Copy fingerprint")),
                            )
                            .when(*issue == HostKeyIssue::Changed, |row| row.child(
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
                                        cx.listener(|this, _, _, cx| this.copy_known_host_cleanup(cx)),
                                    )
                                    .child(self.language.pick("复制清理命令", "Copy cleanup command")),
                            ))
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
                                    .child(if *issue == HostKeyIssue::Unknown {
                                        self.language.pick("取消", "Cancel")
                                    } else {
                                        self.language.pick("关闭", "Close")
                                    }),
                            )
                            .when(*issue == HostKeyIssue::Unknown, |row| row
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
                                    .child(self.language.pick("信任并连接", "Trust and connect")),
                            )),
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

    pub(super) fn render_about(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
}
