use super::super::*;

impl TunnelMateApp {
    pub(crate) fn render_ssh_connection(
        &self,
        form: &TunnelForm,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
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
                                    .child(self.language.pick("SSH 连接", "SSH connection")),
                            )
                            .child(div().text_size(px(10.0)).text_color(MUTED).child(
                                self.language.pick(
                                    "手动填写，或从 SSH config 自动带入下方信息",
                                    "Enter details manually or fill them from SSH config",
                                ),
                            )),
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
                            .hover(|style| style.bg(glass(0x075bea, 0.16)))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.open_primary_ssh_hosts(cx)),
                            )
                            .child(
                                self.language
                                    .pick("从 SSH config 选择…", "Choose from SSH config…"),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(10.0))
                    .child(div().flex_grow(1.0).child(Self::required_form_field(
                        self.language.pick("主机", "Host"),
                        form.ssh_host.clone(),
                    )))
                    .child(div().w(px(100.0)).child(Self::required_form_field(
                        self.language.pick("端口", "Port"),
                        form.ssh_port.clone(),
                    )))
                    .child(div().w(px(150.0)).child(Self::required_form_field(
                        self.language.pick("用户", "User"),
                        form.ssh_user.clone(),
                    ))),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(3.0))
                    .pt(px(10.0))
                    .border_t_1()
                    .border_color(BORDER)
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(TEXT)
                            .child(
                                self.language
                                    .pick("身份验证（可选）", "Authentication (optional)"),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(MUTED)
                            .child(self.language.pick(
                                "留空时依次尝试 SSH Agent 和默认私钥",
                                "Leave blank to try SSH Agent and default keys",
                            )),
                    ),
            )
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
                                    this.select_private_key(PrivateKeyTarget::Primary, cx)
                                }),
                            )
                            .child(self.language.pick("选择…", "Choose…")),
                    ),
            )
            .child(Self::form_field(
                self.language.pick("SSH 密码", "SSH password"),
                form.ssh_password.clone(),
            ))
    }
}
