use super::super::*;

impl TunnelMateApp {
    pub(crate) fn set_filter(&mut self, filter: TunnelFilter, cx: &mut Context<Self>) {
        self.filter = filter;
        self.selected_tunnel = self
            .config
            .tunnels
            .iter()
            .find(|tunnel| match &self.filter {
                TunnelFilter::All => true,
                TunnelFilter::Active => self.is_active(&tunnel.id),
                TunnelFilter::Activity => false,
                TunnelFilter::Group(group_id) => tunnel.group_id.as_ref() == Some(group_id),
            })
            .map(|tunnel| tunnel.id.clone());
        self.notice = None;
        cx.notify();
    }

    pub(crate) fn open_create_sheet(&mut self, cx: &mut Context<Self>) {
        let mut form = TunnelForm::new(None, self.language, cx);
        if let TunnelFilter::Group(group_id) = &self.filter {
            form.group_id = Some(group_id.clone());
        }
        form.ssh_hosts = parse_ssh_config(self.config.settings.ssh_config_path.as_deref());
        self.form = Some(form);
        self.notice = None;
        cx.notify();
    }

    pub(crate) fn close_create_sheet(&mut self, cx: &mut Context<Self>) {
        self.form = None;
        cx.notify();
    }

    pub(crate) fn edit_selected(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_tunnel.clone() else {
            return;
        };
        self.edit_tunnel(id, cx);
    }

    pub(crate) fn edit_tunnel(&mut self, id: String, cx: &mut Context<Self>) {
        self.selected_tunnel = Some(id.clone());
        let tunnel = self
            .config
            .tunnels
            .iter()
            .find(|tunnel| tunnel.id == id)
            .cloned();
        if let Some(mut tunnel) = tunnel {
            if let Some(reference) = tunnel.jump_host_id.as_ref().and_then(|reference_id| {
                self.config
                    .tunnels
                    .iter()
                    .find(|candidate| &candidate.id == reference_id)
            }) {
                tunnel.jump_host_id = None;
                tunnel.jump_host = Some(reference.ssh_host.clone());
                tunnel.jump_port = Some(reference.ssh_port);
                tunnel.jump_user = Some(reference.ssh_user.clone());
                tunnel.jump_identity_file = reference.ssh_identity_file.clone();
                tunnel.jump_password = reference.ssh_password.clone();
            }
            let mut form = TunnelForm::new(Some(&tunnel), self.language, cx);
            form.ssh_hosts = parse_ssh_config(self.config.settings.ssh_config_path.as_deref());
            self.form = Some(form);
            self.notice = None;
            cx.notify();
        }
    }

    pub(crate) fn open_advanced_selected(&mut self, cx: &mut Context<Self>) {
        self.edit_selected(cx);
        if let Some(form) = &mut self.form {
            form.advanced = true;
        }
        cx.notify();
    }

    pub(crate) fn request_delete_selected(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_tunnel.clone() else {
            return;
        };
        self.delete_confirmation = Some(id);
        cx.notify();
    }

    pub(crate) fn request_delete_from_form(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.form.as_ref().and_then(|form| form.editing_id.clone()) else {
            return;
        };
        self.delete_confirmation = Some(id);
        cx.notify();
    }

    pub(crate) fn cancel_delete_confirmation(&mut self, cx: &mut Context<Self>) {
        self.delete_confirmation = None;
        cx.notify();
    }

    pub(crate) fn confirm_delete_tunnel(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.delete_confirmation.take() else {
            return;
        };
        let Some(tunnel) = self
            .config
            .tunnels
            .iter()
            .find(|tunnel| tunnel.id == id)
            .cloned()
        else {
            cx.notify();
            return;
        };

        if self.is_active(&id) {
            self.pending_starts.remove(&id);
            self.pending_delete = Some(id.clone());
            self.form = None;
            self.notice = Some(
                if self.language == Language::Zh {
                    format!("正在停止并删除“{}”…", tunnel.name)
                } else {
                    format!("Stopping and deleting “{}”…", tunnel.name)
                }
                .into(),
            );
            let manager = self.manager.clone();
            let sender = self.messages.clone();
            self.runtime.spawn(async move {
                match TunnelManager::stop_tunnel(manager, &tunnel.id).await {
                    Ok(()) => {
                        let _ = sender.send(AppMessage::DeleteReady(tunnel.id)).await;
                    }
                    Err(message) => {
                        let _ = sender
                            .send(AppMessage::DeleteFailed {
                                tunnel_name: tunnel.name,
                                message,
                            })
                            .await;
                    }
                }
            });
            cx.notify();
            return;
        }

        self.delete_tunnel(id, cx);
    }

