use super::*;

pub(super) fn tunnel_columns(
    identity: impl IntoElement,
    forwarding: impl IntoElement,
    kind: impl IntoElement,
    connection: impl IntoElement,
) -> gpui::Div {
    div()
        .w_full()
        .flex()
        .items_start()
        .gap(px(16.0))
        .px(px(22.0))
        .child(div().flex_1().min_w_0().child(identity))
        .child(div().flex_1().min_w_0().child(forwarding))
        .child(div().w(px(64.0)).flex_none().child(kind))
        .child(div().w(px(108.0)).flex_none().child(connection))
}

fn render_activity_row(theme: Theme, language: Language, event: LogEvent) -> gpui::Div {
    use tunnel_core::event_logger::EventType;
    let (label, tone) = match event.event_type {
        EventType::Created => (language.pick("已创建", "Created"), theme.muted),
        EventType::Updated => (language.pick("配置已更新", "Updated"), theme.muted),
        EventType::Connecting => (language.pick("开始连接", "Connecting"), theme.warning),
        EventType::Started => (language.pick("连接成功", "Connected"), theme.success),
        EventType::Stopped => (language.pick("已断开", "Disconnected"), theme.muted),
        EventType::Restarted => (language.pick("重新连接", "Restarted"), theme.warning),
        EventType::Reconnected => (language.pick("尝试重连", "Reconnecting"), theme.warning),
        EventType::Failed => (language.pick("连接失败", "Failed"), theme.danger),
        EventType::Deleted => (language.pick("已删除", "Deleted"), theme.muted),
    };
    let local_time = event.timestamp.with_timezone(&chrono::Local);
    div()
        .h(px(100.0))
        .w_full()
        .flex()
        .items_start()
        .gap(px(18.0))
        .px(px(24.0))
        .child(
            div()
                .w(px(74.0))
                .flex_none()
                .pt(px(22.0))
                .flex()
                .flex_col()
                .gap(px(5.0))
                .text_size(px(12.0))
                .text_color(theme.muted)
                .child(local_time.format("%H:%M:%S").to_string())
                .child(
                    div()
                        .text_color(theme.muted_dark)
                        .child(local_time.format("%m-%d").to_string()),
                ),
        )
        .child(
            div()
                .relative()
                .w(px(10.0))
                .h_full()
                .flex_none()
                .child(
                    div()
                        .absolute()
                        .left(px(4.0))
                        .top_0()
                        .bottom_0()
                        .w(px(1.0))
                        .bg(theme.border_soft),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(25.0))
                        .size(px(9.0))
                        .rounded(px(5.0))
                        .border_2()
                        .border_color(theme.surface)
                        .bg(tone),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .pt(px(19.0))
                .pb(px(16.0))
                .border_b_1()
                .border_color(theme.border_soft)
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(14.0))
                                .font_weight(FontWeight::MEDIUM)
                                .child(event.tunnel_name.unwrap_or_else(|| "Tunnel Mate".into())),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_size(px(12.0))
                                .text_color(tone)
                                .child(label),
                        ),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.muted)
                        .line_height(relative(1.5))
                        .line_clamp(2)
                        .child(language.runtime_message(&event.message)),
                ),
        )
}

