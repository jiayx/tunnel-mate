use super::*;

impl TunnelMateApp {
    #[allow(dead_code)]
    pub(super) fn render_context_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let shell = div()
            .flex()
            .flex_col()
            .w(px(356.0))
            .my(px(8.0))
            .mr(px(8.0))
            .rounded(px(9.0))
            .border_1()
            .border_color(BORDER)
            .bg(color(0x171a1f))
            .overflow_hidden();
        let Some(tunnel) = self
            .selected_tunnel
            .as_ref()
            .and_then(|id| self.config.tunnels.iter().find(|tunnel| &tunnel.id == id))
        else {
            return shell.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .h_full()
                    .px(px(32.0))
                    .text_center()
                    .child(
                        div()
                            .size(px(38.0))
                            .rounded(px(19.0))
                            .border_1()
                            .border_color(BORDER)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(MUTED_DARK)
                            .child("→"),
                    )
                    .child(
                        div()
                            .mt(px(14.0))
                            .text_size(px(11.0))
                            .text_color(MUTED)
                            .child("选择一个隧道查看连接详情"),
                    ),
            );
        };
        let status = self.status(&tunnel.id);
        let running = self.is_active(&tunnel.id);
        let (status_label, status_color) = match status {
            TunnelStatus::Running => ("已连接", SUCCESS),
            TunnelStatus::Connecting => ("连接中", WARNING),
            TunnelStatus::Reconnecting => ("重连中", WARNING),
            TunnelStatus::Failed => ("连接失败", DANGER),
            TunnelStatus::Stopped => ("已停止", MUTED_DARK),
        };
        let toggle_id = tunnel.id.clone();
        let mut recent_activity = div().flex().flex_col();
        let events = self
            .events
            .iter()
            .rev()
            .filter(|event| event.tunnel_id.as_deref() == Some(tunnel.id.as_str()))
            .take(4)
            .collect::<Vec<_>>();
        if events.is_empty() {
            recent_activity = recent_activity.child(
                div()
                    .py(px(14.0))
                    .text_size(px(10.0))
                    .text_color(MUTED_DARK)
                    .child("暂无活动记录"),
            );
        } else {
            for event in events {
                recent_activity = recent_activity.child(
                    div()
                        .flex()
                        .items_center()
                        .h(px(34.0))
                        .child(div().size(px(6.0)).rounded(px(3.0)).mr(px(9.0)).bg(
                            match event.event_type {
                                tunnel_core::event_logger::EventType::Failed => DANGER,
                                tunnel_core::event_logger::EventType::Started
                                | tunnel_core::event_logger::EventType::Reconnected => SUCCESS,
                                _ => MUTED_DARK,
                            },
                        ))
                        .child(
                            div()
                                .flex_grow(1.0)
                                .min_w_0()
                                .text_size(px(9.0))
                                .text_color(MUTED)
                                .child(event.message.clone()),
                        )
                        .child(
                            div()
                                .ml(px(8.0))
                                .text_size(px(9.0))
                                .text_color(MUTED_DARK)
                                .child(event.timestamp.format("%H:%M").to_string()),
                        ),
                );
            }
        }

        let mut panel = shell
            .child(
                div()
                    .px(px(22.0))
                    .pt(px(24.0))
                    .pb(px(20.0))
                    .border_b_1()
                    .border_color(BORDER_SOFT)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .size(px(9.0))
                                    .rounded(px(4.5))
                                    .mr(px(11.0))
                                    .bg(status_color),
                            )
                            .child(
                                div()
                                    .flex_grow(1.0)
                                    .text_size(px(15.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(TEXT)
                                    .child(tunnel.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(MUTED_DARK)
                                    .child("•••"),
                            ),
                    )
                    .when_some(tunnel.description.clone(), |header, description| {
                        header.child(
                            div()
                                .mt(px(8.0))
                                .text_size(px(10.0))
                                .text_color(MUTED)
                                .child(description),
                        )
                    }),
            )
            .child(
                div()
                    .px(px(22.0))
                    .py(px(20.0))
                    .border_b_1()
                    .border_color(BORDER_SOFT)
                    .child(
                        div()
                            .mb(px(14.0))
                            .text_size(px(10.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(TEXT)
                            .child("连接摘要"),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(MUTED)
                            .child(Self::route(tunnel)),
                    )
                    .child(
                        div()
                            .mt(px(10.0))
                            .text_size(px(10.0))
                            .text_color(MUTED)
                            .child(format!(
                                "{}@{}:{}",
                                tunnel.ssh_user, tunnel.ssh_host, tunnel.ssh_port
                            )),
                    )
                    .child(
                        div()
                            .mt(px(10.0))
                            .text_size(px(9.0))
                            .text_color(MUTED_DARK)
                            .child(self.group_name(tunnel)),
                    )
                    .child(
                        div()
                            .mt(px(14.0))
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .size(px(7.0))
                                    .rounded(px(3.5))
                                    .mr(px(8.0))
                                    .bg(status_color),
                            )
                            .child(
                                div()
                                    .text_size(px(9.0))
                                    .text_color(status_color)
                                    .child(status_label),
                            ),
                    )
                    .child(
                        div()
                            .mt(px(18.0))
                            .h(px(42.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(7.0))
                            .border_1()
                            .border_color(if running { color(0x48695e) } else { PRIMARY })
                            .bg(if running { color(0x1a2b25) } else { PRIMARY })
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if running { PRIMARY } else { PRIMARY_TEXT })
                            .cursor_pointer()
                            .hover(|style| {
                                style.bg(if running {
                                    color(0x20352e)
                                } else {
                                    PRIMARY_HOVER
                                })
                            })
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.request_toggle(toggle_id.clone(), cx)
                                }),
                            )
                            .child(if running {
                                "停止隧道"
                            } else {
                                "启动隧道"
                            }),
                    )
                    .child(
                        div()
                            .mt(px(10.0))
                            .flex()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex_grow(1.0)
                                    .h(px(34.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(6.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(10.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(SURFACE_HOVER))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.run_selected_diagnostics(cx)
                                        }),
                                    )
                                    .child("连接诊断"),
                            )
                            .child(
                                div()
                                    .flex_grow(1.0)
                                    .h(px(34.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(6.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(10.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(SURFACE_HOVER))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.edit_selected(cx)),
                                    )
                                    .child("编辑"),
                            ),
                    ),
            )
            .child(
                div()
                    .px(px(22.0))
                    .py(px(18.0))
                    .border_b_1()
                    .border_color(BORDER_SOFT)
                    .child(
                        div()
                            .mb(px(8.0))
                            .text_size(px(10.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(TEXT)
                            .child("最近活动"),
                    )
                    .child(recent_activity),
            );

        if let Some(logs) = self.logs.get(&tunnel.id).filter(|logs| !logs.is_empty()) {
            let mut log_panel = div()
                .px(px(22.0))
                .py(px(18.0))
                .border_b_1()
                .border_color(BORDER_SOFT)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .mb(px(8.0))
                        .child(
                            div()
                                .text_size(px(10.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(TEXT)
                                .child("实时日志"),
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(10.0))
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(MUTED)
                                        .cursor_pointer()
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.copy_selected_logs(cx)
                                            }),
                                        )
                                        .child("复制"),
                                )
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(MUTED)
                                        .cursor_pointer()
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.export_selected_logs(cx)
                                            }),
                                        )
                                        .child("导出"),
                                )
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(DANGER)
                                        .cursor_pointer()
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.clear_selected_logs(cx)
                                            }),
                                        )
                                        .child("清空"),
                                ),
                        ),
                );
            for line in logs.iter().rev().take(3).rev() {
                log_panel = log_panel.child(
                    div()
                        .mt(px(5.0))
                        .text_size(px(9.0))
                        .text_color(MUTED_DARK)
                        .child(line.clone()),
                );
            }
            panel = panel.child(log_panel);
        }

        panel.child(div().flex_grow(1.0)).child(
            div()
                .h(px(52.0))
                .px(px(22.0))
                .flex()
                .items_center()
                .border_t_1()
                .border_color(BORDER_SOFT)
                .child(
                    div()
                        .flex_grow(1.0)
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(10.0))
                        .text_color(MUTED)
                        .cursor_pointer()
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.open_advanced_selected(cx)),
                        )
                        .child("高级设置")
                        .child("›"),
                )
                .child(
                    div()
                        .ml(px(16.0))
                        .text_size(px(9.0))
                        .text_color(DANGER)
                        .cursor_pointer()
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.delete_selected(cx)),
                        )
                        .child("删除"),
                ),
        )
    }

    pub(super) fn render_workspace(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tunnels = self.filtered_tunnels(cx);
        let title = self.title();
        let mut list = div()
            .flex()
            .flex_col()
            .flex_grow(1.0)
            .mx(px(10.0))
            .mb(px(10.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(BORDER)
            .bg(APP_BG)
            .overflow_hidden();
        if self.filter == TunnelFilter::Activity && !self.events.is_empty() {
            for event in self.events.iter().rev() {
                list = list.child(
                    div()
                        .flex()
                        .items_center()
                        .min_h(px(70.0))
                        .px(px(22.0))
                        .border_b_1()
                        .border_color(BORDER_SOFT)
                        .child(div().size(px(7.0)).rounded(px(3.5)).mr(px(12.0)).bg(
                            match event.event_type {
                                tunnel_core::event_logger::EventType::Failed => DANGER,
                                tunnel_core::event_logger::EventType::Started
                                | tunnel_core::event_logger::EventType::Reconnected => SUCCESS,
                                _ => MUTED_DARK,
                            },
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_grow(1.0)
                                .child(
                                    div().text_size(px(11.0)).text_color(TEXT).child(
                                        event
                                            .tunnel_name
                                            .clone()
                                            .unwrap_or_else(|| "Tunnel Mate".into()),
                                    ),
                                )
                                .child(
                                    div()
                                        .mt(px(5.0))
                                        .text_size(px(10.0))
                                        .text_color(MUTED)
                                        .child(event.message.clone()),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(9.0))
                                .text_color(MUTED_DARK)
                                .child(event.timestamp.format("%m-%d %H:%M").to_string()),
                        ),
                );
            }
        } else if tunnels.is_empty() {
            list = list.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .h(px(260.0))
                    .child(div().text_size(px(11.0)).text_color(MUTED).child(
                        if self.filter == TunnelFilter::Activity {
                            self.language.pick("暂无活动记录", "No activity yet")
                        } else if self.config.tunnels.is_empty() {
                            self.language.pick("还没有隧道", "No tunnels yet")
                        } else {
                            self.language
                                .pick("当前筛选下没有隧道", "No matching tunnels")
                        },
                    ))
                    .when(self.config.tunnels.is_empty(), |empty| {
                        empty.child(
                            div()
                                .mt(px(7.0))
                                .text_size(px(9.0))
                                .text_color(MUTED_DARK)
                                .child(self.language.pick(
                                    "创建第一个连接，常用设置只需一分钟",
                                    "Create your first connection in about a minute",
                                )),
                        )
                    }),
            );
        } else {
            for tunnel in tunnels {
                list = list.child(self.render_tunnel_row(tunnel, cx));
            }
        }

        let mut center = div()
            .flex()
            .flex_col()
            .flex_grow(1.0)
            .min_w_0()
            .h_full()
            .bg(APP_BG)
            .child(
                div()
                    .h(px(76.0))
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_grow(1.0)
                            .text_size(px(18.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(TEXT)
                            .child(title),
                    )
                    .when(
                        self.filter == TunnelFilter::Activity && !self.events.is_empty(),
                        |header| {
                            header.child(
                                div()
                                    .h(px(34.0))
                                    .px(px(11.0))
                                    .mr(px(8.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(6.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(10.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(SURFACE_HOVER))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.clear_activity(cx)),
                                    )
                                    .child(self.language.pick("清空记录", "Clear")),
                            )
                        },
                    )
                    .when(matches!(self.filter, TunnelFilter::Group(_)), |header| {
                        header
                            .child(
                                div()
                                    .h(px(34.0))
                                    .px(px(11.0))
                                    .mr(px(8.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(6.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(10.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.edit_current_group(cx)),
                                    )
                                    .child(self.language.pick("编辑分组", "Edit group")),
                            )
                            .child(
                                div()
                                    .h(px(34.0))
                                    .px(px(10.0))
                                    .mr(px(8.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(6.0))
                                    .text_size(px(10.0))
                                    .text_color(DANGER)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.delete_current_group(cx)),
                                    )
                                    .child(self.language.pick("删除", "Delete")),
                            )
                    })
                    .when(self.filter != TunnelFilter::Activity, |header| {
                        header
                            .child(div().w(px(246.0)).mr(px(10.0)).child(self.search.clone()))
                            .child(
                                div()
                                    .h(px(40.0))
                                    .px(px(14.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(7.0))
                                    .border_1()
                                    .border_color(PRIMARY)
                                    .bg(PRIMARY)
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(PRIMARY_TEXT)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(PRIMARY_HOVER))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.open_create_sheet(cx)),
                                    )
                                    .child(self.language.pick("新建隧道", "New tunnel")),
                            )
                    }),
            );

        if let Some(error) = &self.load_error {
            center = center.child(
                div()
                    .mx(px(18.0))
                    .mt(px(14.0))
                    .px(px(12.0))
                    .py(px(9.0))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(color(0x65383d))
                    .text_size(px(10.0))
                    .text_color(color(0xee9ba2))
                    .child(error.clone()),
            );
        }
        center = center.child(list);
        div().flex().flex_grow(1.0).min_w_0().h_full().child(center)
    }
}
