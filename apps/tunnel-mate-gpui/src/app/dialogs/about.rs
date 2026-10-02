use super::super::*;

impl TunnelMateApp {
    pub(crate) fn render_about(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.backdrop)
            .child(
                div()
                    .w(px(360.0))
                    .max_w(relative(0.94))
                    .p(px(26.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .rounded(px(16.0))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.surface)
                    .shadow_lg()
                    .child(img(self.logo.clone()).size(px(72.0)).rounded(px(17.0)))
                    .child(
                        div()
                            .mt(px(16.0))
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .child("Tunnel Mate"),
                    )
                    .child(
                        div()
                            .mt(px(5.0))
                            .text_size(px(12.0))
                            .text_color(theme.muted)
                            .child(format!("Version {}", env!("CARGO_PKG_VERSION"))),
                    )
                    .child(
                        div()
                            .mt(px(13.0))
                            .text_size(px(12.0))
                            .text_color(theme.muted)
                            .child(self.language.pick(
                                "简洁、可靠的 SSH 隧道管理工具",
                                "A focused, reliable SSH tunnel manager",
                            )),
                    )
                    .child(
                        primary_button(theme, "close-about", self.language.pick("好", "OK"))
                            .mt(px(22.0))
                            .on_click(cx.listener(|this, _, _, cx| this.close_about(cx))),
                    ),
            )
    }
}
