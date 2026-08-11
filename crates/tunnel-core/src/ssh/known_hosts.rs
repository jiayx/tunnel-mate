use ring::hmac;
use russh::keys::ssh_key;
use russh::keys::ssh_key::known_hosts::{HostPatterns, KnownHosts, Marker};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HostKeyStatus {
    Trusted,
    Unknown,
    Changed,
    Revoked,
}

pub(super) fn host_key_error(
    kind: &str,
    host: &str,
    port: u16,
    server_public_key: &ssh_key::PublicKey,
) -> String {
    let saved_fingerprints = known_host_fingerprints(host, port).join(",");
    format!(
        "{}|{}|{}|{}|{}",
        kind,
        host,
        port,
        server_public_key.fingerprint(ssh_key::HashAlg::Sha256),
        saved_fingerprints
    )
}

pub(super) fn known_host_fingerprints(host: &str, port: u16) -> Vec<String> {
    let Some(path) = known_hosts_path() else {
        return Vec::new();
    };
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let host_names = known_host_names(host, port);
    let mut fingerprints = Vec::new();
    for entry in KnownHosts::new(&content).filter_map(Result::ok) {
        if entry.marker() == Some(&Marker::CertAuthority)
            || !known_host_patterns_match(entry.host_patterns(), &host_names)
        {
            continue;
        }
        let fingerprint = entry
            .public_key()
            .fingerprint(ssh_key::HashAlg::Sha256)
            .to_string();
        if !fingerprints.contains(&fingerprint) {
            fingerprints.push(fingerprint);
        }
    }
    fingerprints
}

pub(super) fn remove_known_host_entries(content: &str, host: &str, port: u16) -> String {
    let host_names = known_host_names(host, port);
    let mut retained = Vec::new();
    for line in content.lines() {
        let remove = KnownHosts::new(line).filter_map(Result::ok).any(|entry| {
            entry.marker() != Some(&Marker::CertAuthority)
                && known_host_patterns_match(entry.host_patterns(), &host_names)
        });
        if !remove {
            retained.push(line);
        }
    }
    let mut result = retained.join("\n");
    if content.ends_with('\n') && !result.is_empty() {
        result.push('\n');
    }
    result
}

pub(super) fn check_known_host_key(
    host: &str,
    port: u16,
    server_public_key: &ssh_key::PublicKey,
) -> HostKeyStatus {
    let Some(path) = known_hosts_path() else {
        return HostKeyStatus::Unknown;
    };
    let Ok(content) = fs::read_to_string(path) else {
        return HostKeyStatus::Unknown;
    };

    let host_names = known_host_names(host, port);
    let mut host_has_keys = false;
    let mut matching_key_is_trusted = false;
    let mut matching_key_is_revoked = false;
    for entry in KnownHosts::new(&content).filter_map(Result::ok) {
        if !known_host_patterns_match(entry.host_patterns(), &host_names) {
            continue;
        }
        if entry.marker() == Some(&Marker::CertAuthority) {
            continue;
        }
        host_has_keys = true;
        if entry.public_key() == server_public_key {
            if entry.marker() == Some(&Marker::Revoked) {
                matching_key_is_revoked = true;
            } else {
                matching_key_is_trusted = true;
            }
        }
    }

    if matching_key_is_revoked {
        HostKeyStatus::Revoked
    } else if matching_key_is_trusted {
        HostKeyStatus::Trusted
    } else if host_has_keys {
        HostKeyStatus::Changed
    } else {
        HostKeyStatus::Unknown
    }
}

pub(super) fn known_host_patterns_match(patterns: &HostPatterns, host_names: &[String]) -> bool {
    match patterns {
        HostPatterns::HashedName { salt, hash } => host_names.iter().any(|host| {
            let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, salt);
            hmac::verify(&key, host.as_bytes(), hash).is_ok()
        }),
        HostPatterns::Patterns(patterns) => {
            let mut positive_match = false;
            for pattern in patterns {
                let (negated, pattern) = pattern
                    .strip_prefix('!')
                    .map_or((false, pattern.as_str()), |pattern| (true, pattern));
                if host_names.iter().any(|host| wildcard_match(pattern, host)) {
                    if negated {
                        return false;
                    }
                    positive_match = true;
                }
            }
            positive_match
        }
    }
}

