use super::super::*;

impl TunnelMateApp {
    pub(crate) fn render_import_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0dd6))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(440.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(color(0x171d29))
                    .p(px(22.0))
                    .child(
                        div()
                            .text_size(px(16.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(TEXT)
                            .child(self.language.pick(
                                "停止隧道并导入？",
                                "Stop tunnels and import?",
                            )),
                    )
                    .child(
                        div()
                            .mt(px(10.0))
                            .text_size(px(12.0))
                            .line_height(relative(1.55))
                            .text_color(MUTED)
                            .child(self.language.pick(
                                "导入备份会停止当前运行中的隧道，然后用所选备份替换现有配置。",
                                "Importing a backup stops active tunnels and replaces the current configuration with the selected backup.",
                            )),
                    )
                    .child(
                        div()
                            .mt(px(22.0))
                            .flex()
                            .justify_end()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .h(px(36.0))
                                    .px(px(15.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.0))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(12.0))
                                    .text_color(TEXT)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.cancel_import_backup(cx)
                                        }),
                                    )
                                    .child(self.language.pick("取消", "Cancel")),
                            )
                            .child(
                                div()
                                    .h(px(36.0))
                                    .px(px(15.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.0))
                                    .bg(PRIMARY)
                                    .text_size(px(12.0))
                                    .text_color(PRIMARY_TEXT)
                                    .cursor_pointer()
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.confirm_import_backup(cx)
                                        }),
                                    )
                                    .child(self.language.pick("继续导入", "Continue")),
                            ),
                    ),
            )
    }

    pub(crate) fn render_group_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.group_form.as_ref().expect("group form");
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x080a0db8))
            .child(
                div()
                    .w(px(430.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(BORDER)
                    .bg(SURFACE)
                    .child(
                        div()
                            .h(px(56.0))
                            .px(px(18.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(BORDER)
                            .child(div().font_weight(FontWeight::MEDIUM).child(
                                if form.editing_id.is_some() {
                                    self.language.pick("编辑分组", "Edit group")
                                } else {
                                    self.language.pick("新建分组", "New group")
                                },
                            ))
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
                                        cx.listener(|this, _, _, cx| this.close_group_form(cx)),
                                    )
                                    .child("×"),
                            ),
                    )
                    .child(
                        div()
                            .p(px(18.0))
                            .flex()
                            .flex_col()
                            .gap(px(13.0))
                            .child(Self::required_form_field(
                                self.language.pick("名称", "Name"),
                                form.name.clone(),
                            ))
                            .child(Self::form_field(
                                self.language.pick("说明", "Description"),
                                form.description.clone(),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap(px(8.0))
                                    .mt(px(5.0))
                                    .child(
                                        div()
                                            .h(px(34.0))
                                            .px(px(12.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(8.0))
                                            .border_1()
                                            .border_color(BORDER)
                                            .text_size(px(11.0))
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    this.close_group_form(cx)
                                                }),
                                            )
                                            .child(self.language.pick("取消", "Cancel")),
                                    )
                                    .child(
                                        div()
                                            .h(px(34.0))
                                            .px(px(13.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(8.0))
                                            .bg(PRIMARY)
                                            .text_size(px(11.0))
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| this.save_group(cx)),
                                            )
                                            .child(self.language.pick("保存", "Save")),
                                    ),
                            ),
                    ),
            )
    }
}
