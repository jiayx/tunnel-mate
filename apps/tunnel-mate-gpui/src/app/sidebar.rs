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
            TunnelFilter::Active => ("icons/activity", "nav-active".into()),
            TunnelFilter::Activity => ("icons/activity", "nav-history".into()),
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
                theme.selected_border
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
            .child(icon(theme, glyph))
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
        let active = self
            .config
            .tunnels
            .iter()
            .filter(|t| self.is_active(&t.id))
            .count();
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
            .w(px(212.0))
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
                    .h(px(96.0))
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
        let running = self.is_active(&tunnel.id);
        let (tone, status_label) = match status {
            TunnelStatus::Running => (theme.success, self.language.pick("已连接", "Connected")),
            TunnelStatus::Connecting => (theme.warning, self.language.pick("连接中", "Connecting")),
            TunnelStatus::Reconnecting => {
                (theme.warning, self.language.pick("重连中", "Reconnecting"))
            }
            TunnelStatus::Failed => (theme.danger, self.language.pick("连接失败", "Failed")),
            TunnelStatus::Stopped => (theme.muted, self.language.pick("未连接", "Offline")),
        };
        let kind = match tunnel.forward {
            ForwardSpec::Local { .. } => self.language.pick("本地", "Local"),
            ForwardSpec::Remote { .. } => self.language.pick("远程", "Remote"),
            ForwardSpec::Socks5 { .. } => "SOCKS5",
        };
        let select_id = tunnel.id.clone();
        let toggle_id = tunnel.id.clone();
        let diagnose_id = tunnel.id.clone();
        let edit_id = tunnel.id.clone();
        let mut host = format!(
            "SSH  {}@{}",
            tunnel.ssh_user,
            endpoint_label(&tunnel.ssh_host, tunnel.ssh_port)
        );
        if tunnel.jump_host_enabled {
            host.push_str(self.language.pick("  ·  经跳板机", "  ·  via jump host"));
        }
        let connect_label = match status {
            TunnelStatus::Connecting | TunnelStatus::Reconnecting => {
                self.language.pick("取消", "Cancel")
            }
            TunnelStatus::Running => self.language.pick("断开", "Disconnect"),
            TunnelStatus::Failed => self.language.pick("重试", "Retry"),
            TunnelStatus::Stopped => self.language.pick("连接", "Connect"),
        };
        div()
            .h(px(100.0))
            .w_full()
            .flex()
            .items_center()
            .gap(px(16.0))
            .px(px(24.0))
            .py(px(14.0))
            .border_b_1()
            .border_color(theme.border_soft)
            .bg(if selected {
                theme.selected
            } else {
                theme.app_bg
            })
            .cursor_pointer()
            .hover(|style| style.bg(theme.surface_hover))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.select_tunnel(select_id.clone(), cx)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(5.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(15.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(tunnel.name.clone()),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .text_size(px(11.0))
                                    .text_color(theme.muted)
                                    .child(kind),
                            ),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(12.0))
                            .text_color(theme.text)
                            .child(Self::route(tunnel)),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(12.0))
                            .text_color(theme.muted)
                            .child(host),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap(px(10.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .text_size(px(12.0))
                            .text_color(tone)
                            .child(div().size(px(6.0)).rounded(px(3.0)).bg(tone))
                            .child(status_label),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(6.0))
                            .child(
                                button(
                                    theme,
                                    SharedString::from(format!("diagnose-{}", tunnel.id)),
                                    self.language.pick("诊断", "Diagnose"),
                                )
                                .h(px(30.0))
                                .px(px(9.0))
                                .bg(rgba(0x00000000))
                                .border_color(theme.border_soft)
                                .text_color(theme.muted)
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.run_tunnel_diagnostics(diagnose_id.clone(), cx);
                                    },
                                )),
                            )
                            .child(
                                button(
                                    theme,
                                    SharedString::from(format!("edit-{}", tunnel.id)),
                                    self.language.pick("编辑", "Edit"),
                                )
                                .h(px(30.0))
                                .px(px(9.0))
                                .bg(rgba(0x00000000))
                                .border_color(theme.border_soft)
                                .text_color(theme.muted)
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.edit_tunnel(edit_id.clone(), cx);
                                    },
                                )),
                            )
                            .child(
                                button(
                                    theme,
                                    SharedString::from(format!("toggle-{}", tunnel.id)),
                                    connect_label,
                                )
                                .h(px(30.0))
                                .min_w(px(56.0))
                                .px(px(10.0))
                                .bg(if running {
                                    theme.surface
                                } else {
                                    theme.primary
                                })
                                .border_color(if running { theme.border } else { theme.primary })
                                .text_color(if running {
                                    theme.text
                                } else {
                                    theme.primary_text
                                })
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.request_toggle(toggle_id.clone(), cx);
                                    },
                                )),
                            ),
                    ),
            )
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
