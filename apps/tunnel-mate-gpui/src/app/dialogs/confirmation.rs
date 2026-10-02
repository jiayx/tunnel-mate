use super::super::*;

pub(super) fn confirmation_panel(
    theme: Theme,
    title: &'static str,
    message: impl Into<SharedString>,
    actions: gpui::Div,
) -> impl IntoElement {
    modal_backdrop()
        .flex()
        .items_center()
        .justify_center()
        .bg(theme.backdrop)
        .child(
            div()
                .flex()
                .flex_col()
                .w(px(460.0))
                .max_w(relative(0.94))
                .rounded(px(14.0))
                .border_1()
                .border_color(theme.border)
                .bg(theme.surface)
                .p(px(22.0))
                .child(
                    div()
                        .text_size(px(16.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .child(title),
                )
                .child(
                    div()
                        .mt(px(10.0))
                        .text_size(px(12.0))
                        .line_height(relative(1.55))
                        .text_color(theme.muted)
                        .child(message.into()),
                )
                .child(actions.mt(px(22.0)).flex().justify_end().gap(px(10.0))),
        )
}
