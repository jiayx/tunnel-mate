use super::super::*;

impl TunnelMateApp {
    pub(crate) fn open_settings(&mut self, cx: &mut Context<Self>) {
        let keep_alive = self.config.settings.keep_alive_interval.to_string();
        let connect_timeout = self.config.settings.connect_timeout.to_string();
        let ssh_path = self
            .config
            .settings
            .ssh_config_path
            .clone()
            .unwrap_or_default();
        self.settings_form = Some(SettingsForm {
            launch_on_startup: self.config.settings.launch_on_startup,
            start_minimized: self.config.settings.start_minimized,
            close_to_tray: self.config.settings.close_to_tray,
            keep_alive: cx.new(|cx| TextInput::new(cx, "30", keep_alive)),
            connect_timeout: cx.new(|cx| TextInput::new(cx, "15", connect_timeout)),
            ssh_config_path: cx.new(|cx| TextInput::new(cx, "~/.ssh/config", ssh_path)),
        });
        cx.notify();
    }

    pub(crate) fn cancel_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_form = None;
        cx.notify();
    }

    pub(crate) fn save_settings(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &self.settings_form else {
            return;
        };
        let keep_alive = match form.keep_alive.read(cx).value().parse::<u32>() {
            Ok(value) if value > 0 => value,
            _ => {
                self.notice = Some(
                    self.language
                        .pick(
                            "保活间隔必须是大于 0 的秒数",
                            "Keep-alive must be greater than 0 seconds",
                        )
                        .into(),
                );
                cx.notify();
                return;
            }
        };
        let connect_timeout = match form.connect_timeout.read(cx).value().parse::<u32>() {
            Ok(value) if value > 0 => value,
            _ => {
                self.notice = Some(
                    self.language
                        .pick(
                            "连接超时必须是大于 0 的秒数",
                            "Connection timeout must be greater than 0 seconds",
                        )
                        .into(),
                );
                cx.notify();
                return;
            }
        };
        let ssh_path = form.ssh_config_path.read(cx).value();
        let mut next_config = self.config.clone();
        next_config.settings.launch_on_startup = form.launch_on_startup;
        next_config.settings.start_minimized = form.start_minimized;
        next_config.settings.close_to_tray = form.close_to_tray;
        next_config.settings.keep_alive_interval = keep_alive;
        next_config.settings.connect_timeout = connect_timeout;
        next_config.settings.ssh_config_path = (!ssh_path.trim().is_empty()).then_some(ssh_path);
        if let Err(error) = system::sync_autostart(
            next_config.settings.launch_on_startup,
            next_config.settings.start_minimized,
        ) {
            self.notice = Some(error.into());
            cx.notify();
            return;
        }
        match ConfigStore::new().save_config(&next_config) {
            Ok(()) => {
                self.config = next_config;
                self.settings_form = None;
                self.notice = Some(self.language.pick("设置已保存", "Settings saved").into());
            }
            Err(error) => self.notice = Some(format!("设置保存失败：{error}").into()),
        }
        cx.notify();
    }

    pub(crate) fn toggle_setting(&mut self, setting: SettingToggle, cx: &mut Context<Self>) {
        let Some(form) = &mut self.settings_form else {
            return;
        };
        match setting {
            SettingToggle::Launch => form.launch_on_startup = !form.launch_on_startup,
            SettingToggle::Minimized => form.start_minimized = !form.start_minimized,
            SettingToggle::CloseToTray => form.close_to_tray = !form.close_to_tray,
        }
        cx.notify();
    }
}
