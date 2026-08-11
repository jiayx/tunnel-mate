use super::*;

impl TunnelMateApp {
    pub(super) fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.settings_form.as_ref().expect("settings form");
        let row = |label: &'static str,
                   description: &'static str,
                   enabled: bool,
                   toggle: SettingToggle| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .py(px(13.0))
                .border_b_1()
                .border_color(BORDER)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(div().text_size(px(12.0)).text_color(TEXT).child(label))
                        .child(
                            div()
                                .mt(px(3.0))
                                .text_size(px(10.0))
                                .text_color(MUTED)
                                .child(description),
                        ),
                )
                .child(
                    div()
                        .w(px(42.0))
                        .h(px(24.0))
                        .rounded(px(12.0))
                        .p(px(3.0))
                        .bg(if enabled { PRIMARY } else { rgb(0x343840) })
                        .cursor_pointer()
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| this.toggle_setting(toggle, cx)),
                        )
                        .child(
                            div()
                                .size(px(18.0))
                                .rounded(px(9.0))
                                .bg(TEXT)
                                .when(enabled, |dot| dot.ml(px(18.0))),
                        ),
                )
        };
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0db8))
            .child(
                div()
                    .w(px(520.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(SURFACE)
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
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(self.language.pick("设置", "Settings")),
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
                                        cx.listener(|this, _, _, cx| this.cancel_settings(cx)),
                                    )
                                    .child("×"),
                            ),
                    )
                    .child(
                        div()
                            .px(px(18.0))
                            .pb(px(18.0))
                            .child(row(
                                self.language.pick("开机启动", "Launch at login"),
                                self.language.pick(
                                    "登录系统后自动运行 Tunnel Mate",
                                    "Run Tunnel Mate after signing in",
                                ),
                                form.launch_on_startup,
                                SettingToggle::Launch,
                            ))
                            .child(row(
                                self.language.pick(
                                    "登录启动时隐藏窗口",
                                    "Hide window at login",
                                ),
                                self.language.pick(
                                    "仅影响登录系统后的自动启动，手动打开始终显示窗口",
                                    "Only affects login launch; manual launch always shows the window",
                                ),
                                form.start_minimized,
                                SettingToggle::Minimized,
                            ))
                            .child(row(
                                self.language.pick("关闭到托盘", "Close to tray"),
                                close_to_tray_description(self.language),
                                form.close_to_tray,
                                SettingToggle::CloseToTray,
                            ))
                            .child(
                                div()
                                    .mt(px(14.0))
                                    .flex()
                                    .gap(px(10.0))
                                    .child(
                                        div().flex_grow(1.0).child(Self::required_form_field(
                                            self.language
                                                .pick("保活间隔（秒）", "Keep-alive (seconds)"),
                                            form.keep_alive.clone(),
                                        )),
                                    )
                                    .child(div().flex_grow(1.0).child(
                                        Self::required_form_field(
                                            self.language.pick(
                                                "连接超时（秒）",
                                                "Connection timeout (seconds)",
                                            ),
                                            form.connect_timeout.clone(),
                                        ),
                                    )),
                            )
                            .child(div().mt(px(12.0)).child(Self::form_field(
                                self.language.pick("SSH config 路径", "SSH config path"),
                                form.ssh_config_path.clone(),
                            )))
                            .child(
                                div()
                                    .mt(px(16.0))
                                    .flex()
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
                                            .text_color(TEXT)
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    this.export_backup(cx)
                                                }),
                                            )
                                            .child(self.language.pick("导出备份", "Export backup")),
                                    )
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
                                            .text_color(TEXT)
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    this.import_backup(cx)
                                                }),
                                            )
                                            .child(self.language.pick("导入备份", "Import backup")),
                                    )
                                    .child(div().flex_grow(1.0))
                                    .child(
                                        div()
                                            .h(px(34.0))
                                            .px(px(13.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(8.0))
                                            .bg(PRIMARY)
                                            .text_size(px(11.0))
                                            .text_color(PRIMARY_TEXT)
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    this.save_settings(cx)
                                                }),
                                            )
                                            .child(self.language.pick("保存", "Save")),
                                    ),
                            ),
                    ),
            )
    }
}
