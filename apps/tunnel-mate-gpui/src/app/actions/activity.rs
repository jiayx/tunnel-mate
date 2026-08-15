use super::super::*;

impl TunnelMateApp {
    pub(crate) fn submit_primary(&mut self, cx: &mut Context<Self>) {
        if self
            .form
            .as_ref()
            .is_some_and(|form| form.ssh_picker_target.is_some() || form.group_menu_open)
        {
            return;
        }
        if self.delete_confirmation.is_some() || self.group_delete_confirmation.is_some() {
            return;
        }
        if self.pending_import.is_some() {
            self.confirm_import_backup(cx);
        } else if self.save_confirmation.is_some() {
            self.confirm_save_and_restart(cx);
        } else if matches!(
            self.auth_prompt,
            Some(AuthPrompt::HostKey {
                issue: HostKeyIssue::Unknown,
                ..
            })
        ) {
            self.trust_prompted_host(cx);
        } else if matches!(self.auth_prompt, Some(AuthPrompt::HostKey { .. })) {
            // Host-key replacement and revoked-key dialogs intentionally have no
            // destructive default action. The user must click an explicit button.
        } else if matches!(self.auth_prompt, Some(AuthPrompt::Passphrase { .. })) {
            self.submit_passphrase(cx);
        } else if self.diagnostics.is_some() {
            self.diagnostics = None;
            cx.notify();
        } else if self.group_form.is_some() {
            self.save_group(cx);
        } else if self.settings_form.is_some() {
            self.save_settings(cx);
        } else if self.form.is_some() {
            self.save_form(cx);
        }
    }

    pub(crate) fn clear_activity(&mut self, cx: &mut Context<Self>) {
        match EventLogger::new().clear_events() {
            Ok(()) => {
                self.events.clear();
                self.show_transient_notice(
                    self.language.pick("活动记录已清空", "Activity cleared"),
                    cx,
                );
            }
            Err(error) => self.show_persistent_notice(format!("清空失败：{error}")),
        }
        cx.notify();
    }
}
