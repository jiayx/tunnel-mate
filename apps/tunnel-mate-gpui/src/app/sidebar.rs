use super::*;

impl TunnelMateApp {
    pub(super) fn status(&self, tunnel_id: &str) -> TunnelStatus {
        self.statuses
            .get(tunnel_id)
            .cloned()
            .unwrap_or(TunnelStatus::Stopped)
    }

    pub(super) fn is_active(&self, tunnel_id: &str) -> bool {
        matches!(
            self.status(tunnel_id),
            TunnelStatus::Running | TunnelStatus::Connecting | TunnelStatus::Reconnecting
        )
    }

    pub(super) fn title(&self) -> SharedString {
        match &self.filter {
            TunnelFilter::All => self.language.pick("隧道", "Tunnels").into(),
            TunnelFilter::Active => self.language.pick("运行中", "Active").into(),
            TunnelFilter::Activity => self.language.pick("活动记录", "Activity").into(),
            TunnelFilter::Group(group_id) => self
                .config
                .groups
                .iter()
                .find(|group| &group.id == group_id)
                .map(|group| group.name.clone().into())
                .unwrap_or_else(|| self.language.pick("分组", "Group").into()),
        }
    }

    pub(super) fn filtered_tunnels(&self, cx: &Context<Self>) -> Vec<&Tunnel> {
        let query = self.search.read(cx).value().trim().to_lowercase();
        self.config
            .tunnels
            .iter()
            .filter(|tunnel| {
                let in_filter = match &self.filter {
                    TunnelFilter::All => true,
                    TunnelFilter::Active => self.is_active(&tunnel.id),
                    TunnelFilter::Activity => false,
                    TunnelFilter::Group(group_id) => tunnel.group_id.as_ref() == Some(group_id),
                };
                in_filter
                    && (query.is_empty()
                        || tunnel.name.to_lowercase().contains(&query)
                        || tunnel.ssh_host.to_lowercase().contains(&query)
                        || Self::route(tunnel).to_lowercase().contains(&query))
            })
            .collect()
    }

    pub(super) fn group_name(&self, tunnel: &Tunnel) -> SharedString {
        tunnel
            .group_id
            .as_ref()
            .and_then(|id| self.config.groups.iter().find(|group| &group.id == id))
            .map(|group| group.name.clone().into())
            .unwrap_or_else(|| self.language.pick("未分组", "Ungrouped").into())
    }

    pub(super) fn route(tunnel: &Tunnel) -> String {
        match &tunnel.forward {
            ForwardSpec::Local { listen, target } => format!(
                "{}:{}  →  {}:{}",
                listen.host, listen.port, target.host, target.port
            ),
            ForwardSpec::Remote { listen, target } => format!(
                "{}:{}  ←  {}:{}",
                listen.host, listen.port, target.host, target.port
            ),
            ForwardSpec::Socks5 { listen } => {
                format!("{}:{}  →  SOCKS5", listen.host, listen.port)
            }
        }
    }

