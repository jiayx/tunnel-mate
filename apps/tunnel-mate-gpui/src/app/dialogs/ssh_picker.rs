use super::super::*;

impl TunnelMateApp {
    pub(crate) fn render_notice(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

    pub(crate) fn render_ssh_host_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
}
