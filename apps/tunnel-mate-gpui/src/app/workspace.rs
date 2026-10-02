use super::*;

fn render_activity_row(event: LogEvent) -> gpui::Div {
    let timestamp = event.timestamp.format("%m-%d %H:%M").to_string();
    let tunnel_name = event.tunnel_name.unwrap_or_else(|| "Tunnel Mate".into());
    div()
        .flex()
        .items_center()
        .w_full()
        .h(px(76.0))
        .flex_none()
        .px(px(18.0))
        .border_b_1()
        .border_color(BORDER_SOFT)
        .child(
            div()
                .size(px(7.0))
                .flex_none()
                .rounded(px(4.0))
                .mr(px(14.0))
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
                .flex_1()
                .min_w_0()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(14.0))
                        .text_color(TEXT)
                        .truncate()
                        .child(tunnel_name),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(MUTED)
                        .truncate()
                        .child(event.message),
                ),
        )
        .child(
            div()
                .flex_none()
                .ml(px(14.0))
                .text_size(px(12.0))
                .text_color(MUTED_DARK)
                .child(timestamp),
        )
}

impl TunnelMateApp {
    pub(super) fn render_workspace(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tunnels = self.filtered_tunnels(cx);
        let count = tunnels.len();
        let active = tunnels.iter().filter(|t| self.is_active(&t.id)).count();
        let activity = self.filter == TunnelFilter::Activity;
        let subtitle = if activity {
            self.language
                .pick(
                    "连接状态与最近的操作",
                    "Connection history and recent actions",
                )
                .to_string()
        } else if self.language == Language::Zh {
            format!("{count} 条隧道 · {active} 条运行中")
        } else {
            format!(
                "{count} {} · {active} active",
                if count == 1 { "tunnel" } else { "tunnels" }
            )
        };
        let mut list = div().flex_1().min_h(px(0.0)).mx(px(22.0)).mb(px(20.0));
        if activity && !self.events.is_empty() {
            let events = self.events.clone();
            list = list
                .rounded(px(12.0))
                .border_1()
                .border_color(BORDER_SOFT)
                .bg(SURFACE)
                .child(
                    uniform_list("activity-list", events.len(), move |range, _, _| {
                        events
                            .newest_in(range)
                            .into_iter()
                            .map(render_activity_row)
                            .collect::<Vec<_>>()
                    })
                    .track_scroll(&self.activity_scroll)
                    .size_full(),
                );
        } else if tunnels.is_empty() {
            let first = self.config.tunnels.is_empty();
            let title = if activity {
                self.language.pick("还没有活动记录", "No activity yet")
            } else if first {
                self.language
                    .pick("建立你的第一个连接", "Your first connection starts here")
            } else {
                self.language.pick("没有找到隧道", "No tunnels to show")
            };
            let description = if activity {
                self.language.pick(
                    "连接、断开和重试的记录会显示在这里",
                    "Connections, disconnections and retries will appear here",
                )
            } else if first {
                self.language.pick(
                    "通过 SSH，安全访问数据库、服务与远程网络",
                    "Reach databases, services and remote networks over SSH",
                )
            } else {
                self.language.pick(
                    "试试其他关键词，或切换到全部隧道",
                    "Try another search or return to all tunnels",
                )
            };
            list = list
                .rounded(px(14.0))
                .border_1()
                .border_color(BORDER_SOFT)
                .bg(glass(0x1a2230, 0.35))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(12.0))
                .child(
                    div()
                        .size(px(58.0))
                        .mb(px(6.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(18.0))
                        .bg(SURFACE)
                        .border_1()
                        .border_color(BORDER)
                        .child(
                            icon(if activity {
                                "icons/activity"
                            } else {
                                "icons/tunnels"
                            })
                            .size(px(28.0))
                            .text_color(MUTED),
                        ),
                )
                .child(
                    div()
                        .text_size(px(19.0))
                        .font_weight(FontWeight::MEDIUM)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(MUTED)
                        .child(description),
                )
                .when(!activity && first, |empty| {
                    empty.child(
                        div()
                            .mt(px(12.0))
                            .flex()
                            .gap(px(10.0))
                            .child(
                                primary_button(
                                    "empty-new",
                                    self.language.pick("新建隧道", "New tunnel"),
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.open_create_sheet(cx))),
                            )
                            .child(
                                button(
                                    "empty-ssh-config",
                                    self.language
                                        .pick("从 SSH config 导入", "Import from SSH config"),
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.open_create_sheet(cx);
                                        this.open_primary_ssh_hosts(cx);
                                    },
                                )),
                            ),
                    )
                })
                .when(!activity && !first, |empty| {
                    empty.child(
                        button(
                            "reset-filter",
                            self.language.pick("查看全部隧道", "Show all tunnels"),
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.search.update(cx, |input, cx| input.set_value("", cx));
                            this.set_filter(TunnelFilter::All, cx);
                        })),
                    )
                });
        } else {
            let ids: Vec<String> = tunnels.iter().map(|t| t.id.clone()).collect();
            list = list.child(
                uniform_list(
                    "tunnel-list",
                    count,
                    cx.processor(move |this, range: Range<usize>, _, cx| {
                        range
                            .filter_map(|index| ids.get(index))
                            .filter_map(|id| this.config.tunnels.iter().find(|t| &t.id == id))
                            .map(|tunnel| this.render_tunnel_row(tunnel, cx))
                            .collect::<Vec<_>>()
                    }),
                )
                .track_scroll(&self.tunnel_scroll)
                .size_full(),
            );
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
                    .h(px(96.0))
                    .flex_none()
                    .px(px(24.0))
                    .flex()
                    .items_center()
                    .gap(px(16.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(7.0))
                            .child(
                                div()
                                    .text_size(px(24.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .truncate()
                                    .child(self.title()),
                            )
                            .child(div().text_size(px(12.0)).text_color(MUTED).child(subtitle)),
                    )
                    .when(!activity, |header| {
                        header.child(
                            primary_button(
                                "new-tunnel",
                                self.language.pick("新建隧道", "New tunnel"),
                            )
                            .h(px(38.0))
                            .on_click(cx.listener(|this, _, _, cx| this.open_create_sheet(cx))),
                        )
                    })
                    .when(activity && !self.events.is_empty(), |header| {
                        header.child(
                            button(
                                "clear-activity",
                                self.language.pick("清空记录", "Clear history"),
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.clear_activity(cx))),
                        )
                    }),
            )
            .when(!activity, |center| {
                center.child(
                    div()
                        .px(px(22.0))
                        .pb(px(18.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(div().flex_1().min_w_0().child(self.search.clone()))
                        .when(matches!(self.filter, TunnelFilter::Group(_)), |toolbar| {
                            toolbar
                                .child(
                                    button(
                                        "edit-group",
                                        self.language.pick("编辑分组", "Edit group"),
                                    )
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.edit_current_group(cx)),
                                    ),
                                )
                                .child(
                                    button(
                                        "delete-group",
                                        self.language.pick("删除分组", "Delete group"),
                                    )
                                    .text_color(DANGER)
                                    .on_click(cx.listener(
                                        |this, _, _, cx| this.request_delete_current_group(cx),
                                    )),
                                )
                        }),
                )
            });
        if let Some(error) = &self.load_error {
            center = center.child(
                div()
                    .mx(px(22.0))
                    .mb(px(12.0))
                    .p(px(12.0))
                    .rounded(px(9.0))
                    .border_1()
                    .border_color(DANGER)
                    .bg(glass(0xdc747c, 0.08))
                    .text_size(px(12.0))
                    .text_color(DANGER)
                    .child(error.clone()),
            );
        }
        center.child(list)
    }
}
