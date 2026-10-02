use super::super::*;
use super::confirmation::confirmation_panel;

impl TunnelMateApp {
    pub(crate) fn render_import_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        confirmation_panel(
            theme,
            self.language.pick("停止隧道并导入？", "Stop tunnels and import?"),
            self.language.pick(
                "导入备份会停止当前运行中的隧道，然后用所选备份替换现有配置。",
                "Importing a backup stops active tunnels and replaces the current configuration with the selected backup.",
            ),
            div()
                .child(
                    button(theme, "cancel-import", self.language.pick("取消", "Cancel"))
                        .on_click(cx.listener(|this, _, _, cx| this.cancel_import_backup(cx))),
                )
                .child(
                    primary_button(theme, "confirm-import", self.language.pick("继续导入", "Continue"))
                        .on_click(cx.listener(|this, _, _, cx| this.confirm_import_backup(cx))),
                ),
        )
    }

    pub(crate) fn render_group_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.group_form.as_ref().expect("group form");
        let theme = self.theme;
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.backdrop)
            .child(
                div()
                    .w(px(430.0))
                    .max_w(relative(0.94))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.surface)
                    .text_color(theme.text)
                    .child(
                        div()
                            .h(px(56.0))
                            .px(px(18.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(theme.border_soft)
                            .child(div().font_weight(FontWeight::MEDIUM).child(
                                if form.editing_id.is_some() {
                                    self.language.pick("编辑分组", "Edit group")
                                } else {
                                    self.language.pick("新建分组", "New group")
                                },
                            ))
                            .child(
                                close_button(theme, "close-group-form").on_click(
                                    cx.listener(|this, _, _, cx| this.close_group_form(cx)),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .p(px(18.0))
                            .flex()
                            .flex_col()
                            .gap(px(13.0))
                            .when_some(
                                form.validation_error
                                    .clone()
                                    .filter(|_| form.name.read(cx).value().trim().is_empty()),
                                |body, error| {
                                    body.child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.danger)
                                            .child(error),
                                    )
                                },
                            )
                            .child(Self::required_form_field(
                                theme,
                                self.language.pick("名称", "Name"),
                                form.name.clone(),
                            ))
                            .child(Self::form_field(
                                theme,
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
                                        button(
                                            theme,
                                            "cancel-group-form",
                                            self.language.pick("取消", "Cancel"),
                                        )
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.close_group_form(cx)),
                                        ),
                                    )
                                    .child(
                                        primary_button(
                                            theme,
                                            "save-group",
                                            self.language.pick("保存", "Save"),
                                        )
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.save_group(cx)),
                                        ),
                                    ),
                            ),
                    ),
            )
    }
}
