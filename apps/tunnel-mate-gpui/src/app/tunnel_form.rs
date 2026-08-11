use super::*;

impl TunnelMateApp {
    pub(super) fn render_create_sheet(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.form.as_ref().expect("open form");
        let is_editing = form.editing_id.is_some();
        let heading = if form.editing_id.is_some() {
            self.language.pick("编辑隧道", "Edit tunnel")
        } else {
            self.language.pick("新建隧道", "New tunnel")
        };
        let group_name = form
            .group_id
            .as_ref()
            .and_then(|id| self.config.groups.iter().find(|group| &group.id == id))
            .map(|group| group.name.as_str())
            .unwrap_or(self.language.pick("未分组", "Ungrouped"));
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
        let selected_group_id = form.group_id.clone();
        let mut group_options = div()
            .flex()
            .flex_col()
            .w(px(220.0))
            .max_h(px(240.0))
            .id("tunnel-group-options")
            .overflow_y_scroll()
            .on_scroll_wheel(stop_scroll_propagation)
            .p(px(4.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(BORDER)
            .bg(color(0x111722))
            .shadow_lg()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation());
        let ungrouped_selected = selected_group_id.is_none();
        group_options = group_options.child(
            div()
                .id("group-option-ungrouped")
                .h(px(32.0))
                .px(px(9.0))
                .flex()
                .items_center()
                .justify_between()
                .rounded(px(6.0))
                .border_1()
                .border_color(if ungrouped_selected {
                    glass(0x075bea, 0.48)
                } else {
                    rgba(0x00000000)
                })
                .bg(if ungrouped_selected {
                    glass(0x075bea, 0.18)
                } else {
                    rgba(0x00000000)
                })
                .text_size(px(10.0))
                .text_color(if ungrouped_selected { TEXT } else { MUTED })
                .cursor_pointer()
                .hover(|style| {
                    style
                        .bg(if ungrouped_selected {
                            glass(0x075bea, 0.18)
                        } else {
                            glass(0x075bea, 0.10)
                        })
                        .border_color(glass(0x075bea, 0.36))
                        .text_color(TEXT)
                })
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        cx.stop_propagation();
                        this.select_form_group(None, cx);
                    }),
                )
                .child(self.language.pick("未分组", "Ungrouped"))
                .child(if ungrouped_selected { "✓" } else { "" }),
        );
        for (index, group) in self.config.groups.iter().enumerate() {
            let group_id = group.id.clone();
            let selected = selected_group_id.as_ref() == Some(&group.id);
            group_options = group_options.child(
                div()
                    .id(("group-option", index))
                    .h(px(32.0))
                    .px(px(9.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(if selected {
                        glass(0x075bea, 0.48)
                    } else {
                        rgba(0x00000000)
                    })
                    .bg(if selected {
                        glass(0x075bea, 0.18)
                    } else {
                        rgba(0x00000000)
                    })
                    .text_size(px(10.0))
                    .text_color(if selected { TEXT } else { MUTED })
                    .cursor_pointer()
                    .hover(|style| {
                        style
                            .bg(if selected {
                                glass(0x075bea, 0.18)
                            } else {
                                glass(0x075bea, 0.10)
                            })
                            .border_color(glass(0x075bea, 0.36))
                            .text_color(TEXT)
                    })
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.select_form_group(Some(group_id.clone()), cx)
                        }),
                    )
                    .child(group.name.clone())
                    .child(if selected { "✓" } else { "" }),
            );
        }
        let group_dropdown = div()
            .relative()
            .w(px(220.0))
            .h(px(34.0))
            .child(
                div()
                    .w_full()
                    .h_full()
                    .px(px(11.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(px(7.0))
                    .border_1()
                    .border_color(if form.group_menu_open {
                        PRIMARY
                    } else {
                        BORDER
                    })
                    .bg(APP_BG)
                    .text_size(px(10.0))
                    .text_color(TEXT)
                    .cursor_pointer()
                    .hover(|style| style.bg(SURFACE_HOVER))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            cx.stop_propagation();
                            this.toggle_form_group_menu(cx);
                        }),
                    )
                    .child(group_name.to_string())
                    .child(Self::disclosure_chevron(form.group_menu_open, "▴", "▾")),
            )
            .when(form.group_menu_open, |container| {
                container.child(
                    deferred(
                        anchored()
                            .anchor(Anchor::TopRight)
                            .position(point(px(220.0), px(4.0)))
                            .position_mode(AnchoredPositionMode::Local)
                            .snap_to_window_with_margin(px(8.0))
                            .child(group_options),
                    )
                    .priority(2),
                )
            });
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
                            .child(
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
                                                            .child(self.language.pick(
                                                                "SSH 连接",
                                                                "SSH connection",
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
                                                    .border_color(glass(0x075bea, 0.45))
                                                    .bg(glass(0x075bea, 0.10))
                                                    .text_size(px(10.0))
                                                    .text_color(PRIMARY_TEXT)
                                                    .cursor_pointer()
                                                    .hover(|style| {
                                                        style.bg(glass(0x075bea, 0.16))
                                                    })
                                                    .on_mouse_up(
                                                        MouseButton::Left,
                                                        cx.listener(|this, _, _, cx| {
                                                            this.open_primary_ssh_hosts(cx)
                                                        }),
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
                                    ),
                            )
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
                                            "认证、重连与跳板机",
                                            "Auth, reconnect & jump host",
                                        )
                                    }),
                            )
                            .when(form.advanced, |panel| {
                                panel
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
                                                            this.select_private_key(
                                                                PrivateKeyTarget::Primary,
                                                                cx,
                                                            )
                                                        }),
                                                    )
                                                    .child(self.language.pick("选择…", "Choose…")),
                                            ),
                                    )
                                    .child(Self::form_field(
                                        self.language.pick("SSH 密码", "SSH password"),
                                        form.ssh_password.clone(),
                                    ))
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
                            }),
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
