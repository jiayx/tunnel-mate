use super::super::*;

impl TunnelMateApp {
    pub(crate) fn render_save_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tunnel = self.save_confirmation.as_ref().expect("save confirmation");
        let message = if self.language == Language::Zh {
            format!(
                "“{}”当前正在运行。保存修改会先断开现有连接，然后立即使用新配置重新连接。",
                tunnel.name
            )
        } else {
            format!(
                "“{}” is currently running. Saving will disconnect it and immediately reconnect with the updated configuration.",
                tunnel.name
            )
        };
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
                            .child(self.language.pick("断开并重新连接？", "Reconnect tunnel?")),
                    )
                    .child(
                        div()
                            .mt(px(10.0))
                            .text_size(px(12.0))
                            .line_height(relative(1.55))
                            .text_color(MUTED)
                            .child(message),
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
                                            this.cancel_save_confirmation(cx)
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
                                            this.confirm_save_and_restart(cx)
                                        }),
                                    )
                                    .child(self.language.pick("保存并重连", "Save and reconnect")),
                            ),
                    ),
            )
    }

    pub(crate) fn render_delete_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let id = self
            .delete_confirmation
            .as_ref()
            .expect("delete confirmation");
        let tunnel = self
            .config
            .tunnels
            .iter()
            .find(|tunnel| &tunnel.id == id)
            .expect("tunnel being deleted");
        let running = self.is_active(id);
        let message = match (running, self.language) {
            (true, Language::Zh) => format!(
                "“{}”当前正在运行。删除后会立即断开连接，并且无法恢复。",
                tunnel.name
            ),
            (true, Language::En) => format!(
                "“{}” is running. Deleting it will disconnect immediately and cannot be undone.",
                tunnel.name
            ),
            (false, Language::Zh) => {
                format!("确定删除“{}”吗？此操作无法恢复。", tunnel.name)
            }
            (false, Language::En) => {
                format!("Delete “{}”? This action cannot be undone.", tunnel.name)
            }
        };

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
                            .child(self.language.pick("删除隧道？", "Delete tunnel?")),
                    )
                    .child(
                        div()
                            .mt(px(10.0))
                            .text_size(px(12.0))
                            .line_height(relative(1.55))
                            .text_color(MUTED)
                            .child(message),
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
                                            this.cancel_delete_confirmation(cx)
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
                                    .border_1()
                                    .border_color(DANGER)
                                    .bg(glass(0xdc747c, 0.16))
                                    .text_size(px(12.0))
                                    .text_color(DANGER)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(glass(0xdc747c, 0.24)))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.confirm_delete_tunnel(cx)
                                        }),
                                    )
                                    .child(if running {
                                        self.language.pick("停止并删除", "Stop and delete")
                                    } else {
                                        self.language.pick("删除", "Delete")
                                    }),
                            ),
                    ),
            )
    }

    pub(crate) fn render_group_delete_confirmation(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = self
            .group_delete_confirmation
            .as_ref()
            .expect("group delete confirmation");
        let group = self
            .config
            .groups
            .iter()
            .find(|group| &group.id == id)
            .expect("group being deleted");
        let tunnel_count = self
            .config
            .tunnels
            .iter()
            .filter(|tunnel| tunnel.group_id.as_ref() == Some(id))
            .count();
        let message = if self.language == Language::Zh {
            format!(
                "确定删除分组“{}”吗？分组中的 {} 条隧道不会被删除，将移到未分组。",
                group.name, tunnel_count
            )
        } else {
            format!(
                "Delete group “{}”? Its {} tunnel(s) will not be deleted and will be moved to Ungrouped.",
                group.name, tunnel_count
            )
        };

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
                            .child(self.language.pick("删除分组？", "Delete group?")),
                    )
                    .child(
                        div()
                            .mt(px(10.0))
                            .text_size(px(12.0))
                            .line_height(relative(1.55))
                            .text_color(MUTED)
                            .child(message),
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
                                            this.cancel_group_delete_confirmation(cx)
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
                                    .border_1()
                                    .border_color(DANGER)
                                    .bg(glass(0xdc747c, 0.16))
                                    .text_size(px(12.0))
                                    .text_color(DANGER)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(glass(0xdc747c, 0.24)))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.confirm_delete_current_group(cx)
                                        }),
                                    )
                                    .child(self.language.pick("删除分组", "Delete group")),
                            ),
                    ),
            )
    }
}
