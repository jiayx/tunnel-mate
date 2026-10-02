use super::super::*;
use super::confirmation::confirmation_panel;

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
        let theme = self.theme;
        confirmation_panel(
            theme,
            self.language.pick("断开并重新连接？", "Reconnect tunnel?"),
            message,
            div()
                .child(
                    button(
                        theme,
                        "cancel-reconnect",
                        self.language.pick("取消", "Cancel"),
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.cancel_save_confirmation(cx))),
                )
                .child(
                    primary_button(
                        theme,
                        "confirm-reconnect",
                        self.language.pick("保存并重连", "Save and reconnect"),
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.confirm_save_and_restart(cx))),
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

        let theme = self.theme;
        confirmation_panel(
            theme,
            self.language.pick("删除隧道？", "Delete tunnel?"),
            message,
            div()
                .child(
                    button(
                        theme,
                        "cancel-delete-tunnel",
                        self.language.pick("取消", "Cancel"),
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.cancel_delete_confirmation(cx))),
                )
                .child(
                    button(
                        theme,
                        "confirm-delete-tunnel",
                        if running {
                            self.language.pick("停止并删除", "Stop and delete")
                        } else {
                            self.language.pick("删除", "Delete")
                        },
                    )
                    .border_color(theme.danger)
                    .bg(theme.danger_bg)
                    .text_color(theme.danger)
                    .on_click(cx.listener(|this, _, _, cx| this.confirm_delete_tunnel(cx))),
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

        let theme = self.theme;
        confirmation_panel(
            theme,
            self.language.pick("删除分组？", "Delete group?"),
            message,
            div()
                .child(
                    button(
                        theme,
                        "cancel-delete-group",
                        self.language.pick("取消", "Cancel"),
                    )
                    .on_click(
                        cx.listener(|this, _, _, cx| this.cancel_group_delete_confirmation(cx)),
                    ),
                )
                .child(
                    button(
                        theme,
                        "confirm-delete-group",
                        self.language.pick("删除分组", "Delete group"),
                    )
                    .border_color(theme.danger)
                    .bg(theme.danger_bg)
                    .text_color(theme.danger)
                    .on_click(cx.listener(|this, _, _, cx| this.confirm_delete_current_group(cx))),
                ),
        )
    }
}