    pub(crate) fn delete_tunnel(&mut self, id: String, cx: &mut Context<Self>) {
        let name = self
            .config
            .tunnels
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.name.clone())
            .unwrap_or_default();
        let mut next_config = self.config.clone();
        next_config.tunnels.retain(|t| t.id != id);
        match ConfigStore::new().save_config(&next_config) {
            Ok(()) => {
                self.config = next_config;
                self.statuses.remove(&id);
                self.logs.remove(&id);
                self.pending_starts.remove(&id);
                self.pending_delete = None;
                self.selected_tunnel = None;
                self.form = None;
                self.notice = Some(
                    if self.language == Language::Zh {
                        format!("已删除“{name}”")
                    } else {
                        format!("Deleted “{name}”")
                    }
                    .into(),
                );
                self.refresh_tray();
            }
            Err(error) => {
                self.pending_delete = None;
                self.notice = Some(
                    if self.language == Language::Zh {
                        format!("删除失败：{error}")
                    } else {
                        format!("Could not delete tunnel: {error}")
                    }
                    .into(),
                );
            }
        }
        cx.notify();
    }

    pub(crate) fn run_selected_diagnostics(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_tunnel.clone() else {
            return;
        };
        self.run_tunnel_diagnostics(id, cx);
    }

    pub(crate) fn run_tunnel_diagnostics(&mut self, id: String, cx: &mut Context<Self>) {
        self.selected_tunnel = Some(id.clone());
        let listener_is_current_tunnel = self.status(&id) == TunnelStatus::Running;
        let Some(tunnel) = self.config.tunnels.iter().find(|t| t.id == id).cloned() else {
            return;
        };
        let all = self.config.tunnels.clone();
        let sender = self.messages.clone();
        let language = if self.language == Language::Zh {
            DiagnosticLanguage::Chinese
        } else {
            DiagnosticLanguage::English
        };
        self.diagnostics = Some(Vec::new());
        self.notice = None;
        self.runtime.spawn(async move {
            let steps =
                run_diagnostics(&tunnel, &all, None, language, listener_is_current_tunnel).await;
            let _ = sender.send(AppMessage::Diagnostics(steps)).await;
        });
        cx.notify();
    }

    pub(crate) fn close_diagnostics(&mut self, cx: &mut Context<Self>) {
        self.diagnostics = None;
        cx.notify();
    }

    pub(crate) fn show_about(&mut self, cx: &mut Context<Self>) {
        self.about_open = true;
        cx.notify();
    }

    pub(crate) fn close_about(&mut self, cx: &mut Context<Self>) {
        self.about_open = false;
        cx.notify();
    }

    pub(crate) fn request_quit(&mut self) {
        let manager = self.manager.clone();
        let sender = self.messages.clone();
        self.runtime.spawn(async move {
            TunnelManager::stop_all(manager).await;
            let _ = sender.send(AppMessage::QuitReady).await;
        });
    }

    pub(crate) fn request_close(&mut self, cx: &mut Context<Self>) {
        if self.config.settings.close_to_tray && self._tray.is_some() {
            cx.hide();
            set_dock_visible(false);
        } else {
            self.request_quit();
        }
    }

    pub(crate) fn dismiss(&mut self, cx: &mut Context<Self>) {
        if self.about_open {
            self.about_open = false;
            cx.notify();
            return;
        }
        if let Some(form) = &mut self.form {
            if form.ssh_picker_target.is_some() {
                form.ssh_picker_target = None;
                cx.notify();
                return;
            }
            if form.group_menu_open {
                form.group_menu_open = false;
                cx.notify();
                return;
            }
        }
        if self.pending_import.is_some() {
            self.pending_import = None;
            cx.notify();
            return;
        }
        if self.delete_confirmation.take().is_some() {
            cx.notify();
            return;
        }
        if self.group_delete_confirmation.take().is_some() {
            cx.notify();
            return;
        }
        if self.save_confirmation.take().is_none()
            && self.auth_prompt.take().is_none()
            && self.diagnostics.take().is_none()
            && self.group_form.take().is_none()
            && self.settings_form.take().is_none()
        {
            self.form = None;
        }
        cx.notify();
    }
}
