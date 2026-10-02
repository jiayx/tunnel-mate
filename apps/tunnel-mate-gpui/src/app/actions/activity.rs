use super::super::*;

impl TunnelMateApp {
    pub(crate) fn submit_primary(&mut self, cx: &mut Context<Self>) {
        match self.top_modal() {
            Some(ModalLayer::Import) => self.confirm_import_backup(cx),
            Some(ModalLayer::Save) => self.confirm_save_and_restart(cx),
            Some(ModalLayer::Auth) => match self.auth_prompt {
                Some(AuthPrompt::Passphrase { .. }) => self.submit_passphrase(cx),
                Some(AuthPrompt::HostKey {
                    issue: HostKeyIssue::Unknown,
                    ..
                }) => self.trust_prompted_host(cx),
                _ => {} // Changed or revoked keys require an explicit button.
            },
            Some(ModalLayer::Group) => self.save_group(cx),
            Some(ModalLayer::Settings) => self.save_settings(cx),
            Some(ModalLayer::Tunnel)
                if !self.form.as_ref().is_some_and(|form| form.group_menu_open) =>
            {
                self.save_form(cx)
            }
            _ => {} // Destructive confirmations have no implicit default action.
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
