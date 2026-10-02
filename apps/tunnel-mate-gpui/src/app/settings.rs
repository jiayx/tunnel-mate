use super::*;

impl TunnelMateApp {
    pub(super) fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let form = self.settings_form.as_ref().expect("settings form");
        let row = |id: &'static str,
                   label: &'static str,
                   description: &'static str,
                   checked: bool,
                   available: bool,
                   setting: SettingToggle| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(18.0))
                .py(px(15.0))
                .border_b_1()
                .border_color(theme.border_soft)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(5.0))
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(if available { theme.text } else { theme.muted })
                                .child(label),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.muted)
                                .whitespace_normal()
                                .child(description),
                        ),
                )
                .child(
                    toggle(theme, id, checked)
                        .when(!available, |control| {
                            control.tab_index(-1).opacity(0.45).cursor_default()
                        })
                        .when(available, |control| {
                            control.on_click(
                                cx.listener(move |this, _, _, cx| this.toggle_setting(setting, cx)),
                            )
                        }),
                )
        };
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.backdrop)
            .child(
                div()
                    .w(px(580.0))
                    .h(px(600.0))
                    .max_w(relative(0.94))
                    .max_h(relative(0.92))
                    .flex()
                    .flex_col()
                    .rounded(px(16.0))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.surface)
                    .shadow_lg()
                    .child(
                        div()
                            .h(px(68.0))
                            .flex_none()
                            .px(px(22.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(theme.border_soft)
                            .child(
                                div()
                                    .text_size(px(19.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(self.language.pick("设置", "Settings")),
                            )
                            .child(
                                close_button(theme, "close-settings").on_click(
                                    cx.listener(|this, _, _, cx| this.cancel_settings(cx)),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_h(px(0.0))
                            .child(
                                div()
                                    .id("settings-scroll")
                                    .track_scroll(&form.scroll)
                                    .overflow_y_scroll()
                                    .size_full()
                                    .min_h(px(0.0))
                                    .px(px(22.0))
                                    .pb(px(22.0))
                                    .child(row(
                                        "launch-on-startup",
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
                                        "start-minimized",
                                        self.language.pick(
                                            "登录时在后台启动",
                                            "Start in background at login",
                                        ),
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
                                        "close-to-tray",
                                        close_to_tray_title(self.language),
                                        keep_running_after_close_description(self.language),
                                        form.close_to_tray,
                                        true,
                                        SettingToggle::CloseToTray,
                                    ))
                                    .child(
                                        div()
                                            .mt(px(20.0))
                                            .flex()
                                            .gap(px(14.0))
                                            .child(div().flex_1().child(
                                                Self::required_form_field(
                                                    theme,
                                                    self.language.pick(
                                                        "保活间隔（秒）",
                                                        "Keep-alive (seconds)",
                                                    ),
                                                    form.keep_alive.clone(),
                                                ),
                                            ))
                                            .child(div().flex_1().child(
                                                Self::required_form_field(
                                                    theme,
                                                    self.language.pick(
                                                        "连接超时（秒）",
                                                        "Connection timeout (seconds)",
                                                    ),
                                                    form.connect_timeout.clone(),
                                                ),
                                            )),
                                    )
                                    .child(div().mt(px(16.0)).child(Self::form_field(
                                        theme,
                                        self.language.pick("SSH config 路径", "SSH config path"),
                                        form.ssh_config_path.clone(),
                                    )))
                                    .when_some(form.validation_error.clone(), |body, error| {
                                        body.child(
                                            div()
                                                .mt(px(12.0))
                                                .text_size(px(12.0))
                                                .text_color(theme.danger)
                                                .whitespace_normal()
                                                .child(error),
                                        )
                                    }),
                            )
                            .child(crate::scrollbar::scrollbar(
                                "settings-scrollbar",
                                theme,
                                form.scroll.clone(),
                            )),
                    )
                    .child(
                        div()
                            .h(px(68.0))
                            .flex_none()
                            .px(px(22.0))
                            .border_t_1()
                            .border_color(theme.border_soft)
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                button(
                                    theme,
                                    "export-backup",
                                    self.language.pick("导出备份", "Export backup"),
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.export_backup(cx))),
                            )
                            .child(
                                button(
                                    theme,
                                    "import-backup",
                                    self.language.pick("导入备份", "Import backup"),
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.import_backup(cx))),
                            )
                            .child(div().flex_1())
                            .child(
                                primary_button(
                                    theme,
                                    "save-settings",
                                    self.language.pick("保存", "Save"),
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.save_settings(cx))),
                            ),
                    ),
            )
    }
}
