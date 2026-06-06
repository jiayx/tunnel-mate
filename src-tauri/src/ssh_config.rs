use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SshHostConfig {
    pub host: String,
    pub host_name: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
}

pub fn parse_ssh_config() -> Vec<SshHostConfig> {
    let mut hosts = Vec::new();
    let mut processed_paths = HashSet::new();

    if let Some(home) = dirs::home_dir() {
        let default_config_path = home.join(".ssh").join("config");
        if default_config_path.exists() {
            let _ = parse_config_file(&default_config_path, &mut hosts, &mut processed_paths);
        }
    }

    // Filter out wildcards and empty host configs
    hosts
        .into_iter()
        .filter(|h| !h.host.is_empty() && h.host != "*")
        .collect()
}

fn parse_config_file(
    path: &Path,
    hosts: &mut Vec<SshHostConfig>,
    processed_paths: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if processed_paths.contains(&canonical) {
        return Ok(());
    }
    processed_paths.insert(canonical.clone());

    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;

    let mut current_host: Option<SshHostConfig> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // SSH config values can be separated by spaces or equals
        let parts: Vec<&str> = if trimmed.contains('=') {
            trimmed.splitn(2, '=').map(|s| s.trim()).collect()
        } else {
            trimmed
                .splitn(2, |c: char| c.is_whitespace())
                .map(|s| s.trim())
                .collect()
        };

        if parts.len() < 2 {
            continue;
        }

        let key = parts[0].to_lowercase();
        let val = parts[1].trim_matches('"').trim();

        match key.as_str() {
            "host" => {
                // Save previous host if exists
                if let Some(h) = current_host.take() {
                    hosts.push(h);
                }
                // Split multi-hosts if separated by space
                let host_names: Vec<&str> = val.split_whitespace().collect();
                let primary_host = host_names.first().copied().unwrap_or(val);
                current_host = Some(SshHostConfig {
                    host: primary_host.to_string(),
                    ..Default::default()
                });
            }
            "hostname" => {
                if let Some(ref mut h) = current_host {
                    h.host_name = Some(val.to_string());
                }
            }
            "user" => {
                if let Some(ref mut h) = current_host {
                    h.user = Some(val.to_string());
                }
            }
            "port" => {
                if let Some(ref mut h) = current_host {
                    if let Ok(p) = val.parse::<u16>() {
                        h.port = Some(p);
                    }
                }
            }
            "identityfile" => {
                if let Some(ref mut h) = current_host {
                    let expanded = expand_home_dir(val);
                    h.identity_file = Some(expanded);
                }
            }
            "include" => {
                // Parse include paths (could contain globs)
                let include_pattern = expand_home_dir(val);
                let _ = process_includes(&include_pattern, hosts, processed_paths);
            }
            _ => {}
        }
    }

    // Push the final host
    if let Some(h) = current_host {
        hosts.push(h);
    }

    Ok(())
}

fn expand_home_dir(path_str: &str) -> String {
    if path_str.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path_str[2..]).to_string_lossy().into_owned();
        }
    }
    path_str.to_string()
}

fn process_includes(
    pattern: &str,
    hosts: &mut Vec<SshHostConfig>,
    processed_paths: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    let pattern_path = Path::new(pattern);

    // Simple absolute directory base check
    let parent_dir = if pattern_path.is_absolute() {
        pattern_path
            .parent()
            .unwrap_or(Path::new("/"))
            .to_path_buf()
    } else {
        // Relative to ~/.ssh
        let home = dirs::home_dir().ok_or("No home directory found")?;
        let ssh_dir = home.join(".ssh");
        pattern_path
            .parent()
            .map(|p| ssh_dir.join(p))
            .unwrap_or(ssh_dir)
    };

    if !parent_dir.exists() {
        return Ok(());
    }

    let file_name_pattern = pattern_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    if file_name_pattern.contains('*') || file_name_pattern.contains('?') {
        // Perform simple wildcard matching
        let regex_pattern = file_name_pattern
            .replace('.', "\\.")
            .replace('*', ".*")
            .replace('?', ".");
        let regex = regex::Regex::new(&format!("^{}$", regex_pattern))
            .map_err(|e| format!("Invalid regex pattern: {}", e))?;

        if let Ok(entries) = fs::read_dir(parent_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        if regex.is_match(&name) {
                            let _ = parse_config_file(&entry.path(), hosts, processed_paths);
                        }
                    }
                }
            }
        }
    } else {
        // Direct file include
        let target_path = if pattern_path.is_absolute() {
            pattern_path.to_path_buf()
        } else if let Some(home) = dirs::home_dir() {
            home.join(".ssh").join(pattern_path)
        } else {
            return Err("No home directory found".to_string());
        };

        if target_path.exists() {
            let _ = parse_config_file(&target_path, hosts, processed_paths);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_home_dir() {
        let path = "~/some_key";
        let expanded = expand_home_dir(path);
        if let Some(home) = dirs::home_dir() {
            assert_eq!(expanded, home.join("some_key").to_string_lossy());
        } else {
            assert_eq!(expanded, path);
        }
    }
}
