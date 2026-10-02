use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModalLayer {
    Tunnel,
    SshPicker,
    Save,
    Delete,
    DeleteGroup,
    Diagnostics,
    Group,
    Settings,
    Import,
    Auth,
    About,
}

pub(crate) fn modal_layer(
    layer: ModalLayer,
    active: Option<ModalLayer>,
    focus: &gpui::FocusHandle,
    content: impl IntoElement,
) -> gpui::Div {
    div()
        .absolute()
        .inset_0()
        .when(active == Some(layer), |wrapper| {
            wrapper.tab_group().track_focus(focus)
        })
        .child(content)
}

impl TunnelMateApp {
    pub(crate) fn top_modal(&self) -> Option<ModalLayer> {
        if self.about_open {
            Some(ModalLayer::About)
        } else if self.auth_prompt.is_some() {
            Some(ModalLayer::Auth)
        } else if self.pending_import.is_some() {
            Some(ModalLayer::Import)
        } else if self.settings_form.is_some() {
            Some(ModalLayer::Settings)
        } else if self.group_form.is_some() {
            Some(ModalLayer::Group)
        } else if self.diagnostics.is_some() {
            Some(ModalLayer::Diagnostics)
        } else if self.group_delete_confirmation.is_some() {
            Some(ModalLayer::DeleteGroup)
        } else if self.delete_confirmation.is_some() {
            Some(ModalLayer::Delete)
        } else if self.save_confirmation.is_some() {
            Some(ModalLayer::Save)
        } else if self
            .form
            .as_ref()
            .is_some_and(|form| form.ssh_picker_target.is_some())
        {
            Some(ModalLayer::SshPicker)
        } else if self.form.is_some() {
            Some(ModalLayer::Tunnel)
        } else {
            None
        }
    }

    pub(crate) fn sync_modal_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let active = self.top_modal();
        if active != self.active_modal {
            if self.active_modal.is_none() {
                self.return_focus = window.focused(cx);
            }
            self.active_modal = active;
            if active.is_some() {
                window.focus(&self.modal_focus, cx);
                let focus = self.modal_focus.clone();
                // Tab order is available after the modal has painted.
                window.on_next_frame(move |window, cx| {
                    if focus.is_focused(window) {
                        window.focus_next(cx);
                        if !focus.contains_focused(window, cx) {
                            window.focus(&focus, cx);
                        }
                    }
                });
            } else if let Some(focus) = self.return_focus.take() {
                window.focus(&focus, cx);
            } else {
                window.focus(&self.root_focus, cx);
            }
        } else if window.focused(cx).is_none() {
            window.focus(&self.root_focus, cx);
        }
    }

    pub(crate) fn move_focus(&self, backwards: bool, window: &mut Window, cx: &mut Context<Self>) {
        if backwards {
            window.focus_prev(cx);
        } else {
            window.focus_next(cx);
        }
        if self.active_modal.is_some() && !self.modal_focus.contains_focused(window, cx) {
            window.focus(&self.modal_focus, cx);
            window.focus_next(cx);
            if !self.modal_focus.contains_focused(window, cx) {
                window.focus(&self.modal_focus, cx);
                return;
            }
            if backwards {
                // Find the last stop inside the current modal, skipping the background.
                for _ in 0..256 {
                    let previous = window.focused(cx);
                    window.focus_next(cx);
                    if !self.modal_focus.contains_focused(window, cx) {
                        if let Some(previous) = previous {
                            window.focus(&previous, cx);
                        }
                        break;
                    }
                }
            }
        }
    }
}
