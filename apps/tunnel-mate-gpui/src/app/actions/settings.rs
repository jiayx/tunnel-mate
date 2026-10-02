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
            validation_error: None,
            scroll: gpui::ScrollHandle::new(),
            launch_on_startup: self.config.settings.launch_on_startup,
            start_minimized: self.config.settings.start_minimized,
            close_to_tray: self.config.settings.close_to_tray,
            keep_alive: cx.new(|cx| TextInput::new(cx, "30", keep_alive)),
            connect_timeout: cx.new(|cx| TextInput::new(cx, "15", connect_timeout)),
            ssh_config_path: cx.new(|cx| TextInput::new(cx, "~/.ssh/config", ssh_path)),
        });
        for input in [
            self.settings_form.as_ref().unwrap().keep_alive.clone(),
            self.settings_form.as_ref().unwrap().connect_timeout.clone(),
            self.settings_form.as_ref().unwrap().ssh_config_path.clone(),
        ] {
            let scroll = self.settings_form.as_ref().unwrap().scroll.clone();
            input.update(cx, |input, _| input.set_scroll_parent(scroll));
            cx.subscribe(&input, |this, _, _: &text_input::InputChanged, cx| {
                if let Some(form) = &mut this.settings_form {
                    form.validation_error = None;
                }
                cx.notify();
            })
            .detach();
        }
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
        let values = [
            (
                form.keep_alive.clone(),
                self.language.pick(
                    "保活间隔必须是大于 0 的秒数",
                    "Keep-alive must be greater than 0 seconds",
                ),
            ),
            (
                form.connect_timeout.clone(),
                self.language.pick(
                    "连接超时必须是大于 0 的秒数",
                    "Connection timeout must be greater than 0 seconds",
                ),
            ),
        ];
        let mut parsed = Vec::new();
        for (input, message) in values {
            match input.read(cx).value().trim().parse::<u32>() {
                Ok(value) if value > 0 => {
                    input.update(cx, |input, cx| input.set_invalid(false, cx));
                    parsed.push(value);
                }
                _ => {
                    input.update(cx, |input, cx| input.set_invalid(true, cx));
                    self.pending_field_focus = Some(input);
                    self.settings_form.as_mut().unwrap().validation_error = Some(message.into());
                    cx.notify();
                    return;
                }
            }
        }
        let [keep_alive, connect_timeout] = [parsed[0], parsed[1]];
        let form = self.settings_form.as_ref().unwrap();
        let ssh_path = form.ssh_config_path.read(cx).value();
        let mut next_config = self.config.clone();
        let autostart_changed = form.launch_on_startup != self.config.settings.launch_on_startup
            || (form.launch_on_startup
                && form.start_minimized != self.config.settings.start_minimized);
        next_config.settings.launch_on_startup = form.launch_on_startup;
        next_config.settings.start_minimized = form.start_minimized;
        next_config.settings.close_to_tray = form.close_to_tray;
        next_config.settings.keep_alive_interval = keep_alive;
        next_config.settings.connect_timeout = connect_timeout;
        next_config.settings.ssh_config_path = (!ssh_path.trim().is_empty()).then_some(ssh_path);
        if autostart_changed {
            if let Err(error) = system::sync_autostart(
                next_config.settings.launch_on_startup,
                next_config.settings.start_minimized,
            ) {
                self.show_persistent_notice(error);
                cx.notify();
                return;
            }
        }
        match ConfigStore::new().save_config(&next_config) {
            Ok(()) => {
                self.config = next_config;
                self.settings_form = None;
                self.show_transient_notice(self.language.pick("设置已保存", "Settings saved"), cx);
            }
            Err(error) => self.show_persistent_notice(format!("设置保存失败：{error}")),
        }
        cx.notify();
    }

    pub(crate) fn toggle_setting(&mut self, setting: SettingToggle, cx: &mut Context<Self>) {
        let Some(form) = &mut self.settings_form else {
            return;
        };
        match setting {
            SettingToggle::Launch => form.launch_on_startup = !form.launch_on_startup,
            SettingToggle::Minimized if form.launch_on_startup => {
                form.start_minimized = !form.start_minimized
            }
            SettingToggle::Minimized => {}
            SettingToggle::CloseToTray => form.close_to_tray = !form.close_to_tray,
        }
        cx.notify();
    }
}
