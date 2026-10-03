use super::*;

#[path = "tunnel_form/advanced_settings.rs"]
mod advanced_settings;
#[path = "tunnel_form/group_selector.rs"]
mod group_selector;
#[path = "tunnel_form/ssh_connection.rs"]
mod ssh_connection;

impl TunnelMateApp {
    pub(super) fn render_create_sheet(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let form = self.form.as_ref().expect("open form");
        let editing = form.editing_id.is_some();
        let kind_button = |label: &'static str, kind: ForwardKind, id: &'static str| {
            let selected = form.kind == kind;
            segment_button(theme, id, label, selected)
                .flex_1()
                .h(px(36.0))
                .on_click(cx.listener(move |this, _, _, cx| this.set_form_kind(kind, cx)))
        };
        let (description, listen_label, target_label) = match form.kind {
            ForwardKind::Local => (
                self.language.pick(
                    "本机端口 → SSH 服务器 → 远端服务",
                    "Local port → SSH server → Remote service",
                ),
                self.language.pick("本机监听地址", "Local listen address"),
                self.language.pick("远端目标地址", "Remote target address"),
            ),
            ForwardKind::Remote => (
                self.language.pick(
                    "远端端口 → SSH 隧道 → 本机服务",
                    "Remote port → SSH tunnel → Local service",
                ),
                self.language.pick("远端监听地址", "Remote listen address"),
                self.language.pick("本机目标地址", "Local target address"),
            ),
            ForwardKind::Socks5 => (
                self.language.pick(
                    "应用 → 本机 SOCKS5 代理 → SSH 服务器",
                    "App → Local SOCKS5 proxy → SSH server",
                ),
                self.language
                    .pick("SOCKS5 监听地址", "SOCKS5 listen address"),
                "",
            ),
        };
        let endpoint = |label: &'static str, host: Entity<TextInput>, port: Entity<TextInput>| {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .gap(px(8.0))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(Self::required_form_field(theme, label, host)),
                )
                .child(
                    div()
                        .w(px(82.0))
                        .flex_none()
                        .child(Self::required_form_field(
                            theme,
                            self.language.pick("端口", "Port"),
                            port,
                        )),
                )
        };
        let body = div()
            .id("tunnel-form-scroll")
            .track_scroll(&form.scroll)
            .overflow_y_scroll()
            .size_full()
            .min_h(px(0.0))
            .p(px(22.0))
            .flex()
            .flex_col()
            .gap(px(20.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .child(
                        div()
                            .flex()
                            .gap(px(4.0))
                            .p(px(4.0))
                            .rounded(px(11.0))
                            .bg(theme.app_bg)
                            .child(kind_button(
                                self.language.pick("本地转发", "Local forwarding"),
                                ForwardKind::Local,
                                "kind-local",
                            ))
                            .child(kind_button(
                                self.language.pick("远程转发", "Remote forwarding"),
                                ForwardKind::Remote,
                                "kind-remote",
                            ))
                            .child(kind_button("SOCKS5", ForwardKind::Socks5, "kind-socks")),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.muted)
                            .child(description),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(14.0))
                    .items_end()
                    .child(div().flex_1().min_w_0().child(Self::required_form_field(
                        theme,
                        self.language.pick("名称", "Name"),
                        form.name.clone(),
                    )))
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(section_heading(theme, self.language.pick("分组", "Group")))
                            .child(self.render_group_dropdown(form, cx)),
                    ),
            )
            .child(self.render_ssh_connection(form, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .pt(px(16.0))
                    .border_t_1()
                    .border_color(theme.border_soft)
                    .child(section_heading(
                        theme,
                        self.language.pick("转发端口", "Port forwarding"),
                    ))
                    .child(
                        div()
                            .flex()
                            .gap(px(16.0))
                            .child(endpoint(
                                listen_label,
                                form.listen_host.clone(),
                                form.listen_port.clone(),
                            ))
                            .when(form.kind != ForwardKind::Socks5, |row| {
                                row.child(endpoint(
                                    target_label,
                                    form.target_host.clone(),
                                    form.target_port.clone(),
                                ))
                            }),
                    ),
            )
            .child(self.render_advanced_settings(form, cx));
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.backdrop)
            .child(
                div()
                    .w(px(720.0))
                    .max_w(relative(0.94))
                    .h(relative(0.92))
                    .max_h(px(740.0))
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
                                    .child(if editing {
                                        self.language.pick("编辑隧道", "Edit tunnel")
                                    } else {
                                        self.language.pick("新建隧道", "New tunnel")
                                    }),
                            )
                            .child(close_button(theme, "close-tunnel-form").on_click(
                                cx.listener(|this, _, _, cx| this.close_create_sheet(cx)),
                            )),
                    )
                    .when_some(form.validation_error.clone(), |panel, error| {
                        panel.child(
                            div()
                                .flex_none()
                                .mx(px(22.0))
                                .mt(px(12.0))
                                .p(px(12.0))
                                .rounded(px(8.0))
                                .border_1()
                                .border_color(theme.danger)
                                .bg(theme.danger_bg)
                                .text_size(px(12.0))
                                .text_color(theme.text)
                                .whitespace_normal()
                                .child(error),
                        )
                    })
                    .child(div().relative().flex_1().min_h(px(0.0)).child(body).child(
                        crate::scrollbar::scrollbar(
                            "tunnel-form-scrollbar",
                            theme,
                            form.scroll.clone(),
                        ),
                    ))
                    .child(
                        div()
                            .flex_none()
                            .h(px(66.0))
                            .px(px(22.0))
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .border_t_1()
                            .border_color(theme.border_soft)
                            .when(editing, |footer| {
                                footer.child(
                                    button(
                                        theme,
                                        "delete-tunnel",
                                        self.language.pick("删除隧道", "Delete tunnel"),
                                    )
                                    .text_color(theme.danger)
                                    .border_color(rgba(0x00000000))
                                    .on_click(cx.listener(
                                        |this, _, _, cx| this.request_delete_from_form(cx),
                                    )),
                                )
                            })
                            .child(div().flex_1())
                            .child(
                                button(
                                    theme,
                                    "cancel-tunnel",
                                    self.language.pick("取消", "Cancel"),
                                )
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.close_create_sheet(cx)),
                                ),
                            )
                            .child(
                                button(
                                    theme,
                                    "start-after-save",
                                    self.language.pick("保存后连接", "Connect after save"),
                                )
                                .border_color(rgba(0x00000000))
                                .px(px(4.0))
                                .child(
                                    div()
                                        .size(px(18.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(5.0))
                                        .border_1()
                                        .border_color(if form.start_after_save {
                                            theme.primary
                                        } else {
                                            theme.border
                                        })
                                        .bg(if form.start_after_save {
                                            theme.primary
                                        } else {
                                            theme.app_bg
                                        })
                                        .when(form.start_after_save, |check| {
                                            check.child(icon(theme, "icons/check").size(px(13.0)))
                                        }),
                                )
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.toggle_start_after_save(cx)),
                                ),
                            )
                            .child(
                                primary_button(
                                    theme,
                                    "save-tunnel",
                                    if form.start_after_save {
                                        self.language.pick("保存并连接", "Save & connect")
                                    } else {
                                        self.language.pick("保存", "Save")
                                    },
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.save_form(cx))),
                            ),
                    ),
            )
            .when(form.group_menu_open, |backdrop| {
                backdrop.child(
                    div()
                        .absolute()
                        .inset_0()
                        .occlude()
                        .on_scroll_wheel(stop_scroll_propagation)
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                cx.stop_propagation();
                                this.close_form_group_menu(cx);
                            }),
                        ),
                )
            })
    }
}
