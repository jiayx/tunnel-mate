use super::super::*;

impl TunnelMateApp {
    pub(crate) fn render_advanced_settings(
        &self,
        form: &TunnelForm,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let switch = |enabled: bool| {
            div()
                .w(px(42.0))
                .h(px(24.0))
                .rounded(px(12.0))
                .p(px(3.0))
                .bg(if enabled { PRIMARY } else { rgb(0x343840) })
                .child(
                    div()
                        .size(px(18.0))
                        .rounded(px(9.0))
                        .bg(TEXT)
                        .when(enabled, |dot| dot.ml(px(18.0))),
                )
        };
        div()
            .flex()
            .flex_col()
            .gap(px(14.0))
            .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .py(px(9.0))
                                                .border_t_1()
                                                .border_color(BORDER)
                                                .text_size(px(12.0))
                                                .text_color(MUTED)
                                                .cursor_pointer()
                                                .on_mouse_up(
                                                    MouseButton::Left,
                                                    cx.listener(|this, _, _, cx| this.toggle_form_advanced(cx)),
                                                )
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(6.0))
                                                        .child(
                                                            Self::disclosure_chevron(
                                                                form.advanced,
                                                                "▾",
                                                                "▸",
                                                            ),
                                                        )
                                                        .child(
                                                            self.language.pick("高级设置", "Advanced"),
                                                        ),
                                                )
                                                .child(if form.advanced {
                                                    self.language.pick("收起", "Collapse")
                                                } else {
                                                    self.language.pick(
                                                        "重连与跳板机",
                                                        "Reconnect & jump host",
                                                    )
                                                }),
                                        )
                                        .when(form.advanced, |panel| {
                                            panel
                                                .child(
                                                    div()
                                                        .flex()
                                                        .gap(px(10.0))
                                                        .child(div().flex_grow(1.0).child(Self::required_form_field(
                                                            self.language.pick("重试次数", "Retry count"),
                                                            form.retry_count.clone(),
                                                        )))
                                                        .child(div().flex_grow(1.0).child(Self::required_form_field(
                                                            self.language.pick(
                                                                "重试间隔（秒）",
                                                                "Retry interval (seconds)",
                                                            ),
                                                            form.retry_interval.clone(),
                                                        ))),
                                                )
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .justify_between()
                                                        .text_size(px(12.0))
                                                        .text_color(TEXT)
                                                        .child(
                                                            self.language.pick(
                                                                "断线后自动重连",
                                                                "Reconnect automatically",
                                                            ),
                                                        )
                                                        .child(
                                                            switch(form.auto_reconnect)
                                                                .cursor_pointer()
                                                                .on_mouse_up(
                                                                    MouseButton::Left,
                                                                    cx.listener(|this, _, _, cx| {
                                                                        this.toggle_form_reconnect(cx)
                                                                    }),
                                                                ),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .justify_between()
                                                        .pt(px(10.0))
                                                        .border_t_1()
                                                        .border_color(BORDER)
                                                        .text_size(px(12.0))
                                                        .text_color(TEXT)
                                                        .child(
                                                            self.language.pick("使用跳板机", "Use jump host"),
                                                        )
                                                        .child(
                                                            switch(form.jump_enabled)
                                                                .cursor_pointer()
                                                                .on_mouse_up(
                                                                    MouseButton::Left,
                                                                    cx.listener(|this, _, _, cx| {
                                                                        this.toggle_jump_host(cx)
                                                                    }),
                                                                ),
                                                        ),
                                                )
                                                .when(form.jump_enabled, |advanced| {
                                                    advanced.child(
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
                                                                                    .font_weight(
                                                                                        FontWeight::MEDIUM,
                                                                                    )
                                                                                    .text_color(TEXT)
                                                                                    .child(self.language.pick(
                                                                                        "跳板机连接",
                                                                                        "Jump host connection",
                                                                                    )),
                                                                            )
                                                                            .child(
                                                                                div()
                                                                                    .text_size(px(10.0))
                                                                                    .text_color(MUTED)
                                                                                    .child(self.language.pick(
                                                                                        "手动填写，或从 SSH config 自动带入下方信息",
                                                                                        "Enter details manually or fill them from SSH config",
                                                                                    )),
                                                                            ),
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .h(px(30.0))
                                                                            .px(px(10.0))
                                                                            .flex()
                                                                            .items_center()
                                                                            .rounded(px(7.0))
                                                                            .border_1()
                                                                            .border_color(glass(
                                                                                0x075bea, 0.45,
                                                                            ))
                                                                            .bg(glass(0x075bea, 0.10))
                                                                            .text_size(px(10.0))
                                                                            .text_color(PRIMARY_TEXT)
                                                                            .cursor_pointer()
                                                                            .hover(|style| {
                                                                                style.bg(glass(
                                                                                    0x075bea, 0.16,
                                                                                ))
                                                                            })
                                                                            .on_mouse_up(
                                                                                MouseButton::Left,
                                                                                cx.listener(
                                                                                    |this, _, _, cx| {
                                                                                        this.open_jump_ssh_hosts(
                                                                                            cx,
                                                                                        )
                                                                                    },
                                                                                ),
                                                                            )
                                                                            .child(self.language.pick(
                                                                                "从 SSH config 选择…",
                                                                                "Choose from SSH config…",
                                                                            )),
                                                                    ),
                                                            )
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .gap(px(10.0))
                                                                    .child(div().flex_grow(1.0).child(
                                                                        Self::required_form_field(
                                                                            self.language.pick("主机", "Host"),
                                                                            form.jump_host.clone(),
                                                                        ),
                                                                    ))
                                                                    .child(div().w(px(100.0)).child(
                                                                        Self::required_form_field(
                                                                            self.language.pick("端口", "Port"),
                                                                            form.jump_port.clone(),
                                                                        ),
                                                                    ))
                                                                    .child(div().w(px(140.0)).child(
                                                                        Self::required_form_field(
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
                                                                    .child(div().flex_grow(1.0).child(
                                                                        Self::form_field(
                                                                            self.language.pick(
                                                                                "私钥文件",
                                                                                "Private key",
                                                                            ),
                                                                            form.jump_identity_file.clone(),
                                                                        ),
                                                                    ))
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
                                                                                cx.listener(
                                                                                    |this, _, _, cx| {
                                                                                        this.select_private_key(
                                                                                            PrivateKeyTarget::JumpHost,
                                                                                            cx,
                                                                                        )
                                                                                    },
                                                                                ),
                                                                            )
                                                                            .child(
                                                                                self.language.pick(
                                                                                    "选择…",
                                                                                    "Choose…",
                                                                                ),
                                                                            ),
                                                                    ),
                                                            )
                                                            .child(Self::form_field(
                                                                self.language.pick("密码", "Password"),
                                                                form.jump_password.clone(),
                                                            )),
                                                    )
                                                })
                                        })
    }
}
