use super::super::*;

impl TunnelMateApp {
    pub(crate) fn render_about(&self, cx: &mut Context<Self>) -> impl IntoElement {
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0dd4))
            .child(
                div()
                    .w(px(360.0))
                    .p(px(26.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .rounded(px(16.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(SURFACE)
                    .shadow_lg()
                    .child(img(self.logo.clone()).size(px(72.0)).rounded(px(17.0)))
                    .child(
                        div()
                            .mt(px(16.0))
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(TEXT)
                            .child("Tunnel Mate"),
                    )
                    .child(
                        div()
                            .mt(px(5.0))
                            .text_size(px(11.0))
                            .text_color(MUTED)
                            .child(format!("Version {}", env!("CARGO_PKG_VERSION"))),
                    )
                    .child(
                        div()
                            .mt(px(13.0))
                            .text_size(px(11.0))
                            .text_color(MUTED)
                            .child(self.language.pick(
                                "简洁、可靠的 SSH 隧道管理工具",
                                "A focused, reliable SSH tunnel manager",
                            )),
                    )
                    .child(
                        div()
                            .mt(px(22.0))
                            .h(px(34.0))
                            .px(px(18.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(8.0))
                            .bg(PRIMARY)
                            .text_size(px(11.0))
                            .text_color(PRIMARY_TEXT)
                            .cursor_pointer()
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.close_about(cx)),
                            )
                            .child(self.language.pick("好", "OK")),
                    ),
            )
    }
}
