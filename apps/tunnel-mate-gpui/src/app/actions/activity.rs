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
        if self.delete_confirmation.is_some() {
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
                self.notice = Some(
                    self.language
                        .pick("活动记录已清空", "Activity cleared")
                        .into(),
                );
            }
            Err(error) => self.notice = Some(format!("清空失败：{error}").into()),
        }
        cx.notify();
    }

    pub(crate) fn clear_selected_logs(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = &self.selected_tunnel {
            self.logs.remove(id);
        }
        self.notice = Some(
            self.language
                .pick("当前隧道日志已清空", "Tunnel logs cleared")
                .into(),
        );
        cx.notify();
    }

    pub(crate) fn copy_selected_logs(&mut self, cx: &mut Context<Self>) {
        let content = self
            .selected_tunnel
            .as_ref()
            .and_then(|id| self.logs.get(id))
            .map(|logs| logs.join("\n"))
            .unwrap_or_default();
        cx.write_to_clipboard(ClipboardItem::new_string(content));
        self.notice = Some(self.language.pick("日志已复制", "Logs copied").into());
        cx.notify();
    }

    pub(crate) fn export_selected_logs(&mut self, cx: &mut Context<Self>) {
        let Some(id) = &self.selected_tunnel else {
            return;
        };
        let content = self
            .logs
            .get(id)
            .map(|logs| logs.join("\n"))
            .unwrap_or_default();
        let receiver = cx.prompt_for_new_path(Path::new("."), Some("tunnel-mate.log"));
        let sender = self.messages.clone();
        cx.spawn(async move |_, _| {
            if let Ok(Ok(Some(path))) = receiver.await {
                let message = match fs::write(&path, content) {
                    Ok(()) => format!("日志已导出到 {}", path.display()),
                    Err(error) => format!("日志导出失败：{error}"),
                };
                let _ = sender.send(AppMessage::FileOperation(message)).await;
            }
        })
        .detach();
    }
}
