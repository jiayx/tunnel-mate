#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Language {
    Zh,
    En,
}

impl Language {
    pub(crate) fn system() -> Self {
        let locale = std::env::var("TUNNEL_MATE_LANG")
            .ok()
            .unwrap_or_else(|| sys_locale::get_locale().unwrap_or_default());
        Self::from_locale(&locale)
    }

    pub(crate) fn from_locale(locale: &str) -> Self {
        if locale.to_ascii_lowercase().starts_with("zh") {
            Self::Zh
        } else {
            Self::En
        }
    }

    pub(crate) fn pick(self, zh: &'static str, en: &'static str) -> &'static str {
        if self == Self::Zh {
            zh
        } else {
            en
        }
    }

    pub(crate) fn runtime_message(self, message: &str) -> String {
        if self == Self::En {
            return message.to_string();
        }
        match message {
            "Tunnel is already running" | "Tunnel is already running or connecting" => {
                "隧道已经在运行或连接中".to_string()
            }
            "Tunnel operation was cancelled" => "隧道操作已取消".to_string(),
            "Max reconnect attempts reached" => "已达到最大重连次数".to_string(),
            "Session disconnected" => "SSH 会话已断开".to_string(),
            "The SSH server only supports legacy ssh-rsa (SHA-1). Enable RSA-SHA2 on the server or update its SSH software." => {
                "SSH 服务器只支持已弃用的 ssh-rsa（SHA-1）。请在服务器启用 RSA-SHA2，或升级 SSH 软件。".to_string()
            }
            _ if message.starts_with("SSH connection failed:") => {
                message.replacen("SSH connection failed:", "SSH 连接失败：", 1)
            }
            _ if message.starts_with("Failed to start forwarding listeners:") => message.replacen(
                "Failed to start forwarding listeners:",
                "启动转发监听失败：",
                1,
            ),
            _ => message.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooses_language_from_locale() {
        assert_eq!(Language::from_locale("zh-CN"), Language::Zh);
        assert_eq!(Language::from_locale("en-US"), Language::En);
        assert_eq!(Language::from_locale("ja-JP"), Language::En);
    }

    #[test]
    fn translates_runtime_messages() {
        assert_eq!(
            Language::Zh.runtime_message("Max reconnect attempts reached"),
            "已达到最大重连次数"
        );
        assert_eq!(
            Language::En.runtime_message("Session disconnected"),
            "Session disconnected"
        );
        assert!(Language::Zh
            .runtime_message("The SSH server only supports legacy ssh-rsa (SHA-1). Enable RSA-SHA2 on the server or update its SSH software.")
            .starts_with("SSH 服务器只支持"));
    }
}
