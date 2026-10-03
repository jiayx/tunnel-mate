use super::super::*;

impl TunnelMateApp {
    pub(crate) fn render_group_dropdown(
        &self,
        form: &TunnelForm,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let theme = self.theme;
        let ungrouped = self.language.pick("未分组", "Ungrouped");
        let group_name = form
            .group_id
            .as_ref()
            .and_then(|id| self.config.groups.iter().find(|group| &group.id == id))
            .map(|group| group.name.as_str())
            .unwrap_or(ungrouped);
        let mut options = div()
            .id("tunnel-group-options")
            .size_full()
            .track_scroll(&form.group_menu_scroll)
            .overflow_y_scroll()
            .on_scroll_wheel(stop_scroll_propagation)
            .flex()
            .flex_col()
            .p(px(4.0))
            .pr(px(14.0));
        let groups = std::iter::once((None, ungrouped.to_string())).chain(
            self.config
                .groups
                .iter()
                .map(|group| (Some(group.id.clone()), group.name.clone())),
        );
        for (index, (group_id, name)) in groups.enumerate() {
            let selected = form.group_id == group_id;
            options = options.child(
                div()
                    .id(("group-option", index))
                    .key_context("TunnelButton")
                    .tab_index(0)
                    .focus_visible(|style| style.border_color(theme.primary))
                    .h(px(32.0))
                    .flex_none()
                    .px(px(9.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(rgba(0x00000000))
                    .bg(if selected {
                        theme.selected
                    } else {
                        rgba(0x00000000)
                    })
                    .text_size(px(12.0))
                    .text_color(if selected { theme.text } else { theme.muted })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.selected).text_color(theme.text))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.select_form_group(group_id.clone(), cx);
                    }))
                    .child(div().flex_1().min_w_0().truncate().child(name))
                    .when(selected, |option| option.child("✓")),
            );
        }
        let group_options = div()
            .relative()
            .w(px(220.0))
            .h(px(
                ((self.config.groups.len() + 1) as f32 * 32.0 + 8.0).min(240.0)
            ))
            .rounded(px(8.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .shadow_lg()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(options)
            .child(crate::scrollbar::scrollbar(
                "group-options-scrollbar",
                theme,
                form.group_menu_scroll.clone(),
            ));
        div()
            .relative()
            .w(px(220.0))
            .h(px(40.0))
            .child(
                div()
                    .id("form-group-selector")
                    .key_context("TunnelButton")
                    .tab_index(0)
                    .focus_visible(|style| style.border_color(theme.primary))
                    .w_full()
                    .h_full()
                    .px(px(11.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded(px(7.0))
                    .border_1()
                    .border_color(if form.group_menu_open {
                        theme.primary
                    } else {
                        theme.border
                    })
                    .bg(theme.app_bg)
                    .text_size(px(12.0))
                    .text_color(theme.text)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.surface_hover))
                    .on_click(cx.listener(|this, _, _, cx| {
                        cx.stop_propagation();
                        this.toggle_form_group_menu(cx);
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(group_name.to_string()),
                    )
                    .child(Self::disclosure_chevron(
                        theme,
                        form.group_menu_open,
                        "▴",
                        "▾",
                    )),
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
