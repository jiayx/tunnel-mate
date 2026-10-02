use super::super::*;
use crate::scrollbar::scrollbar;

impl TunnelMateApp {
    pub(crate) fn render_diagnostics(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let diagnostics = self.diagnostics.as_ref().expect("open diagnostics");
        let loading = diagnostics.steps.is_none();
        let retry_id = diagnostics.tunnel_id.clone();
        let edit_id = diagnostics.tunnel_id.clone();
        let mut steps = div()
            .id("diagnostics-scroll")
            .overflow_y_scroll()
            .track_scroll(&diagnostics.scroll)
            .h_full()
            .px(px(22.0))
            .py(px(10.0))
            .flex()
            .flex_col();
        if loading {
            steps = steps.child(
                div()
                    .h(px(180.0))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(12.0))
                    .text_color(theme.muted)
                    .child(
                        icon(theme, "icons/activity")
                            .size(px(28.0))
                            .text_color(theme.primary_hover),
                    )
                    .child(self.language.pick("正在检查连接…", "Checking connection…"))
                    .child(div().text_size(px(12.0)).child(self.language.pick(
                        "关闭此窗口会取消本次检查",
                        "Closing this window cancels the check",
                    ))),
            );
        }
        for (index, step) in diagnostics.steps.iter().flatten().enumerate() {
            let (tone, label) = match step.status.as_str() {
                "success" => (theme.success, self.language.pick("通过", "Passed")),
                "warning" => (theme.warning, self.language.pick("提示", "Notice")),
                _ => (theme.danger, self.language.pick("失败", "Failed")),
            };
            steps = steps.child(
                div()
                    .flex()
                    .flex_none()
                    .gap(px(12.0))
                    .py(px(14.0))
                    .when(index > 0, |row| {
                        row.border_t_1().border_color(theme.border_soft)
                    })
                    .child(
                        div()
                            .size(px(26.0))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(12.0))
                            .text_color(theme.muted)
                            .child(format!("{:02}", index + 1)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .gap(px(10.0))
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .text_color(theme.text)
                                            .child(step.name.clone()),
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
                                    .whitespace_normal()
                                    .line_height(relative(1.5))
                                    .child(step.message.clone()),
                            ),
                    ),
            );
        }
        if let Some(failed) = diagnostics
            .steps
            .iter()
            .flatten()
            .find(|step| step.status == "error")
        {
            let advice = if failed.name.contains("DNS") {
                self.language.pick(
                    "核对主机名，并检查当前网络能否解析该地址。",
                    "Check the hostname and whether your network can resolve it.",
                )
            } else if failed.name.contains("TCP") {
                self.language.pick("检查 SSH 服务是否运行、端口是否正确，以及防火墙是否允许连接。", "Check that the SSH service is running, the port is correct, and the firewall allows the connection.")
            } else {
                self.language.pick("打开连接设置核对端口、凭据和跳板机；也可以复制报告继续排查。", "Review ports, credentials and the jump host in connection settings, or copy the report for further troubleshooting.")
            };
            steps = steps.child(
                div()
                    .py(px(16.0))
                    .flex_none()
                    .border_t_1()
                    .border_color(theme.border_soft)
                    .text_size(px(12.0))
                    .text_color(theme.muted)
                    .whitespace_normal()
                    .line_height(relative(1.5))
                    .child(
                        div()
                            .mb(px(5.0))
                            .text_color(theme.text)
                            .child(self.language.pick("下一步", "Next step")),
                    )
                    .child(advice),
            );
        }
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.backdrop)
            .child(
                div()
                    .w(px(640.0))
                    .max_w(relative(0.94))
                    .h(px(if loading { 320.0 } else { 580.0 }))
                    .max_h(relative(0.90))
                    .flex()
                    .flex_col()
                    .rounded(px(16.0))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.surface)
                    .shadow_lg()
                    .child(
                        div()
                            .px(px(22.0))
                            .py(px(18.0))
                            .flex_none()
                            .flex()
                            .items_start()
                            .justify_between()
                            .border_b_1()
                            .border_color(theme.border_soft)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(px(18.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(
                                                self.language
                                                    .pick("连接诊断", "Connection diagnostics"),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.muted)
                                            .truncate()
                                            .child(format!(
                                                "{}  ·  {}",
                                                diagnostics.tunnel_name, diagnostics.address
                                            )),
                                    ),
                            )
                            .child(close_button(theme, "close-diagnostics").on_click(
                                cx.listener(|this, _, _, cx| this.close_diagnostics(cx)),
                            )),
                    )
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_h(px(0.0))
                            .child(steps)
                            .child(scrollbar(
                                "diagnostics-scrollbar",
                                theme,
                                diagnostics.scroll.clone(),
                            )),
                    )
                    .child(
                        div()
                            .flex_none()
                            .px(px(22.0))
                            .py(px(14.0))
                            .border_t_1()
                            .border_color(theme.border_soft)
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .when(!loading, |footer| {
                                footer
                                    .child(
                                        button(
                                            theme,
                                            "copy-diagnostics",
                                            if diagnostics.copied {
                                                self.language.pick("已复制", "Copied")
                                            } else {
                                                self.language.pick("复制报告", "Copy report")
                                            },
                                        )
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.copy_diagnostics(cx)),
                                        ),
                                    )
                                    .child(
                                        button(
                                            theme,
                                            "edit-diagnostic-tunnel",
                                            self.language.pick("编辑连接", "Edit connection"),
                                        )
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.close_diagnostics(cx);
                                                this.edit_tunnel(edit_id.clone(), cx);
                                            }),
                                        ),
                                    )
                            })
                            .child(div().flex_1())
                            .when(!loading, |footer| {
                                footer.child(
                                    primary_button(
                                        theme,
                                        "retry-diagnostics",
                                        self.language.pick("重新检查", "Check again"),
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.run_tunnel_diagnostics(retry_id.clone(), cx)
                                        },
                                    )),
                                )
                            })
                            .when(loading, |footer| {
                                footer.child(
                                    button(
                                        theme,
                                        "cancel-diagnostics",
                                        self.language.pick("取消检查", "Cancel check"),
                                    )
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.close_diagnostics(cx)),
                                    ),
                                )
                            }),
                    ),
            )
    }
}
