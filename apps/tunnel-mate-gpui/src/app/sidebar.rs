use super::*;

impl TunnelMateApp {
    pub(super) fn status(&self, tunnel_id: &str) -> TunnelStatus {
        self.statuses
            .get(tunnel_id)
            .cloned()
            .unwrap_or(TunnelStatus::Stopped)
    }

    pub(super) fn is_active(&self, tunnel_id: &str) -> bool {
        TunnelStatusFilter::Active.matches(&self.status(tunnel_id))
    }

    pub(super) fn title(&self) -> SharedString {
        match &self.filter {
            TunnelFilter::All => self.language.pick("隧道", "Tunnels").into(),
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
                self.filter.includes(tunnel)
                    && self.status_filter.matches(&self.status(&tunnel.id))
                    && (query.is_empty()
                        || tunnel.name.to_lowercase().contains(&query)
                        || tunnel.ssh_host.to_lowercase().contains(&query)
                        || Self::route(tunnel).to_lowercase().contains(&query))
            })
            .collect()
    }

    pub(super) fn route(tunnel: &Tunnel) -> String {
        match &tunnel.forward {
            ForwardSpec::Local { listen, target } | ForwardSpec::Remote { listen, target } => {
                format!(
                    "{}  →  {}",
                    endpoint_label(&listen.host, listen.port),
                    endpoint_label(&target.host, target.port)
                )
            }
            ForwardSpec::Socks5 { listen } => {
                format!("{}  →  SOCKS5", endpoint_label(&listen.host, listen.port))
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
        let theme = self.theme;
        let selected = self.filter == filter;
        let (glyph, id) = match &filter {
            TunnelFilter::All => ("icons/tunnels", "nav-all".to_string()),
            TunnelFilter::Activity => ("icons/history", "nav-history".into()),
            TunnelFilter::Group(id) => ("icons/folder", format!("nav-group-{id}")),
        };
        div()
            .id(SharedString::from(id))
            .mx(px(12.0))
            .key_context("TunnelButton")
            .tab_index(0)
            .h(px(42.0))
            .flex_none()
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .rounded(px(9.0))
            .border_1()
            .border_color(if selected {
                theme.selected
            } else {
                rgba(0x00000000)
            })
            .bg(if selected {
                theme.selected
            } else {
                rgba(0x00000000)
            })
            .text_size(px(13.0))
            .text_color(if selected { theme.text } else { theme.muted })
            .cursor_pointer()
            .hover(|style| style.bg(theme.surface_hover))
            .focus(|style| style.border_color(theme.primary_hover))
            .on_click(cx.listener(move |this, _, _, cx| this.set_filter(filter.clone(), cx)))
            .child(icon(theme, glyph).text_color(if selected { theme.accent } else { theme.muted }))
            .child(div().flex_1().min_w_0().truncate().child(label.into()))
            .when_some(count, |item, count| {
                item.child(
                    div()
                        .flex_none()
                        .min_w(px(22.0))
                        .text_center()
                        .text_size(px(12.0))
                        .text_color(if selected {
                            theme.accent
                        } else {
                            theme.muted_dark
                        })
                        .child(count.to_string()),
                )
            })
    }

    pub(super) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let total = self.config.tunnels.len();
        let mut groups = div()
            .id("sidebar-groups")
            .track_scroll(&self.groups_scroll)
            .overflow_y_scroll()
            .h_full()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .gap(px(3.0));
        for group in &self.config.groups {
            let count = self
                .config
                .tunnels
                .iter()
                .filter(|t| t.group_id.as_ref() == Some(&group.id))
                .count();
            groups = groups.child(self.nav_item(
                group.name.clone(),
                Some(count),
                TunnelFilter::Group(group.id.clone()),
                cx,
            ));
        }
        div()
            .w(px(196.0))
            .flex_none()
            .h_full()
            .pb(px(14.0))
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(theme.border_soft)
            .bg(theme.sidebar_bg)
            .child(
                div()
                    .h(px(84.0))
                    .flex_none()
                    .px(px(21.0))
                    .flex()
                    .items_center()
                    .gap(px(11.0))
                    .child(img(self.logo.clone()).size(px(32.0)).rounded(px(9.0)))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(15.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Tunnel Mate"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.muted)
                                    .child(self.language.pick("SSH 隧道管理", "SSH connections")),
                            ),
                    ),
            )
            .child(self.nav_item(
                self.language.pick("隧道", "Tunnels"),
                Some(total),
                TunnelFilter::All,
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
                    .mt(px(26.0))
                    .mb(px(8.0))
                    .px(px(24.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(section_heading(theme, self.language.pick("分组", "Groups")))
                    .child(
                        button(theme, "add-group", "")
                            .size(px(28.0))
                            .px(px(0.0))
                            .bg(rgba(0x00000000))
                            .border_color(rgba(0x00000000))
                            .child(icon(theme, "icons/plus"))
                            .on_click(cx.listener(|this, _, _, cx| this.open_group_form(cx))),
                    ),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h(px(0.0))
                    .child(groups)
                    .child(crate::scrollbar::scrollbar(
                        "groups-scrollbar",
                        theme,
                        self.groups_scroll.clone(),
                    )),
            )
            .child(
                div()
                    .h(px(1.0))
                    .flex_none()
                    .mx(px(8.0))
                    .my(px(12.0))
                    .bg(theme.border_soft),
            )
            .child(
                div()
                    .id("open-settings")
                    .mx(px(12.0))
                    .key_context("TunnelButton")
                    .tab_index(0)
                    .h(px(42.0))
                    .flex_none()
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .rounded(px(9.0))
                    .border_1()
                    .border_color(rgba(0x00000000))
                    .text_size(px(13.0))
                    .text_color(theme.muted)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.surface_hover))
                    .focus(|style| style.border_color(theme.primary_hover))
                    .on_click(cx.listener(|this, _, _, cx| this.open_settings(cx)))
                    .child(icon(theme, "icons/settings"))
                    .child(self.language.pick("设置", "Settings")),
            )
    }

    pub(super) fn render_tunnel_row(
        &self,
        tunnel: &Tunnel,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let selected = self.selected_tunnel.as_deref() == Some(tunnel.id.as_str());
        let status = self.status(&tunnel.id);
        let (tone, status_label, connect_label) = match status {
            TunnelStatus::Running => (
                theme.success,
                self.language.pick("已连接", "Connected"),
                self.language.pick("断开", "Disconnect"),
            ),
            TunnelStatus::Connecting => (
                theme.warning,
                self.language.pick("连接中", "Connecting"),
                self.language.pick("取消", "Cancel"),
            ),
            TunnelStatus::Reconnecting => (
                theme.warning,
                self.language.pick("重连中", "Reconnecting"),
                self.language.pick("取消", "Cancel"),
            ),
            TunnelStatus::Failed => (
                theme.danger,
                self.language.pick("连接失败", "Failed"),
                self.language.pick("重试", "Retry"),
            ),
            TunnelStatus::Stopped => (
                theme.muted_dark,
                self.language.pick("未连接", "Offline"),
                self.language.pick("连接", "Connect"),
            ),
        };
        let (kind, listen, target) = match &tunnel.forward {
            ForwardSpec::Local { listen, target } => {
                (self.language.pick("本地", "Local"), listen, Some(target))
            }
            ForwardSpec::Remote { listen, target } => {
                (self.language.pick("远程", "Remote"), listen, Some(target))
            }
            ForwardSpec::Socks5 { listen } => ("SOCKS5", listen, None),
        };
        let select_id = tunnel.id.clone();
        let toggle_id = tunnel.id.clone();
        let diagnose_id = tunnel.id.clone();
        let edit_id = tunnel.id.clone();
        let host = format!(
            "{}@{}{}",
            tunnel.ssh_user,
            endpoint_label(&tunnel.ssh_host, tunnel.ssh_port),
            if tunnel.jump_host_enabled {
                self.language.pick(" · 跳板机", " · jump host")
            } else {
                ""
            }
        );
        super::workspace::tunnel_columns(
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .h(px(32.0))
                        .line_height(px(32.0))
                        .truncate()
                        .text_size(px(14.0))
                        .font_weight(FontWeight::MEDIUM)
                        .child(tunnel.name.clone()),
                )
                .child(
                    div()
                        .h(px(18.0))
                        .line_height(px(18.0))
                        .truncate()
                        .text_size(px(12.0))
                        .text_color(theme.muted)
                        .child(host),
                ),
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .h(px(32.0))
                        .line_height(px(32.0))
                        .truncate()
                        .text_size(px(13.0))
                        .child(endpoint_label(&listen.host, listen.port)),
                )
                .child(
                    div()
                        .h(px(18.0))
                        .line_height(px(18.0))
                        .truncate()
                        .text_size(px(12.0))
                        .text_color(theme.muted)
                        .child(
                            target
                                .map(|target| {
                                    format!("→ {}", endpoint_label(&target.host, target.port))
                                })
                                .unwrap_or_else(|| {
                                    self.language
                                        .pick("→ 动态代理", "→ Dynamic proxy")
                                        .to_string()
                                }),
                        ),
                ),
            div()
                .h(px(32.0))
                .line_height(px(32.0))
                .text_size(px(12.0))
                .text_color(theme.muted)
                .child(kind),
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    button(
                        theme,
                        SharedString::from(format!("toggle-{}", tunnel.id)),
                        connect_label,
                    )
                    .aria_label(format!("{status_label}, {connect_label}"))
                    .flex_row_reverse()
                    .w(px(104.0))
                    .h(px(32.0))
                    .px(px(8.0))
                    .bg(if selected {
                        theme.selected
                    } else {
                        theme.surface
                    })
                    .border_color(if selected {
                        theme.selected_border
                    } else {
                        theme.border
                    })
                    .text_color(if status == TunnelStatus::Failed {
                        theme.danger
                    } else if !self.is_active(&tunnel.id) {
                        theme.accent
                    } else {
                        theme.text
                    })
                    .child(div().size(px(6.0)).flex_none().rounded(px(3.0)).bg(tone))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.selected_tunnel = Some(toggle_id.clone());
                        this.request_toggle(toggle_id.clone(), cx);
                    })),
                )
                .child(
                    button(
                        theme,
                        SharedString::from(format!("diagnose-{}", tunnel.id)),
                        self.language.pick("诊断", "Diagnose"),
                    )
                    .w(px(72.0))
                    .h(px(32.0))
                    .px(px(8.0))
                    .bg(rgba(0x00000000))
                    .border_color(rgba(0x00000000))
                    .text_color(theme.muted)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.run_tunnel_diagnostics(diagnose_id.clone(), cx);
                    })),
                )
                .child(
                    button(
                        theme,
                        SharedString::from(format!("edit-{}", tunnel.id)),
                        self.language.pick("编辑", "Edit"),
                    )
                    .w(px(52.0))
                    .h(px(32.0))
                    .px(px(8.0))
                    .bg(rgba(0x00000000))
                    .border_color(rgba(0x00000000))
                    .text_color(theme.muted)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.edit_tunnel(edit_id.clone(), cx);
                    })),
                ),
        )
        .id(SharedString::from(format!("tunnel-row-{}", tunnel.id)))
        .role(gpui::Role::Button)
        .aria_label(tunnel.name.clone())
        .key_context("TunnelButton")
        .tab_index(0)
        .h(px(84.0))
        .py(px(14.0))
        .border_b_1()
        .border_color(theme.border_soft)
        .bg(if selected {
            theme.selected
        } else {
            theme.surface
        })
        .cursor_pointer()
        .hover(|style| style.bg(theme.surface_hover))
        .focus(|style| style.border_color(theme.primary))
        .on_click(cx.listener(move |this, _, _, cx| this.select_tunnel(select_id.clone(), cx)))
    }

    pub(super) fn form_field(
        theme: Theme,
        label: &'static str,
        input: Entity<TextInput>,
    ) -> impl IntoElement {
        Self::form_field_with_requirement(theme, label, input, false)
    }

    pub(super) fn disclosure_chevron(
        theme: Theme,
        expanded: bool,
        expanded_glyph: &'static str,
        collapsed_glyph: &'static str,
    ) -> impl IntoElement {
        div()
            .flex_none()
            .size(px(20.0))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(16.0))
            .line_height(relative(1.0))
            .text_color(theme.muted)
            .child(if expanded {
                expanded_glyph
            } else {
                collapsed_glyph
            })
    }

    pub(super) fn required_form_field(
        theme: Theme,
        label: &'static str,
        input: Entity<TextInput>,
    ) -> impl IntoElement {
        Self::form_field_with_requirement(theme, label, input, true)
    }

    fn form_field_with_requirement(
        theme: Theme,
        label: &'static str,
        input: Entity<TextInput>,
        required: bool,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(3.0))
                    .text_size(px(12.0))
                    .text_color(theme.muted)
                    .child(label)
                    .when(required, |label| {
                        label.child(div().text_color(theme.danger).child("*"))
                    }),
            )
            .child(input)
    }
}