pub(super) fn wildcard_match(pattern: &str, value: &str) -> bool {
    let pattern = pattern.to_ascii_lowercase().into_bytes();
    let value = value.to_ascii_lowercase().into_bytes();
    let mut previous = vec![false; value.len() + 1];
    previous[0] = true;

    for token in pattern {
        let mut current = vec![false; value.len() + 1];
        match token {
            b'*' => {
                current[0] = previous[0];
                for index in 1..=value.len() {
                    current[index] = previous[index] || current[index - 1];
                }
            }
            b'?' => {
                current[1..].copy_from_slice(&previous[..value.len()]);
            }
            literal => {
                for index in 1..=value.len() {
                    current[index] = previous[index - 1] && value[index - 1] == literal;
                }
            }
        }
        previous = current;
    }

    previous[value.len()]
}

pub(super) fn append_known_host(
    host: &str,
    port: u16,
    server_public_key: &ssh_key::PublicKey,
) -> Result<(), String> {
    let path = known_hosts_path().ok_or_else(|| "No home directory found".to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create known_hosts directory: {}", e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
                .map_err(|e| format!("Failed to secure known_hosts directory: {}", e))?;
        }
    }
    let host_name = known_host_names(host, port)
        .into_iter()
        .next()
        .ok_or_else(|| "Missing SSH host".to_string())?;
    let public_key = server_public_key
        .to_openssh()
        .map_err(|e| format!("Failed to encode server public key: {}", e))?;
    let mut options = fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&path)
        .map_err(|e| format!("Failed to open known_hosts: {}", e))?;
    writeln!(file, "{} {}", host_name, public_key)
        .map_err(|e| format!("Failed to write known_hosts: {}", e))
}

pub(super) fn known_hosts_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".ssh").join("known_hosts"))
}

pub(super) fn known_host_names(host: &str, port: u16) -> Vec<String> {
    if port == 22 {
        vec![host.to_string()]
    } else {
        vec![format!("[{}]:{}", host, port)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_host_patterns_support_wildcards_and_negation() {
        let hosts = vec!["api.example.com".to_string()];
        assert!(known_host_patterns_match(
            &HostPatterns::Patterns(vec!["*.example.com".to_string()]),
            &hosts
        ));
        assert!(!known_host_patterns_match(
            &HostPatterns::Patterns(vec![
                "*.example.com".to_string(),
                "!api.example.com".to_string(),
            ]),
            &hosts
        ));
    }

    #[test]
    fn known_host_patterns_support_openssh_hashed_names() {
        let salt = vec![7; 20];
        let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, &salt);
        let tag = hmac::sign(&key, b"[example.com]:2222");
        let mut hash = [0u8; 20];
        hash.copy_from_slice(tag.as_ref());
        let pattern = HostPatterns::HashedName { salt, hash };

        assert!(known_host_patterns_match(
            &pattern,
            &["[example.com]:2222".to_string()]
        ));
        assert!(!known_host_patterns_match(
            &pattern,
            &["[example.com]:22".to_string()]
        ));
    }

    #[test]
    fn replacing_host_key_removes_only_matching_non_ca_entries() {
        let key =
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIALndOGf6U9tjg0dJPWxBm+FxomYRRJOt2HZ2jFUh67F";
        let content = format!(
            "example.com {key}\nother.example.com {key}\n@cert-authority example.com {key}\n"
        );

        assert_eq!(
            remove_known_host_entries(&content, "example.com", 22),
            format!("other.example.com {key}\n@cert-authority example.com {key}\n")
        );
    }
}
