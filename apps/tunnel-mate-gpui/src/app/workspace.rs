use super::*;

fn render_activity_row(event: LogEvent) -> gpui::Div {
    let timestamp = event.timestamp.format("%m-%d %H:%M").to_string();
    let tunnel_name = event.tunnel_name.unwrap_or_else(|| "Tunnel Mate".into());
    div()
        .flex()
        .items_center()
        .w_full()
        .h(px(70.0))
        .flex_none()
        .px(px(22.0))
        .border_b_1()
        .border_color(BORDER_SOFT)
        .child(
            div()
                .size(px(7.0))
                .flex_none()
                .rounded(px(3.5))
                .mr(px(12.0))
                .bg(match event.event_type {
                    tunnel_core::event_logger::EventType::Failed => DANGER,
                    tunnel_core::event_logger::EventType::Started
                    | tunnel_core::event_logger::EventType::Reconnected => SUCCESS,
                    _ => MUTED_DARK,
                }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_grow(1.0)
                .min_w_0()
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(TEXT)
                        .truncate()
                        .child(tunnel_name),
                )
                .child(
                    div()
                        .mt(px(5.0))
                        .text_size(px(10.0))
                        .text_color(MUTED)
                        .truncate()
                        .child(event.message),
                ),
        )
        .child(
            div()
                .flex_none()
                .ml(px(12.0))
                .text_size(px(9.0))
                .text_color(MUTED_DARK)
                .child(timestamp),
        )
}

impl TunnelMateApp {
    pub(super) fn render_workspace(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tunnels = self.filtered_tunnels(cx);
        let title = self.title();
        let mut list = div()
            .id("workspace-list-scroll")
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.0))
            .mx(px(10.0))
            .mb(px(10.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(BORDER)
            .bg(APP_BG)
            .overflow_hidden();
        if self.filter == TunnelFilter::Activity && !self.events.is_empty() {
            let events = self.events.clone();
            let event_count = events.len();
            list = list.child(
                uniform_list("activity-list", event_count, move |range, _, _| {
                    events
                        .newest_in(range)
                        .into_iter()
                        .map(render_activity_row)
                        .collect::<Vec<_>>()
                })
                .track_scroll(&self.activity_scroll)
                .flex_1()
                .w_full()
                .min_h(px(0.0)),
            );
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
            list = list.overflow_y_scroll();
            for tunnel in tunnels {
                list = list.child(self.render_tunnel_row(tunnel, cx));
            }
        }

        let mut center = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h(px(0.0))
            .h_full()
            .bg(APP_BG)
            .child(
                div()
                    .h(px(76.0))
                    .flex_none()
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
                                        cx.listener(|this, _, _, cx| {
                                            this.request_delete_current_group(cx)
                                        }),
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
        div()
            .flex()
            .flex_1()
            .min_w_0()
            .min_h(px(0.0))
            .h_full()
            .child(center)
    }
}
