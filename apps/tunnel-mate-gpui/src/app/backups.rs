use super::*;

impl TunnelMateApp {
    pub(super) fn export_backup(&mut self, cx: &mut Context<Self>) {
        let content = match export_config_string(&self.config) {
            Ok(content) => content,
            Err(error) => {
                self.notice = Some(
                    format!(
                        "{}: {error}",
                        self.language.pick("导出失败", "Export failed")
                    )
                    .into(),
                );
                cx.notify();
                return;
            }
        };
        let directory = downloads_dir();
        let receiver = cx.prompt_for_new_path(&directory, Some("config.tunnelmate.json"));
        let sender = self.messages.clone();
        let language = self.language;
        cx.spawn(async move |_, _| {
            if let Ok(Ok(Some(path))) = receiver.await {
                let message = match fs::write(&path, content) {
                    Ok(()) => format!(
                        "{} {}",
                        language.pick("配置已导出到", "Configuration exported to"),
                        path.display()
                    ),
                    Err(error) => {
                        format!("{}: {error}", language.pick("导出失败", "Export failed"))
                    }
                };
                let _ = sender.send(AppMessage::FileOperation(message)).await;
            }
        })
        .detach();
    }

    pub(super) fn import_backup(&mut self, cx: &mut Context<Self>) {
        self.open_import_picker(cx);
    }

    pub(super) fn cancel_import_backup(&mut self, cx: &mut Context<Self>) {
        self.pending_import = None;
        cx.notify();
    }

    pub(super) fn confirm_import_backup(&mut self, cx: &mut Context<Self>) {
        let Some(config) = self.pending_import.take() else {
            return;
        };
        self.settings_form = None;
        let manager = self.manager.clone();
        let sender = self.messages.clone();
        let language = self.language;
        self.runtime.spawn(async move {
            TunnelManager::stop_all(manager).await;
            let message = match ConfigStore::new().save_config(&config) {
                Ok(()) => AppMessage::ConfigImported(config),
                Err(error) => AppMessage::ImportFailed(format!(
                    "{}: {error}",
                    language.pick("导入保存失败", "Could not save imported configuration")
                )),
            };
            let _ = sender.send(message).await;
        });
        cx.notify();
    }

    pub(super) fn commit_import(&self, config: AppConfig) {
        let sender = self.messages.clone();
        let language = self.language;
        self.runtime.spawn(async move {
            let message = match ConfigStore::new().save_config(&config) {
                Ok(()) => AppMessage::ConfigImported(config),
                Err(error) => AppMessage::ImportFailed(format!(
                    "{}: {error}",
                    language.pick("导入保存失败", "Could not save imported configuration")
                )),
            };
            let _ = sender.send(message).await;
        });
    }

    pub(super) fn open_import_picker(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("导入配置".into()),
        });
        let sender = self.messages.clone();
        let language = self.language;
        cx.spawn(async move |_, _| match receiver.await {
            Ok(Ok(Some(paths))) => {
                if let Some(path) = paths.into_iter().next() {
                    let result = fs::read_to_string(&path)
                        .map_err(|error| {
                            format!("{}: {error}", language.pick("读取失败", "Read failed"))
                        })
                        .and_then(|content| import_config_string(&content));
                    match result {
                        Ok(config) => {
                            let _ = sender.send(AppMessage::ImportSelected(config)).await;
                        }
                        Err(error) => {
                            let _ = sender
                                .send(AppMessage::ImportFailed(format!(
                                    "{}: {error}",
                                    language.pick("导入失败", "Import failed")
                                )))
                                .await;
                        }
                    }
                }
            }
            Ok(Ok(None)) => {}
            Ok(Err(error)) => {
                let _ = sender
                    .send(AppMessage::ImportFailed(format!(
                        "{}: {error}",
                        language.pick("无法打开文件选择器", "Could not open file picker")
                    )))
                    .await;
            }
            Err(error) => {
                let _ = sender
                    .send(AppMessage::ImportFailed(format!(
                        "{}: {error}",
                        language.pick("文件选择器意外关闭", "File picker closed unexpectedly")
                    )))
                    .await;
            }
        })
        .detach();
    }
}
