use super::*;

#[path = "tunnel_form/advanced_settings.rs"]
mod advanced_settings;
#[path = "tunnel_form/group_selector.rs"]
mod group_selector;
#[path = "tunnel_form/ssh_connection.rs"]
mod ssh_connection;

impl TunnelMateApp {
    pub(super) fn render_create_sheet(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.form.as_ref().expect("open form");
        let is_editing = form.editing_id.is_some();
        let group_dropdown = self.render_group_dropdown(form, cx);
        let ssh_connection = self.render_ssh_connection(form, cx);
        let advanced_settings = self.render_advanced_settings(form, cx);
        let (
            forward_title,
            forward_description,
            forward_route,
            listen_address_label,
            listen_port_label,
            target_address_label,
            target_port_label,
        ) = match form.kind {
            ForwardKind::Local => (
                self.language.pick(
                    "从本机访问远端服务",
                    "Access a remote service locally",
                ),
                self.language.pick(
                    "应用连接本机监听端口，流量通过 SSH 隧道转发到远端目标。",
                    "Apps connect to a local port; traffic travels through SSH to the remote target.",
                ),
                self.language.pick(
                    "本机监听  →  SSH  →  远端目标",
                    "Local listen  →  SSH  →  Remote target",
                ),
                self.language.pick("本机监听地址", "Local listen address"),
                self.language.pick("本机端口", "Local port"),
                self.language.pick("远端目标地址", "Remote target address"),
                self.language.pick("远端目标端口", "Remote target port"),
            ),
            ForwardKind::Remote => (
                self.language.pick(
                    "让远端访问本机服务",
                    "Expose a local service remotely",
                ),
                self.language.pick(
                    "SSH 服务器监听远端端口，收到的流量通过隧道转发到本机目标。",
                    "The SSH server listens remotely and forwards incoming traffic to a local target.",
                ),
                self.language.pick(
                    "远端监听  →  SSH  →  本机目标",
                    "Remote listen  →  SSH  →  Local target",
                ),
                self.language.pick("远端监听地址", "Remote listen address"),
                self.language.pick("远端端口", "Remote port"),
                self.language.pick("本机目标地址", "Local target address"),
                self.language.pick("本机目标端口", "Local target port"),
            ),
            ForwardKind::Socks5 => (
                self.language
                    .pick("创建本机 SOCKS5 代理", "Create a local SOCKS5 proxy"),
                self.language.pick(
                    "应用使用本机 SOCKS5 端口，目标地址由应用决定并通过 SSH 访问。",
                    "Apps use a local SOCKS5 port and choose destinations that are reached through SSH.",
                ),
                self.language.pick(
                    "应用  →  本机 SOCKS5  →  SSH  →  目标地址",
                    "App  →  Local SOCKS5  →  SSH  →  Destination",
                ),
                self.language.pick("SOCKS5 监听地址", "SOCKS5 listen address"),
                self.language.pick("SOCKS5 端口", "SOCKS5 port"),
                self.language.pick("目标地址", "Target address"),
                self.language.pick("目标端口", "Target port"),
            ),
        };
        let kind_button = |label: &'static str, kind: ForwardKind| {
            let selected = form.kind == kind;
            div()
                .flex_grow(1.0)
                .h(px(34.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.0))
                .bg(if selected { PRIMARY } else { APP_BG })
                .text_color(if selected { TEXT } else { MUTED })
                .text_size(px(11.0))
                .cursor_pointer()
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| this.set_form_kind(kind, cx)),
                )
                .child(label)
        };
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

        let heading = if form.editing_id.is_some() {
            self.language.pick("编辑隧道", "Edit tunnel")
        } else {
            self.language.pick("新建隧道", "New tunnel")
        };
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0db8))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(620.0))
                    .max_h(px(540.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(SURFACE)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .h(px(58.0))
                            .px(px(18.0))
                            .border_b_1()
                            .border_color(BORDER)
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(TEXT)
                                    .child(heading),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size(px(30.0))
                                    .rounded(px(7.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(SURFACE_HOVER))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.close_create_sheet(cx)),
                                    )
                                    .child("×"),
                            ),
                    )
                    .when_some(form.validation_error.clone(), |sheet, error| {
                        sheet.child(
                            div()
                                .mx(px(18.0))
                                .mt(px(12.0))
                                .px(px(12.0))
                                .py(px(9.0))
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .rounded(px(8.0))
                                .border_1()
                                .border_color(glass(0xdc747c, 0.55))
                                .bg(glass(0xdc747c, 0.10))
                                .text_size(px(11.0))
                                .text_color(TEXT)
                                .child(div().text_color(DANGER).child("!"))
                                .child(
                                    div()
                                        .min_w_0()
                                        .whitespace_normal()
                                        .line_height(relative(1.4))
                                        .child(error),
                                ),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(14.0))
                            .p(px(18.0))
                            .id("tunnel-form-scroll")
                            .overflow_y_scroll()
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
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(MUTED)
                                            .child(self.language.pick("分组", "Group")),
                                    )
                                    .child(group_dropdown),
                            )
                            .child(ssh_connection)
                            .child(
                                div()
                                    .flex()
                                    .gap(px(6.0))
                                    .p(px(4.0))
                                    .rounded(px(9.0))
                                    .bg(APP_BG)
                                    .child(kind_button(
                                        self.language.pick("本地转发", "Local"),
                                        ForwardKind::Local,
                                    ))
                                    .child(kind_button(
                                        self.language.pick("远程转发", "Remote"),
                                        ForwardKind::Remote,
                                    ))
                                    .child(kind_button("SOCKS5", ForwardKind::Socks5)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(14.0))
                                    .p(px(12.0))
                                    .rounded(px(9.0))
                                    .border_1()
                                    .border_color(glass(0x075bea, 0.32))
                                    .bg(glass(0x075bea, 0.08))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(4.0))
                                            .min_w_0()
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(TEXT)
                                                    .child(forward_title),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.0))
                                                    .line_height(relative(1.4))
                                                    .text_color(MUTED)
                                                    .child(forward_description),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex_none()
                                            .px(px(9.0))
                                            .py(px(6.0))
                                            .rounded(px(6.0))
                                            .bg(APP_BG)
                                            .text_size(px(9.0))
                                            .text_color(PRIMARY_TEXT)
                                            .child(forward_route),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(10.0))
                                    .child(div().flex_grow(1.0).child(Self::required_form_field(
                                        listen_address_label,
                                        form.listen_host.clone(),
                                    )))
                                    .child(div().w(px(130.0)).child(Self::required_form_field(
                                        listen_port_label,
                                        form.listen_port.clone(),
                                    ))),
                            )
                            .when(form.kind != ForwardKind::Socks5, |panel| {
                                panel.child(
                                    div()
                                        .flex()
                                        .gap(px(10.0))
                                        .child(div().flex_grow(1.0).child(Self::required_form_field(
                                            target_address_label,
                                            form.target_host.clone(),
                                        )))
                                        .child(div().w(px(130.0)).child(Self::required_form_field(
                                            target_port_label,
                                            form.target_port.clone(),
                                        ))),
                                )
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .p(px(12.0))
                                    .rounded(px(9.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .bg(APP_BG)
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(3.0))
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(TEXT)
                                                    .child(self.language.pick(
                                                        "应用启动时自动连接",
                                                        "Connect when app starts",
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.0))
                                                    .text_color(MUTED)
                                                    .child(self.language.pick(
                                                        "打开 Tunnel Mate 后自动建立此隧道",
                                                        "Connect this tunnel automatically when Tunnel Mate opens",
                                                    )),
                                            ),
                                    )
                                    .child(
                                        switch(form.start_with_app)
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    this.toggle_form_start_with_app(cx)
                                                }),
                                            ),
                                    ),
                            )
                            .child(advanced_settings),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .h(px(58.0))
                            .px(px(18.0))
                            .items_center()
                            .border_t_1()
                            .border_color(BORDER)
                            .when(is_editing, |footer| {
                                footer.justify_between().child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .h(px(36.0))
                                        .px(px(12.0))
                                        .rounded(px(8.0))
                                        .text_size(px(12.0))
                                        .text_color(DANGER)
                                        .cursor_pointer()
                                        .hover(|style| style.bg(glass(0xdc747c, 0.12)))
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.request_delete_from_form(cx)
                                            }),
                                        )
                                        .child(self.language.pick("删除隧道", "Delete tunnel")),
                                )
                            })
                            .when(!is_editing, |footer| footer.justify_end())
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.0))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .h(px(36.0))
                                            .px(px(14.0))
                                            .rounded(px(8.0))
                                            .border_1()
                                            .border_color(BORDER)
                                            .text_size(px(12.0))
                                            .text_color(TEXT)
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    this.close_create_sheet(cx)
                                                }),
                                            )
                                            .child(self.language.pick("取消", "Cancel")),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.0))
                                            .mr(px(4.0))
                                            .h(px(36.0))
                                            .px(px(4.0))
                                            .text_size(px(12.0))
                                            .text_color(TEXT)
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    this.toggle_start_after_save(cx)
                                                }),
                                            )
                                            .child(
                                                div()
                                                    .size(px(20.0))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded(px(5.0))
                                                    .border_1()
                                                    .border_color(if form.start_after_save {
                                                        PRIMARY
                                                    } else {
                                                        color(0x4b5260)
                                                    })
                                                    .bg(if form.start_after_save {
                                                        PRIMARY
                                                    } else {
                                                        APP_BG
                                                    })
                                                    .text_size(px(13.0))
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(PRIMARY_TEXT)
                                                    .child(if form.start_after_save {
                                                        "✓"
                                                    } else {
                                                        ""
                                                    }),
                                            )
                                            .child(
                                                self.language.pick(
                                                    "保存后启动",
                                                    "Start after save",
                                                ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .h(px(36.0))
                                            .px(px(14.0))
                                            .rounded(px(8.0))
                                            .bg(PRIMARY)
                                            .text_size(px(12.0))
                                            .text_color(PRIMARY_TEXT)
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| this.save_form(cx)),
                                            )
                                            .child(self.language.pick("保存", "Save")),
                                    ),
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