    pub(super) fn nav_item(
        &self,
        label: impl Into<SharedString>,
        count: Option<usize>,
        filter: TunnelFilter,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.filter == filter;
        let icon = match &filter {
            TunnelFilter::All => "◇",
            TunnelFilter::Active => "▷",
            TunnelFilter::Activity => "↻",
            TunnelFilter::Group(_) => "",
        };
        let icon_color = if selected { TEXT } else { MUTED_DARK };
        let icon_element = if matches!(&filter, TunnelFilter::Group(_)) {
            div()
                .flex()
                .flex_col()
                .items_start()
                .justify_center()
                .w(px(15.0))
                .h(px(14.0))
                .child(
                    div()
                        .ml(px(1.0))
                        .w(px(7.0))
                        .h(px(3.0))
                        .rounded_t(px(2.0))
                        .bg(icon_color),
                )
                .child(
                    div()
                        .w(px(15.0))
                        .h(px(10.0))
                        .rounded(px(2.0))
                        .border_1()
                        .border_color(icon_color),
                )
                .into_any_element()
        } else {
            div()
                .w(px(15.0))
                .text_center()
                .text_size(px(12.0))
                .text_color(icon_color)
                .child(icon)
                .into_any_element()
        };
        let handler_filter = filter.clone();

        div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(42.0))
            .px(px(12.0))
            .rounded(px(7.0))
            .text_size(px(12.0))
            .text_color(if selected { TEXT } else { MUTED })
            .font_weight(if selected {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            })
            .bg(if selected {
                glass(0x123b7a, 0.62)
            } else {
                SIDEBAR_BG
            })
            .cursor_pointer()
            .hover(|style| style.bg(SURFACE_HOVER))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.set_filter(handler_filter.clone(), cx)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(icon_element)
                    .child(label.into()),
            )
            .when_some(count, |item, count| {
                item.child(
                    div()
                        .min_w(px(22.0))
                        .h(px(20.0))
                        .px(px(6.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(10.0))
                        .bg(if selected { color(0x282d32) } else { SURFACE })
                        .text_size(px(10.0))
                        .text_color(if selected { MUTED } else { MUTED_DARK })
                        .child(count.to_string()),
                )
            })
    }

    pub(super) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let total = self.config.tunnels.len();
        let active = self
            .config
            .tunnels
            .iter()
            .filter(|tunnel| self.is_active(&tunnel.id))
            .count();
        let mut sidebar = div()
            .flex()
            .flex_col()
            .w(px(208.0))
            .h_full()
            .px(px(12.0))
            .pb(px(12.0))
            .border_r_1()
            .border_color(BORDER_SOFT)
            .bg(SIDEBAR_BG)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(11.0))
                    .h(px(64.0))
                    .px(px(8.0))
                    .mb(px(12.0))
                    .child(img(self.logo.clone()).size(px(32.0)).rounded(px(8.0)))
                    .child(
                        div()
                            .text_size(px(15.0))
                            .text_color(TEXT)
                            .font_weight(FontWeight::MEDIUM)
                            .child("Tunnel Mate"),
                    ),
            )
            .child(self.nav_item(
                self.language.pick("全部隧道", "All tunnels"),
                Some(total),
                TunnelFilter::All,
                cx,
            ))
            .child(self.nav_item(
                self.language.pick("正在运行", "Active"),
                Some(active),
                TunnelFilter::Active,
                cx,
            ))
            .child(self.nav_item(
                self.language.pick("活动记录", "Activity"),
                None,
                TunnelFilter::Activity,
                cx,
            ))
            .child(
                div()
                    .mt(px(22.0))
                    .mb(px(8.0))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(9.0))
                    .text_color(MUTED_DARK)
                    .child(self.language.pick("分组", "Groups"))
                    .child(
                        div()
                            .size(px(22.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(5.0))
                            .text_size(px(15.0))
                            .cursor_pointer()
                            .hover(|style| style.bg(SURFACE_HOVER))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.open_group_form(cx)),
                            )
                            .child("+"),
                    ),
            );

        for group in &self.config.groups {
            let count = self
                .config
                .tunnels
                .iter()
                .filter(|tunnel| tunnel.group_id.as_ref() == Some(&group.id))
                .count();
            sidebar = sidebar.child(self.nav_item(
                group.name.clone(),
                Some(count),
                TunnelFilter::Group(group.id.clone()),
                cx,
            ));
        }

        sidebar
            .child(div().flex_grow(1.0))
            .child(div().h(px(1.0)).mx(px(4.0)).mb(px(10.0)).bg(BORDER_SOFT))
            .child(
                div()
                    .h(px(42.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .rounded(px(7.0))
                    .text_size(px(12.0))
                    .text_color(MUTED)
                    .cursor_pointer()
                    .hover(|style| style.bg(SURFACE_HOVER))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.open_settings(cx)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .w(px(15.0))
                                    .text_center()
                                    .text_size(px(14.0))
                                    .text_color(MUTED_DARK)
                                    .child("⚙"),
                            )
                            .child(self.language.pick("设置", "Settings")),
                    ),
            )
    }

    pub(super) fn render_tunnel_row(
        &self,
        tunnel: &Tunnel,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.selected_tunnel.as_deref() == Some(tunnel.id.as_str());
        let status = self.status(&tunnel.id);
        let running = self.is_active(&tunnel.id);
        let select_id = tunnel.id.clone();
        let toggle_id = tunnel.id.clone();
        let diagnose_id = tunnel.id.clone();
        let edit_id = tunnel.id.clone();
        let status_color = match status {
            TunnelStatus::Running => SUCCESS,
            TunnelStatus::Connecting | TunnelStatus::Reconnecting => WARNING,
            TunnelStatus::Failed => DANGER,
            TunnelStatus::Stopped => MUTED_DARK,
        };
        let status_label = match status {
            TunnelStatus::Connecting => self.language.pick("连接中", "Connecting"),
            TunnelStatus::Reconnecting => self.language.pick("重连中", "Reconnecting"),
            TunnelStatus::Running => self.language.pick("已连接", "Connected"),
            TunnelStatus::Failed => self.language.pick("连接失败", "Failed"),
            TunnelStatus::Stopped => self.language.pick("已停止", "Stopped"),
        };
        let kind_label = match &tunnel.forward {
            ForwardSpec::Local { .. } => "LOCAL",
            ForwardSpec::Remote { .. } => "REMOTE",
            ForwardSpec::Socks5 { .. } => "SOCKS5",
        };

        div()
            .flex()
            .items_center()
            .h(px(78.0))
            .mx(px(8.0))
            .mt(px(8.0))
            .px(px(18.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(if selected {
                glass(0x2f76f6, 0.72)
            } else {
                rgba(0x00000000)
            })
            .bg(if selected {
                glass(0x112c58, 0.58)
            } else {
                APP_BG
            })
            .cursor_pointer()
            .hover(|style| style.bg(SURFACE_HOVER))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.select_tunnel(select_id.clone(), cx)),
            )
            .child(
                div()
                    .size(px(9.0))
                    .mr(px(14.0))
                    .rounded(px(4.5))
                    .bg(status_color),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .flex_grow(1.0)
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(TEXT)
                            .child(tunnel.name.clone()),
                    )
                    .child(
                        div()
                            .mt(px(6.0))
                            .text_size(px(10.0))
                            .text_color(MUTED)
                            .child(Self::route(tunnel)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .mr(px(14.0))
                    .child(
                        div()
                            .h(px(30.0))
                            .px(px(11.0))
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(glass(0x2f76f6, 0.68))
                            .bg(glass(0x075bea, 0.10))
                            .text_size(px(10.0))
                            .text_color(color(0xaecbff))
                            .cursor_pointer()
                            .hover(|style| style.bg(glass(0x075bea, 0.24)).text_color(TEXT))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.run_tunnel_diagnostics(diagnose_id.clone(), cx)
                                }),
                            )
                            .child("◇")
                            .child(self.language.pick("诊断", "Diagnose")),
                    )
                    .child(
                        div()
                            .h(px(30.0))
                            .px(px(11.0))
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(BORDER)
                            .bg(glass(0xffffff, 0.025))
                            .text_size(px(10.0))
                            .text_color(MUTED)
                            .cursor_pointer()
                            .hover(|style| style.bg(SURFACE_HOVER).text_color(TEXT))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.edit_tunnel(edit_id.clone(), cx)
                                }),
                            )
                            .child("✎")
                            .child(self.language.pick("编辑", "Edit")),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .w(px(74.0))
                    .child(
                        div()
                            .px(px(7.0))
                            .py(px(3.0))
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(BORDER)
                            .text_size(px(8.0))
                            .text_color(MUTED)
                            .child(kind_label),
                    )
                    .child(
                        div()
                            .mt(px(6.0))
                            .text_size(px(9.0))
                            .text_color(status_color)
                            .child(status_label),
                    ),
            )
            .child(
                div()
                    .w(px(48.0))
                    .h(px(26.0))
                    .p(px(3.0))
                    .rounded(px(13.0))
                    .border_1()
                    .border_color(if running { PRIMARY } else { color(0x3a3f46) })
                    .bg(if running { PRIMARY } else { color(0x202329) })
                    .cursor_pointer()
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.request_toggle(toggle_id.clone(), cx)
                        }),
                    )
                    .child(
                        div()
                            .size(px(18.0))
                            .rounded(px(9.0))
                            .bg(if running {
                                PRIMARY_TEXT
                            } else {
                                color(0xd7d9dc)
                            })
                            .when(running, |dot| dot.ml(px(20.0))),
                    ),
            )
    }

    pub(super) fn form_field(label: &'static str, input: Entity<TextInput>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(div().text_size(px(11.0)).text_color(MUTED).child(label))
            .child(input)
    }
}
