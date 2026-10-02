use super::super::*;

impl TunnelMateApp {
    pub(crate) fn render_advanced_settings(
        &self,
        form: &TunnelForm,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let theme = self.theme;
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .pt(px(12.0))
            .border_t_1()
            .border_color(theme.border_soft)
            .child(
                button(
                    theme,
                    "advanced-settings",
                    self.language.pick("高级设置", "Advanced settings"),
                )
                .w_full()
                .justify_between()
                .h(px(40.0))
                .px(px(0.0))
                .border_color(rgba(0x00000000))
                .bg(rgba(0x00000000))
                .child(div().text_color(theme.muted).child(if form.advanced {
                    self.language.pick("收起 ↑", "Collapse ↑")
                } else {
                    self.language
                        .pick("启动、重连与跳板机 ↓", "Startup, reconnect & jump host ↓")
                }))
                .on_click(cx.listener(|this, _, _, cx| this.toggle_form_advanced(cx))),
            )
            .when(form.advanced, |panel| {
                panel
                    .child(Self::form_field(
                        theme,
                        self.language.pick("说明（可选）", "Description (optional)"),
                        form.description.clone(),
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(13.0))
                            .child(
                                self.language
                                    .pick("应用启动时连接", "Connect when app starts"),
                            )
                            .child(
                                toggle(theme, "connect-on-startup", form.start_with_app).on_click(
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
                            .text_size(px(13.0))
                            .child(
                                self.language
                                    .pick("断线后自动重连", "Reconnect automatically"),
                            )
                            .child(
                                toggle(theme, "auto-reconnect", form.auto_reconnect).on_click(
                                    cx.listener(|this, _, _, cx| this.toggle_form_reconnect(cx)),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(14.0))
                            .child(div().flex_1().child(Self::required_form_field(
                                theme,
                                self.language.pick("重试次数", "Retry count"),
                                form.retry_count.clone(),
                            )))
                            .child(
                                div().flex_1().child(Self::required_form_field(
                                    theme,
                                    self.language
                                        .pick("重试间隔（秒）", "Retry interval (seconds)"),
                                    form.retry_interval.clone(),
                                )),
                            ),
                    )
                    .child(
                        div()
                            .pt(px(16.0))
                            .border_t_1()
                            .border_color(theme.border_soft)
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(13.0))
                            .child(self.language.pick("使用跳板机", "Use jump host"))
                            .child(
                                toggle(theme, "use-jump-host", form.jump_enabled).on_click(
                                    cx.listener(|this, _, _, cx| this.toggle_jump_host(cx)),
                                ),
                            ),
                    )
                    .when(form.jump_enabled, |panel| {
                        panel.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(14.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(section_heading(
                                            theme,
                                            self.language
                                                .pick("跳板机连接", "Jump host connection"),
                                        ))
                                        .child(
                                            button(
                                                theme,
                                                "choose-jump-ssh",
                                                self.language.pick(
                                                    "从 SSH config 选择…",
                                                    "Choose from SSH config…",
                                                ),
                                            )
                                            .h(px(30.0))
                                            .on_click(
                                                cx.listener(|this, _, _, cx| {
                                                    this.open_jump_ssh_hosts(cx)
                                                }),
                                            ),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap(px(10.0))
                                        .child(div().flex_1().min_w_0().child(
                                            Self::required_form_field(
                                                theme,
                                                self.language.pick("主机", "Host"),
                                                form.jump_host.clone(),
                                            ),
                                        ))
                                        .child(div().w(px(84.0)).child(Self::required_form_field(
                                            theme,
                                            self.language.pick("端口", "Port"),
                                            form.jump_port.clone(),
                                        )))
                                        .child(div().w(px(140.0)).child(
                                            Self::required_form_field(
                                                theme,
                                                self.language.pick("用户", "User"),
                                                form.jump_user.clone(),
                                            ),
                                        )),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_end()
                                        .gap(px(8.0))
                                        .child(div().flex_1().min_w_0().child(
                                            Self::form_field(
                                                theme,
                                                self.language.pick(
                                                    "私钥文件（可选）",
                                                    "Private key (optional)",
                                                ),
                                                form.jump_identity_file.clone(),
                                            ),
                                        ))
                                        .child(
                                            button(
                                                theme,
                                                "choose-jump-key",
                                                self.language.pick("选择…", "Choose…"),
                                            )
                                            .h(px(40.0))
                                            .on_click(
                                                cx.listener(|this, _, _, cx| {
                                                    this.select_private_key(
                                                        PrivateKeyTarget::JumpHost,
                                                        cx,
                                                    )
                                                }),
                                            ),
                                        ),
                                )
                                .child(Self::form_field(
                                    theme,
                                    self.language.pick("密码（可选）", "Password (optional)"),
                                    form.jump_password.clone(),
                                )),
                        )
                    })
            })
    }
}
