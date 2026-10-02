use super::super::*;
use crate::scrollbar::scrollbar;

impl TunnelMateApp {
    pub(crate) fn render_auth_prompt(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let (title, body, actions, height) = match self.auth_prompt.as_ref().expect("auth prompt") {
            AuthPrompt::HostKey {
                issue,
                host,
                port,
                fingerprint,
                saved_fingerprints,
                confirm_replace,
                ..
            } => {
                let is_confirming = *confirm_replace;
                let title = if is_confirming {
                    self.language.pick("确认更新主机密钥？", "Confirm host key update?")
                } else {
                    match issue {
                        HostKeyIssue::Unknown => self.language.pick("信任 SSH 主机密钥", "Trust SSH host key"),
                        HostKeyIssue::Changed => self.language.pick("SSH 主机密钥已变化", "SSH host key changed"),
                        HostKeyIssue::Revoked => self.language.pick("SSH 主机密钥已撤销", "SSH host key revoked"),
                    }
                };
                let body = div()
                    .flex()
                    .flex_col()
                    .gap(px(18.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .line_height(relative(1.5))
                            .text_color(theme.muted)
                            .child(match (issue, is_confirming, self.language) {
                                (HostKeyIssue::Unknown, _, Language::Zh) => format!("首次连接 {host}:{port}，请核对指纹后再信任。"),
                                (HostKeyIssue::Unknown, _, Language::En) => format!("First connection to {host}:{port}. Verify the fingerprint before trusting it."),
                                (HostKeyIssue::Changed, false, Language::Zh) => format!("{host}:{port} 的主机密钥与已保存记录不一致。请通过可信渠道确认服务器确实更换了密钥，再继续更新。"),
                                (HostKeyIssue::Changed, false, Language::En) => format!("The key for {host}:{port} differs from the saved key. Independently verify that the server key really changed before updating it."),
                                (HostKeyIssue::Changed, true, Language::Zh) => "继续会替换 known_hosts 中的旧记录并立即重连。若下面的新指纹未经核实，请取消操作。".to_string(),
                                (HostKeyIssue::Changed, true, Language::En) => "Continuing replaces the old known_hosts entry and reconnects immediately. Cancel if you have not verified the new fingerprint.".to_string(),
                                (HostKeyIssue::Revoked, _, Language::Zh) => format!("{host}:{port} 的密钥在 known_hosts 中被标记为已撤销。请联系服务器管理员核实，应用已阻止连接。"),
                                (HostKeyIssue::Revoked, _, Language::En) => format!("The key for {host}:{port} is marked as revoked in known_hosts. Contact the server administrator; the connection was blocked."),
                            }),
                    )
                    .when(*issue == HostKeyIssue::Changed && !saved_fingerprints.is_empty(), |column| {
                        column.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(6.0))
                                .text_size(px(12.0))
                                .text_color(theme.muted)
                                .child(self.language.pick("已保存的指纹", "Saved fingerprint"))
                                .children(saved_fingerprints.iter().map(|fingerprint| div().child(fingerprint.clone()))),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .text_size(px(12.0))
                            .child(div().text_color(theme.muted).child(
                                if *issue == HostKeyIssue::Changed {
                                    self.language.pick("服务器的新指纹", "New server fingerprint")
                                } else {
                                    self.language.pick("服务器指纹", "Server fingerprint")
                                },
                            ))
                            .child(div().text_color(theme.text).child(fingerprint.clone())),
                    );
                let actions = div()
                    .flex()
                    .flex_wrap()
                    .justify_end()
                    .gap(px(8.0))
                    .when(!is_confirming, |row| row.child(
                        button(theme, "copy-fingerprint", self.language.pick("复制指纹", "Copy fingerprint"))
                            .on_click(cx.listener(|this, _, _, cx| this.copy_prompted_fingerprint(cx))),
                    ))
                    .when(*issue == HostKeyIssue::Changed && !is_confirming, |row| row.child(
                        button(theme, "copy-cleanup-command", self.language.pick("复制清理命令", "Copy cleanup command"))
                            .on_click(cx.listener(|this, _, _, cx| this.copy_known_host_cleanup(cx))),
                    ))
                    .child(
                        button(theme, "cancel-host-key", if is_confirming || *issue == HostKeyIssue::Unknown {
                            self.language.pick("取消", "Cancel")
                        } else {
                            self.language.pick("关闭", "Close")
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if is_confirming {
                                this.cancel_host_key_replacement(cx);
                            } else {
                                this.close_auth_prompt(cx);
                            }
                        })),
                    )
                    .when(*issue == HostKeyIssue::Unknown, |row| row.child(
                        primary_button(theme, "trust-host-key", self.language.pick("信任并连接", "Trust and connect"))
                            .on_click(cx.listener(|this, _, _, cx| this.trust_prompted_host(cx))),
                    ))
                    .when(*issue == HostKeyIssue::Changed && !is_confirming, |row| row.child(
                        primary_button(theme, "review-host-key-replacement", self.language.pick("更新密钥并连接", "Update key and connect"))
                            .on_click(cx.listener(|this, _, _, cx| this.begin_host_key_replacement(cx))),
                    ))
                    .when(*issue == HostKeyIssue::Changed && is_confirming, |row| row.child(
                        button(theme, "confirm-host-key-replacement", self.language.pick("确认更新并连接", "Confirm update and connect"))
                            .bg(theme.danger_bg)
                            .border_color(theme.danger)
                            .text_color(theme.danger)
                            .on_click(cx.listener(|this, _, _, cx| this.replace_prompted_host_key(cx))),
                    ));
                let height = if *issue == HostKeyIssue::Changed { 460.0 } else { 340.0 };
                (title, body, actions, height)
            }
            AuthPrompt::Passphrase { input, .. } => (
                self.language.pick("输入私钥口令", "Enter private key passphrase"),
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .line_height(relative(1.5))
                            .text_color(theme.muted)
                            .child(self.language.pick(
                                "该私钥已加密，口令只用于本次连接，不会写入配置文件。",
                                "This key is encrypted. The passphrase is used only for this connection and is never saved.",
                            )),
                    )
                    .child(Self::form_field(
                        theme,
                        self.language.pick("私钥口令", "Private key passphrase"),
                        input.clone(),
                    )),
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        button(theme, "cancel-passphrase", self.language.pick("取消", "Cancel"))
                            .on_click(cx.listener(|this, _, _, cx| this.close_auth_prompt(cx))),
                    )
                    .child(
                        primary_button(theme, "submit-passphrase", self.language.pick("连接", "Connect"))
                            .on_click(cx.listener(|this, _, _, cx| this.submit_passphrase(cx))),
                    ),
                290.0,
            ),
        };
        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.backdrop)
            .child(
                div()
                    .w(px(500.0))
                    .max_w(relative(0.94))
                    .h(px(height))
                    .max_h(relative(0.90))
                    .flex()
                    .flex_col()
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.surface)
                    .text_color(theme.text)
                    .child(
                        div()
                            .h(px(56.0))
                            .flex_none()
                            .px(px(18.0))
                            .flex()
                            .items_center()
                            .border_b_1()
                            .border_color(theme.border_soft)
                            .font_weight(FontWeight::MEDIUM)
                            .child(title),
                    )
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_h(px(0.0))
                            .child(
                                div()
                                    .id("auth-prompt-scroll")
                                    .h_full()
                                    .overflow_y_scroll()
                                    .track_scroll(&self.auth_prompt_scroll)
                                    .p(px(18.0))
                                    .child(body),
                            )
                            .child(scrollbar(
                                "auth-prompt-scrollbar",
                                theme,
                                self.auth_prompt_scroll.clone(),
                            )),
                    )
                    .child(
                        actions
                            .flex_none()
                            .p(px(18.0))
                            .border_t_1()
                            .border_color(theme.border_soft),
                    ),
            )
    }
}
