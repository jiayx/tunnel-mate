use super::super::*;

impl TunnelMateApp {
    pub(crate) fn render_group_dropdown(
        &self,
        form: &TunnelForm,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let group_name = form
            .group_id
            .as_ref()
            .and_then(|id| self.config.groups.iter().find(|group| &group.id == id))
            .map(|group| group.name.as_str())
            .unwrap_or(self.language.pick("未分组", "Ungrouped"));
        let selected_group_id = form.group_id.clone();
        let mut group_options = div()
            .flex()
            .flex_col()
            .w(px(220.0))
            .max_h(px(240.0))
            .id("tunnel-group-options")
            .overflow_y_scroll()
            .on_scroll_wheel(stop_scroll_propagation)
            .p(px(4.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(BORDER)
            .bg(color(0x111722))
            .shadow_lg()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation());
        let ungrouped_selected = selected_group_id.is_none();
        group_options = group_options.child(
            div()
                .id("group-option-ungrouped")
                .h(px(32.0))
                .px(px(9.0))
                .flex()
                .items_center()
                .justify_between()
                .rounded(px(6.0))
                .border_1()
                .border_color(if ungrouped_selected {
                    glass(0x075bea, 0.48)
                } else {
                    rgba(0x00000000)
                })
                .bg(if ungrouped_selected {
                    glass(0x075bea, 0.18)
                } else {
                    rgba(0x00000000)
                })
                .text_size(px(10.0))
                .text_color(if ungrouped_selected { TEXT } else { MUTED })
                .cursor_pointer()
                .hover(|style| {
                    style
                        .bg(if ungrouped_selected {
                            glass(0x075bea, 0.18)
                        } else {
                            glass(0x075bea, 0.10)
                        })
                        .border_color(glass(0x075bea, 0.36))
                        .text_color(TEXT)
                })
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        cx.stop_propagation();
                        this.select_form_group(None, cx);
                    }),
                )
                .child(self.language.pick("未分组", "Ungrouped"))
                .child(if ungrouped_selected { "✓" } else { "" }),
        );
        for (index, group) in self.config.groups.iter().enumerate() {
            let group_id = group.id.clone();
            let selected = selected_group_id.as_ref() == Some(&group.id);
            group_options = group_options.child(
                div()
                    .id(("group-option", index))
                    .h(px(32.0))
                    .px(px(9.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(if selected {
                        glass(0x075bea, 0.48)
                    } else {
                        rgba(0x00000000)
                    })
                    .bg(if selected {
                        glass(0x075bea, 0.18)
                    } else {
                        rgba(0x00000000)
                    })
                    .text_size(px(10.0))
                    .text_color(if selected { TEXT } else { MUTED })
                    .cursor_pointer()
                    .hover(|style| {
                        style
                            .bg(if selected {
                                glass(0x075bea, 0.18)
                            } else {
                                glass(0x075bea, 0.10)
                            })
                            .border_color(glass(0x075bea, 0.36))
                            .text_color(TEXT)
                    })
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.select_form_group(Some(group_id.clone()), cx)
                        }),
                    )
                    .child(group.name.clone())
                    .child(if selected { "✓" } else { "" }),
            );
        }
        div()
            .relative()
            .w(px(220.0))
            .h(px(34.0))
            .child(
                div()
                    .w_full()
                    .h_full()
                    .px(px(11.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(px(7.0))
                    .border_1()
                    .border_color(if form.group_menu_open {
                        PRIMARY
                    } else {
                        BORDER
                    })
                    .bg(APP_BG)
                    .text_size(px(10.0))
                    .text_color(TEXT)
                    .cursor_pointer()
                    .hover(|style| style.bg(SURFACE_HOVER))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            cx.stop_propagation();
                            this.toggle_form_group_menu(cx);
                        }),
                    )
                    .child(group_name.to_string())
                    .child(Self::disclosure_chevron(form.group_menu_open, "▴", "▾")),
            )
            .when(form.group_menu_open, |container| {
                container.child(
                    deferred(
                        anchored()
                            .anchor(Anchor::TopRight)
                            .position(point(px(220.0), px(4.0)))
                            .position_mode(AnchoredPositionMode::Local)
                            .snap_to_window_with_margin(px(8.0))
                            .child(group_options),
                    )
                    .priority(2),
                )
            })
    }
}
