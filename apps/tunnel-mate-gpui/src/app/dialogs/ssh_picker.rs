use super::super::*;
use crate::scrollbar::scrollbar;

impl TunnelMateApp {
    pub(crate) fn render_notice(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        div()
            .absolute()
            .right(px(18.0))
            .bottom(px(if self.filter == TunnelFilter::Activity {
                18.0
            } else {
                76.0
            }))
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .max_w(px(420.0))
            .px(px(13.0))
            .py(px(11.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .rounded(px(9.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .shadow_lg()
            .text_size(px(12.0))
            .text_color(theme.text)
            .child(div().size(px(7.0)).rounded(px(4.0)).bg(theme.primary))
            .child(
                div().flex_grow(1.0).min_w_0().whitespace_normal().child(
                    self.notice
                        .as_ref()
                        .map(|notice| notice.message.clone())
                        .unwrap_or_default(),
                ),
            )
            .child(
                close_button(theme, "dismiss_notice")
                    .size(px(24.0))
                    .tab_index(0)
                    .on_click(cx.listener(|this, _, _, cx| {
                        cx.stop_propagation();
                        this.dismiss_notice(cx);
                    })),
            )
    }

    pub(crate) fn render_ssh_host_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let form = self.form.as_ref().expect("open tunnel form");
        let picker_target = form.ssh_picker_target.unwrap_or(SshPickerTarget::Primary);
        let (title, description) = match picker_target {
            SshPickerTarget::Primary => (
                self.language.pick("选择 SSH 主机", "Choose an SSH host"),
                self.language.pick(
                    "选择后会填入 SSH 连接的主机、端口、用户和私钥",
                    "Fills the SSH connection host, port, user, and identity",
                ),
            ),
            SshPickerTarget::JumpHost => (
                self.language.pick("选择跳板机", "Choose a jump host"),
                self.language.pick(
                    "选择后会填入跳板机的主机、端口、用户和私钥",
                    "Fills the jump host, port, user, and identity",
                ),
            ),
        };
        let current_host = match picker_target {
            SshPickerTarget::Primary => form.ssh_host.read(cx).value(),
            SshPickerTarget::JumpHost => form.jump_host.read(cx).value(),
        };
        let matched_index = form
            .ssh_hosts
            .iter()
            .position(|host| ssh_host_matches(host, &current_host));
        let mut host_indices = (0..form.ssh_hosts.len()).collect::<Vec<_>>();
        if let Some(index) = matched_index {
            host_indices.remove(index);
            host_indices.insert(0, index);
        }
        let mut hosts = div()
            .id("ssh-host-picker-scroll")
            .flex()
            .flex_col()
            .track_scroll(&form.ssh_picker_scroll)
            .h_full()
            .overflow_y_scroll()
            .py(px(6.0));

        if form.ssh_hosts.is_empty() {
            hosts = hosts.child(
                div()
                    .h(px(140.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(7.0))
                    .text_color(theme.muted)
                    .child(
                        div().text_size(px(12.0)).child(
                            self.language
                                .pick("SSH config 中没有可用主机", "No hosts found in SSH config"),
                        ),
                    ),
            );
        } else {
            for index in host_indices {
                let host = &form.ssh_hosts[index];
                let selected = matched_index == Some(index);
                let endpoint = format!(
                    "{}@{}:{}",
                    host.user
                        .as_deref()
                        .unwrap_or(self.language.pick("默认用户", "default user")),
                    host.host_name.as_deref().unwrap_or(&host.host),
                    host.port.unwrap_or(22)
                );
                let identity = host.identity_file.as_deref().map(|path| {
                    Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(path)
                        .to_string()
                });
                hosts = hosts.child(
                    div()
                        .id(("ssh-host-option", index))
                        .role(gpui::Role::Button)
                        .aria_label(host.host.clone())
                        .flex_none()
                        .key_context("TunnelButton")
                        .tab_index(0)
                        .focus(|style| style.border_color(theme.primary_hover))
                        .px(px(18.0))
                        .py(px(12.0))
                        .flex()
                        .items_center()
                        .gap(px(11.0))
                        .border_l_2()
                        .border_color(if selected {
                            theme.primary
                        } else {
                            theme.surface
                        })
                        .bg(if selected {
                            theme.selected
                        } else {
                            theme.surface
                        })
                        .cursor_pointer()
                        .hover(move |style| style.bg(theme.surface_hover))
                        .on_click(cx.listener(move |this, _, _, cx| this.apply_ssh_host(index, cx)))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_grow(1.0)
                                .min_w_0()
                                .gap(px(3.0))
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .truncate()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.text)
                                        .child(host.host.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(theme.muted)
                                        .truncate()
                                        .child(endpoint),
                                )
                                .when_some(identity, |column, identity| {
                                    column.child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.muted)
                                            .truncate()
                                            .child(identity),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .w(px(24.0))
                                .text_center()
                                .text_color(theme.primary)
                                .child(if selected { "✓" } else { "›" }),
                        ),
                );
            }
        }

        modal_backdrop()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.backdrop)
            .child(
                div()
                    .w(px(520.0))
                    .max_w(relative(0.94))
                    .h(px(if form.ssh_hosts.is_empty() {
                        240.0
                    } else {
                        500.0
                    }))
                    .max_h(relative(0.90))
                    .flex()
                    .flex_col()
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.surface)
                    .shadow_lg()
                    .overflow_hidden()
                    .child(
                        div()
                            .h(px(70.0))
                            .flex_none()
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(theme.border)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(3.0))
                                    .child(div().font_weight(FontWeight::MEDIUM).child(title))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.muted)
                                            .child(description),
                                    ),
                            )
                            .child(
                                close_button(theme, "close_ssh_hosts").on_click(
                                    cx.listener(|this, _, _, cx| this.close_ssh_hosts(cx)),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_h(px(0.0))
                            .child(hosts)
                            .child(scrollbar(
                                "ssh-host-picker-scrollbar",
                                theme,
                                form.ssh_picker_scroll.clone(),
                            )),
                    ),
            )
    }
}
