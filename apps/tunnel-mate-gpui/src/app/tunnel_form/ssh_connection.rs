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
            .gap(px(12.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(section_heading(
                        self.language.pick("SSH 服务器", "SSH server"),
                    ))
                    .child(
                        button(
                            "choose-ssh-config",
                            self.language
                                .pick("从 SSH config 选择", "Choose from SSH config"),
                        )
                        .h(px(28.0))
                        .px(px(8.0))
                        .border_color(rgba(0x00000000))
                        .bg(rgba(0x00000000))
                        .text_color(color(0xa8c8ff))
                        .on_click(cx.listener(|this, _, _, cx| this.open_primary_ssh_hosts(cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(div().flex_1().min_w_0().child(Self::required_form_field(
                        self.language.pick("主机", "Host"),
                        form.ssh_host.clone(),
                    )))
                    .child(
                        div()
                            .w(px(84.0))
                            .flex_none()
                            .child(Self::required_form_field(
                                self.language.pick("端口", "Port"),
                                form.ssh_port.clone(),
                            )),
                    )
                    .child(
                        div()
                            .w(px(140.0))
                            .flex_none()
                            .child(Self::required_form_field(
                                self.language.pick("用户", "User"),
                                form.ssh_user.clone(),
                            )),
                    ),
            )
            .child(
                div()
                    .id("authentication-options")
                    .key_context("TunnelButton")
                    .tab_index(0)
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .text_size(px(12.0))
                    .text_color(MUTED)
                    .cursor_pointer()
                    .focus(|style| style.text_color(TEXT))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(form) = &mut this.form {
                            form.authentication_expanded = !form.authentication_expanded;
                        }
                        cx.notify();
                    }))
                    .child(Self::disclosure_chevron(
                        form.authentication_expanded,
                        "▾",
                        "▸",
                    ))
                    .child(self.language.pick("身份验证", "Authentication"))
                    .child(div().flex_1())
                    .child(if form.authentication_expanded {
                        self.language.pick("收起", "Collapse")
                    } else if !form.identity_file.read(cx).value().is_empty()
                        || !form.ssh_password.read(cx).value().is_empty()
                    {
                        self.language
                            .pick("已配置私钥或密码", "Custom key or password")
                    } else {
                        self.language
                            .pick("SSH Agent / 默认私钥", "SSH Agent / default keys")
                    }),
            )
            .when(form.authentication_expanded, |section| {
                section.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        .p(px(14.0))
                        .rounded(px(10.0))
                        .bg(APP_BG)
                        .border_1()
                        .border_color(BORDER_SOFT)
                        .child(div().text_size(px(12.0)).text_color(MUTED).child(
                            self.language.pick(
                                "留空时自动尝试 SSH Agent 和默认私钥",
                                "Leave blank to try SSH Agent and default keys",
                            ),
                        ))
                        .child(
                            div()
                                .flex()
                                .items_end()
                                .gap(px(8.0))
                                .child(div().flex_1().min_w_0().child(Self::form_field(
                                    self.language.pick("私钥文件", "Private key"),
                                    form.identity_file.clone(),
                                )))
                                .child(
                                    button(
                                        "choose-private-key",
                                        self.language.pick("选择…", "Choose…"),
                                    )
                                    .h(px(40.0))
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.select_private_key(PrivateKeyTarget::Primary, cx)
                                        },
                                    )),
                                ),
                        )
                        .child(Self::form_field(
                            self.language.pick("SSH 密码", "SSH password"),
                            form.ssh_password.clone(),
                        )),
                )
            })
    }
}
