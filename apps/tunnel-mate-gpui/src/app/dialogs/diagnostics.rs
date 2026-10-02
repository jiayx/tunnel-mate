use super::super::*;

impl TunnelMateApp {
    pub(crate) fn render_diagnostics(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let diagnostics = self.diagnostics.as_ref().expect("open diagnostics");
        let loading = diagnostics.steps.is_none();
        let retry_id = diagnostics.tunnel_id.clone();
        let edit_id = diagnostics.tunnel_id.clone();
        let mut steps = div()
            .id("diagnostics-scroll")
            .overflow_y_scroll()
            .flex_1()
            .min_h(px(0.0))
            .p(px(22.0))
            .flex()
            .flex_col()
            .gap(px(10.0));
        if loading {
            steps = steps.child(
                div()
                    .h(px(180.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(12.0))
                    .text_color(MUTED)
                    .child(
                        icon("icons/activity")
                            .size(px(28.0))
                            .text_color(PRIMARY_HOVER),
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
                "success" => (SUCCESS, self.language.pick("通过", "Passed")),
                "warning" => (WARNING, self.language.pick("提示", "Notice")),
                _ => (DANGER, self.language.pick("失败", "Failed")),
            };
            steps = steps.child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .p(px(14.0))
                    .rounded(px(10.0))
                    .border_1()
                    .border_color(BORDER_SOFT)
                    .bg(APP_BG)
                    .child(
                        div()
                            .size(px(26.0))
                            .flex_none()
                            .rounded(px(8.0))
                            .bg(SURFACE)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(12.0))
                            .text_color(MUTED)
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
                                            .text_color(TEXT)
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
                                    .text_color(MUTED)
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
                    .p(px(14.0))
                    .rounded(px(10.0))
                    .bg(glass(0xd2a85e, 0.08))
                    .text_size(px(12.0))
                    .text_color(MUTED)
                    .whitespace_normal()
                    .line_height(relative(1.5))
                    .child(
                        div()
                            .mb(px(5.0))
                            .text_color(TEXT)
                            .child(self.language.pick("下一步", "Next step")),
                    )
                    .child(advice),
            );
        }
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080c14bc))
            .child(
                div()
                    .w(px(640.0))
                    .max_w(relative(0.94))
                    .max_h(relative(0.90))
                    .flex()
                    .flex_col()
                    .rounded(px(16.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(SURFACE)
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
                            .border_color(BORDER_SOFT)
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
                                            .text_color(MUTED)
                                            .truncate()
                                            .child(format!(
                                                "{}  ·  {}",
                                                diagnostics.tunnel_name, diagnostics.address
                                            )),
                                    ),
                            )
                            .child(close_button("close-diagnostics").on_click(
                                cx.listener(|this, _, _, cx| this.close_diagnostics(cx)),
                            )),
                    )
                    .child(steps)
                    .child(
                        div()
                            .flex_none()
                            .px(px(22.0))
                            .py(px(14.0))
                            .border_t_1()
                            .border_color(BORDER_SOFT)
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .when(!loading, |footer| {
                                footer
                                    .child(
                                        button(
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
