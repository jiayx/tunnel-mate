use super::super::*;

impl TunnelMateApp {
    pub(crate) fn render_diagnostics(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let loading = self.diagnostics.as_ref().is_some_and(Vec::is_empty);
        let mut steps = div()
            .flex()
            .flex_col()
            .gap(px(9.0))
            .w_full()
            .max_h(px(520.0))
            .id("diagnostics-scroll")
            .overflow_y_scroll()
            .p(px(18.0));
        if loading {
            steps = steps.child(
                div()
                    .h(px(160.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(10.0))
                    .text_color(MUTED)
                    .child(
                        div()
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(14.0))
                            .border_1()
                            .border_color(PRIMARY)
                            .text_color(PRIMARY)
                            .child("•••"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .child(self.language.pick("正在检查连接…", "Checking connection…")),
                    ),
            );
        }
        for step in self.diagnostics.as_ref().into_iter().flatten() {
            let good = step.status == "success";
            let warning = step.status == "warning";
            steps = steps.child(
                div()
                    .flex()
                    .w_full()
                    .min_w_0()
                    .gap(px(10.0))
                    .p(px(11.0))
                    .rounded(px(8.0))
                    .bg(APP_BG)
                    .child(
                        div()
                            .mt(px(3.0))
                            .size(px(8.0))
                            .flex_none()
                            .rounded(px(4.0))
                            .bg(if good {
                                SUCCESS
                            } else if warning {
                                color(0xd8a94a)
                            } else {
                                color(0xc45b5b)
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_grow(1.0)
                            .min_w_0()
                            .whitespace_normal()
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .whitespace_normal()
                                    .text_size(px(12.0))
                                    .text_color(TEXT)
                                    .child(step.name.clone()),
                            )
                            .child(
                                div()
                                    .mt(px(3.0))
                                    .w_full()
                                    .min_w_0()
                                    .whitespace_normal()
                                    .text_size(px(10.0))
                                    .text_color(MUTED)
                                    .child(step.message.clone()),
                            ),
                    ),
            );
        }
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0db8))
            .child(
                div()
                    .w(px(600.0))
                    .max_h(px(540.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(SURFACE)
                    .overflow_hidden()
                    .child(
                        div()
                            .h(px(58.0))
                            .px(px(18.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(BORDER)
                            .child(
                                div().font_weight(FontWeight::MEDIUM).child(
                                    self.language.pick("连接诊断", "Connection diagnostics"),
                                ),
                            )
                            .child(
                                div()
                                    .size(px(30.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(7.0))
                                    .text_color(MUTED)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.close_diagnostics(cx)),
                                    )
                                    .child("×"),
                            ),
                    )
                    .child(steps),
            )
    }
}