impl TunnelMateApp {
    pub(super) fn render_workspace(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let activity = self.filter == TunnelFilter::Activity;
        let tunnels = self.filtered_tunnels(cx);
        let selected = tunnels
            .iter()
            .find(|tunnel| self.selected_tunnel.as_deref() == Some(tunnel.id.as_str()))
            .copied();
        let scope = self
            .config
            .tunnels
            .iter()
            .filter(|tunnel| self.filter.includes(tunnel))
            .collect::<Vec<_>>();
        let active = scope
            .iter()
            .filter(|tunnel| self.is_active(&tunnel.id))
            .count();
        let failed = scope
            .iter()
            .filter(|tunnel| self.status(&tunnel.id) == TunnelStatus::Failed)
            .count();
        let query_empty = self.search.read(cx).value().trim().is_empty();
        let mut list = div().relative().flex_1().min_h(px(0.0));
        if activity && !self.events.is_empty() {
            let events = self.events.clone();
            let language = self.language;
            list = list
                .child(
                    uniform_list("activity-list", events.len(), move |range, _, _| {
                        events
                            .newest_in(range)
                            .into_iter()
                            .map(|event| render_activity_row(theme, language, event))
                            .collect::<Vec<_>>()
                    })
                    .track_scroll(&self.activity_scroll)
                    .size_full(),
                )
                .child(crate::scrollbar::scrollbar(
                    "activity-scrollbar",
                    theme,
                    self.activity_scroll.0.borrow().base_handle.clone(),
                ));
        } else if !activity && !tunnels.is_empty() {
            let ids = tunnels
                .iter()
                .map(|tunnel| tunnel.id.clone())
                .collect::<Vec<_>>();
            list = list
                .child(
                    uniform_list(
                        "tunnel-list",
                        ids.len(),
                        cx.processor(move |this, range: Range<usize>, _, cx| {
                            range
                                .filter_map(|index| ids.get(index))
                                .filter_map(|id| {
                                    this.config.tunnels.iter().find(|tunnel| &tunnel.id == id)
                                })
                                .map(|tunnel| this.render_tunnel_row(tunnel, cx))
                                .collect::<Vec<_>>()
                        }),
                    )
                    .track_scroll(&self.tunnel_scroll)
                    .size_full(),
                )
                .child(crate::scrollbar::scrollbar(
                    "tunnel-scrollbar",
                    theme,
                    self.tunnel_scroll.0.borrow().base_handle.clone(),
                ));
        } else {
            let first = self.config.tunnels.is_empty();
            let (glyph, title, description) = if activity {
                (
                    "icons/history",
                    self.language.pick("还没有活动记录", "No activity yet"),
                    self.language.pick(
                        "连接和配置变更会按时间显示在这里",
                        "Connections and configuration changes will appear here",
                    ),
                )
            } else if first {
                (
                    "icons/tunnels",
                    self.language
                        .pick("添加你的第一条隧道", "Add your first tunnel"),
                    self.language.pick(
                        "连接 SSH 主机，访问远程数据库和服务",
                        "Connect to an SSH host to access remote databases and services",
                    ),
                )
            } else if query_empty && self.status_filter == TunnelStatusFilter::Active {
                (
                    "icons/power",
                    self.language
                        .pick("当前没有运行中的隧道", "No active tunnels"),
                    self.language.pick(
                        "切换到全部，选择一条隧道连接",
                        "Switch to All and choose a tunnel to connect",
                    ),
                )
            } else if query_empty && self.status_filter == TunnelStatusFilter::Failed {
                (
                    "icons/check",
                    self.language
                        .pick("当前没有失败的连接", "No failed connections"),
                    self.language.pick(
                        "连接失败的隧道会显示在这里",
                        "Tunnels with connection failures will appear here",
                    ),
                )
            } else {
                (
                    "icons/tunnels",
                    self.language.pick("没有匹配的隧道", "No matching tunnels"),
                    self.language.pick(
                        "试试其他关键词，或清除筛选条件",
                        "Try another search or clear the filters",
                    ),
                )
            };
            list = list
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .px(px(24.0))
                .gap(px(14.0))
                .child(
                    icon(theme, glyph)
                        .size(px(36.0))
                        .text_color(theme.muted_dark),
                )
                .child(
                    div()
                        .text_size(px(20.0))
                        .font_weight(FontWeight::MEDIUM)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.muted)
                        .child(description),
                )
                .when(!activity && first, |empty| {
                    empty.child(
                        div()
                            .mt(px(8.0))
                            .flex()
                            .gap(px(10.0))
                            .child(
                                primary_button(
                                    theme,
                                    "empty-new",
                                    self.language.pick("新建隧道", "New tunnel"),
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.open_create_sheet(cx))),
                            )
                            .child(
                                button(
                                    theme,
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
                            theme,
                            "reset-filter",
                            self.language.pick("显示全部", "Show all"),
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.status_filter = TunnelStatusFilter::All;
                            this.search.update(cx, |input, cx| input.set_value("", cx));
                            this.set_filter(this.filter.clone(), cx);
                        })),
                    )
                });
        }
        let header = div()
            .h(px(72.0))
            .flex_none()
            .px(px(22.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(22.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.title()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(13.0))
                            .text_color(theme.muted)
                            .child(
                                if activity {
                                    self.events.len()
                                } else {
                                    scope.len()
                                }
                                .to_string(),
                            ),
                    ),
            )
            .when(matches!(self.filter, TunnelFilter::Group(_)), |header| {
                header
                    .child(
                        button(
                            theme,
                            "edit-group",
                            self.language.pick("编辑分组", "Edit group"),
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.edit_current_group(cx))),
                    )
                    .child(
                        button(
                            theme,
                            "delete-group",
                            self.language.pick("删除分组", "Delete group"),
                        )
                        .text_color(theme.danger)
                        .on_click(
                            cx.listener(|this, _, _, cx| this.request_delete_current_group(cx)),
                        ),
                    )
            })
            .when(!activity, |header| {
                header.child(
                    primary_button(
                        theme,
                        "new-tunnel",
                        self.language.pick("新建隧道", "New tunnel"),
                    )
                    .h(px(34.0))
                    .flex_row_reverse()
                    .child(
                        icon(theme, "icons/plus")
                            .size(px(14.0))
                            .text_color(theme.primary_text),
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.open_create_sheet(cx))),
                )
            })
            .when(activity && !self.events.is_empty(), |header| {
                header.child(
                    button(
                        theme,
                        "clear-activity",
                        self.language.pick("清空记录", "Clear history"),
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.clear_activity(cx))),
                )
            });
        let mut center = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h(px(0.0))
            .h_full()
            .bg(theme.surface)
            .child(header);
        if !activity {
            let filters = [
                (
                    TunnelStatusFilter::All,
                    self.language.pick("全部", "All"),
                    None,
                    scope.len(),
                ),
                (
                    TunnelStatusFilter::Active,
                    self.language.pick("运行中", "Active"),
                    Some("icons/power"),
                    active,
                ),
                (
                    TunnelStatusFilter::Failed,
                    self.language.pick("失败", "Failed"),
                    Some("icons/alert"),
                    failed,
                ),
            ];
            center =
                center.child(
                    div()
                        .h(px(60.0))
                        .flex_none()
                        .px(px(22.0))
                        .pb(px(16.0))
                        .flex()
                        .items_center()
                        .gap(px(16.0))
                        .child(div().flex().gap(px(3.0)).children(
                            filters.into_iter().enumerate().map(
                                |(index, (filter, label, glyph, count))| {
                                    let selected = self.status_filter == filter;
                                    button(
                                        theme,
                                        ("status-filter", index),
                                        format!("{label}  {count}"),
                                    )
                                    .h(px(34.0))
                                    .flex_row_reverse()
                                    .px(px(10.0))
                                    .rounded(px(6.0))
                                    .bg(if selected {
                                        theme.selected
                                    } else {
                                        theme.surface
                                    })
                                    .border_color(if selected {
                                        theme.selected_border
                                    } else {
                                        rgba(0x00000000)
                                    })
                                    .text_color(if selected { theme.accent } else { theme.muted })
                                    .when_some(glyph, |button, glyph| {
                                        button.child(icon(theme, glyph).size(px(13.0)).text_color(
                                            if selected { theme.accent } else { theme.muted },
                                        ))
                                    })
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.status_filter = filter;
                                            this.set_filter(this.filter.clone(), cx);
                                        },
                                    ))
                                },
                            ),
                        ))
                        .child(div().flex_1().min_w_0().child(self.search.clone())),
                );
            center = center.child(
                tunnel_columns(
                    div()
                        .truncate()
                        .child(self.language.pick("隧道 / SSH 主机", "TUNNEL / SSH HOST")),
                    div()
                        .truncate()
                        .child(self.language.pick("监听 → 目标", "LISTEN → TARGET")),
                    div().child(self.language.pick("类型", "TYPE")),
                    div()
                        .text_center()
                        .child(self.language.pick("连接", "CONNECTION")),
                )
                .h(px(34.0))
                .flex_none()
                .items_center()
                .bg(theme.app_bg)
                .border_y_1()
                .border_color(theme.border_soft)
                .text_size(px(11.0))
                .text_color(theme.muted),
            );
        } else {
            center = center.child(
                div()
                    .flex_none()
                    .px(px(24.0))
                    .pb(px(16.0))
                    .border_b_1()
                    .border_color(theme.border_soft)
                    .text_size(px(12.0))
                    .text_color(theme.muted)
                    .child(self.language.pick(
                        "连接与配置变更 · 最新记录在前",
                        "Connections and configuration changes · Newest first",
                    )),
            );
        }
        if let Some(error) = &self.load_error {
            center = center.child(
                div()
                    .px(px(22.0))
                    .py(px(12.0))
                    .bg(theme.danger_bg)
                    .text_size(px(12.0))
                    .text_color(theme.danger)
                    .child(error.clone()),
            );
        }
        center = center.child(list);
        if !activity {
            let mut footer = div()
                .h(px(58.0))
                .flex_none()
                .px(px(22.0))
                .flex()
                .items_center()
                .gap(px(12.0))
                .border_t_1()
                .border_color(theme.border_soft)
                .bg(theme.surface);
            if let Some(tunnel) = selected {
                let diagnose_id = tunnel.id.clone();
                let edit_id = tunnel.id.clone();
                footer = footer
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(12.0))
                                    .child(tunnel.name.clone()),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(11.0))
                                    .text_color(theme.muted)
                                    .child(Self::route(tunnel)),
                            ),
                    )
                    .child(
                        button(
                            theme,
                            "diagnose-selected",
                            self.language.pick("诊断连接", "Diagnose"),
                        )
                        .flex_row_reverse()
                        .child(
                            icon(theme, "icons/activity")
                                .size(px(14.0))
                                .text_color(theme.muted),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.run_tunnel_diagnostics(diagnose_id.clone(), cx)
                        })),
                    )
                    .child(
                        button(theme, "edit-selected", self.language.pick("编辑", "Edit"))
                            .flex_row_reverse()
                            .child(
                                icon(theme, "icons/edit")
                                    .size(px(14.0))
                                    .text_color(theme.muted),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.edit_tunnel(edit_id.clone(), cx)
                            })),
                    );
            } else {
                footer = footer.child(div().text_size(px(12.0)).text_color(theme.muted).child(
                    if self.language == Language::Zh {
                        format!("{} 条隧道", tunnels.len())
                    } else {
                        format!("{} tunnels", tunnels.len())
                    },
                ));
            }
            center = center.child(footer);
        }
        center
    }
}
