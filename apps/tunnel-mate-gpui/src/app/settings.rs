use super::*;

impl TunnelMateApp {
    pub(super) fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.settings_form.as_ref().expect("settings form");
        let row = |label: &'static str,
                   description: &'static str,
                   checked: bool,
                   available: bool,
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
                        .flex_1()
                        .min_w_0()
                        .pr(px(16.0))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(if available { TEXT } else { MUTED })
                                .child(label),
                        )
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
                        .flex_shrink_0()
                        .rounded(px(12.0))
                        .p(px(3.0))
                        .bg(if checked && available {
                            PRIMARY
                        } else {
                            rgb(0x343840)
                        })
                        .when(available, |control| {
                            control.cursor_pointer().on_mouse_up(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| this.toggle_setting(toggle, cx)),
                            )
                        })
                        .child(
                            div()
                                .size(px(18.0))
                                .rounded(px(9.0))
                                .bg(if available { TEXT } else { MUTED })
                                .when(checked, |dot| dot.ml(px(18.0))),
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
                                self.language.pick("登录时启动", "Launch at login"),
                                self.language.pick(
                                    "登录系统后自动运行 Tunnel Mate",
                                    "Run Tunnel Mate after signing in",
                                ),
                                form.launch_on_startup,
                                true,
                                SettingToggle::Launch,
                            ))
                            .child(row(
                                self.language
                                    .pick("登录时在后台启动", "Start in background at login"),
                                if form.launch_on_startup {
                                    start_in_background_description(self.language)
                                } else {
                                    self.language.pick(
                                        "请先开启“登录时启动”",
                                        "Turn on “Launch at login” first",
                                    )
                                },
                                form.start_minimized,
                                form.launch_on_startup,
                                SettingToggle::Minimized,
                            ))
                            .child(row(
                                self.language.pick(
                                    "关闭窗口后继续运行",
                                    "Keep running after closing the window",
                                ),
                                keep_running_after_close_description(self.language),
                                form.close_to_tray,
                                true,
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
