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

enum Directive {
    Host(Vec<HostPattern>),
    Include(Vec<Directive>),
    Match,
    Option(String, String),
}

struct HostPattern {
    value: String,
    matcher: Option<regex::Regex>,
}

pub fn parse_ssh_config(config_path: Option<&str>) -> Vec<SshHostConfig> {
    let path = config_path
        .filter(|path| !path.trim().is_empty())
        .map(|path| PathBuf::from(expand_home_dir(path.trim())))
        .or_else(|| dirs::home_dir().map(|home| home.join(".ssh/config")));
    let Some(path) = path else { return Vec::new() };
    let directives = read_directives(&path, &mut HashSet::new());
    let mut aliases = Vec::new();
    collect_aliases(&directives, &mut aliases, &mut HashSet::new());
    aliases
        .into_iter()
        .map(|host| {
            let mut resolved = SshHostConfig {
                host,
                ..Default::default()
            };
            resolve_directives(&directives, &mut true, &mut resolved);
            resolved
        })
        .collect()
}

fn collect_aliases(
    directives: &[Directive],
    aliases: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    for directive in directives {
        match directive {
            Directive::Host(patterns) => {
                for pattern in patterns {
                    let alias = &pattern.value;
                    if !alias.starts_with('!')
                        && !alias.contains(['*', '?'])
                        && seen.insert(alias.to_ascii_lowercase())
                    {
                        aliases.push(alias.clone());
                    }
                }
            }
            Directive::Include(included) => collect_aliases(included, aliases, seen),
            _ => {}
        }
    }
}

fn matches_host(patterns: &[HostPattern], host: &str) -> bool {
    let mut matched = false;
    for pattern in patterns {
        if pattern
            .matcher
            .as_ref()
            .is_some_and(|matcher| matcher.is_match(host))
        {
            if pattern.value.starts_with('!') {
                return false;
            }
            matched = true;
        }
    }
    matched
}

fn resolve_directives(directives: &[Directive], active: &mut bool, result: &mut SshHostConfig) {
    for directive in directives {
        match directive {
            Directive::Host(patterns) => *active = matches_host(patterns, &result.host),
            // Match can depend on commands and connection state. Never execute it
            // or import its conditional values unconditionally.
            Directive::Match => *active = false,
            Directive::Include(included) if *active => resolve_directives(included, active, result),
            Directive::Option(key, value) if *active => match key.as_str() {
                "hostname" if result.host_name.is_none() => result.host_name = Some(value.clone()),
                "user" if result.user.is_none() => result.user = Some(value.clone()),
                "port" if result.port.is_none() => result.port = value.parse().ok(),
                // The UI stores one identity file; preserve the first one.
                "identityfile" if result.identity_file.is_none() => {
                    result.identity_file = Some(expand_home_dir(value))
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn read_directives(path: &Path, visiting: &mut HashSet<PathBuf>) -> Vec<Directive> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if visiting.len() >= 64 || !visiting.insert(canonical.clone()) {
        return Vec::new();
    }
    let mut directives = Vec::new();
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some(separator) = line.find(|c: char| c.is_whitespace() || c == '=') else {
                continue;
            };
            let key = line[..separator].to_ascii_lowercase();
            let value = line[separator..]
                .trim_start()
                .trim_start_matches('=')
                .trim_start();
            let values = split_values(value);
            match key.as_str() {
                "host" => directives.push(Directive::Host(
                    values
                        .into_iter()
                        .map(|value| {
                            let matcher =
                                wildcard_regex(value.strip_prefix('!').unwrap_or(&value), true);
                            HostPattern { value, matcher }
                        })
                        .collect(),
                )),
                "match" => directives.push(Directive::Match),
                "include" => {
                    let mut included = Vec::new();
                    for pattern in values {
                        for file in include_paths(&expand_home_dir(&pattern)) {
                            included.extend(read_directives(&file, visiting));
                        }
                    }
                    directives.push(Directive::Include(included));
                }
                _ => {
                    if let Some(value) = values.into_iter().next() {
                        directives.push(Directive::Option(key, value));
                    }
                }
            }
        }
    }
    visiting.remove(&canonical);
    directives
}

fn split_values(value: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut token = String::new();
    let mut quote = None;
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '#' if quote.is_none() => break,
            '"' | '\'' if quote == Some(c) => quote = None,
            '"' | '\'' if quote.is_none() => quote = Some(c),
            '\\' if chars.peek().is_some_and(|next| {
                next.is_whitespace() || matches!(next, '"' | '\'' | '#' | '\\')
            }) =>
            {
                token.push(chars.next().unwrap_or('\\'));
            }
            c if c.is_whitespace() && quote.is_none() => {
                if !token.is_empty() {
                    values.push(std::mem::take(&mut token));
                }
            }
            _ => token.push(c),
        }
    }
    if !token.is_empty() {
        values.push(token);
    }
    values
}

fn expand_home_dir(path: &str) -> String {
    if let Some(relative) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(relative).to_string_lossy().into_owned();
        }
    }
    path.to_string()
}

fn wildcard_regex(pattern: &str, case_insensitive: bool) -> Option<regex::Regex> {
    let expression = regex::escape(pattern)
        .replace("\\*", ".*")
        .replace("\\?", ".");
    regex::RegexBuilder::new(&format!("^{expression}$"))
        .case_insensitive(case_insensitive)
        .build()
        .ok()
}

fn include_paths(pattern: &str) -> Vec<PathBuf> {
    let path = PathBuf::from(pattern);
    let path = if path.is_absolute() {
        path
    } else {
        let Some(home) = dirs::home_dir() else {
            return Vec::new();
        };
        home.join(".ssh").join(path)
    };
    let Some(parent) = path.parent() else {
        return Vec::new();
    };
    let pattern = path.file_name().unwrap_or_default().to_string_lossy();
    if !pattern.contains(['*', '?']) {
        return vec![path];
    }
    let Some(regex) = wildcard_regex(&pattern, false) else {
        return Vec::new();
    };
    let mut files = fs::read_dir(parent)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.path().is_file() && regex.is_match(&entry.file_name().to_string_lossy())
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(content: &str) -> Vec<SshHostConfig> {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config");
        fs::write(&path, content).unwrap();
        parse_ssh_config(path.to_str())
    }

    #[test]
    fn expands_home_dir() {
        if let Some(home) = dirs::home_dir() {
            assert_eq!(
                expand_home_dir("~/some_key"),
                home.join("some_key").to_string_lossy()
            );
        }
    }

    #[test]
    fn reads_custom_config_and_all_aliases() {
        let hosts =
            parse("Host primary secondary\nHostName example.test\nUser deploy\nPort 2222\n");
        assert_eq!(
            hosts
                .iter()
                .map(|host| host.host.as_str())
                .collect::<Vec<_>>(),
            ["primary", "secondary"]
        );
        for host in hosts {
            assert_eq!(host.host_name.as_deref(), Some("example.test"));
            assert_eq!(host.user.as_deref(), Some("deploy"));
            assert_eq!(host.port, Some(2222));
        }
    }

    #[test]
    fn merges_matching_blocks_with_first_value_winning() {
        let hosts = parse(
            "Host production\nHostName example.test\nPort 2200\nHost *\nUser deploy\nPort 2222\n",
        );
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].user.as_deref(), Some("deploy"));
        assert_eq!(hosts[0].port, Some(2200));
        let hosts = parse("Host *\nUser global\nHost production\nUser specific\n");
        assert_eq!(hosts[0].user.as_deref(), Some("global"));
    }

    #[test]
    fn matches_patterns_and_negations_without_creating_wildcard_hosts() {
        let hosts = parse(
            "Host prod-a prod-b dev\nHost prod-* !prod-b\nUser deploy\nHost *\nUser fallback\n",
        );
        assert_eq!(hosts.len(), 3);
        assert_eq!(hosts[0].user.as_deref(), Some("deploy"));
        assert_eq!(hosts[1].user.as_deref(), Some("fallback"));
        assert_eq!(hosts[2].user.as_deref(), Some("fallback"));
    }

    #[test]
    fn handles_equals_comments_quotes_and_repeated_aliases() {
        let hosts = parse("Host production # comment\nUser = deploy\nIdentityFile \"/tmp/key with spaces\"\nHost PRODUCTION\nPort=2222\n");
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].user.as_deref(), Some("deploy"));
        assert_eq!(
            hosts[0].identity_file.as_deref(),
            Some("/tmp/key with spaces")
        );
        assert_eq!(hosts[0].port, Some(2222));
    }

    #[test]
    fn includes_are_sorted_and_cycles_do_not_recurse_forever() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("config");
        fs::write(directory.path().join("20.conf"), "User second\n").unwrap();
        fs::write(
            directory.path().join("10.conf"),
            format!("User first\nInclude {}\n", root.display()),
        )
        .unwrap();
        fs::write(
            &root,
            format!(
                "Host production\nInclude {}/*.conf\n",
                directory.path().display()
            ),
        )
        .unwrap();
        let hosts = parse_ssh_config(root.to_str());
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].user.as_deref(), Some("first"));
    }

    #[test]
    fn conditional_includes_do_not_leak_values_to_other_hosts() {
        let directory = tempfile::tempdir().unwrap();
        let included = directory.path().join("options");
        fs::write(&included, "User deploy\n").unwrap();
        let hosts = parse(&format!(
            "Host prod\nInclude {}\nHost dev\nHost *\nUser fallback\n",
            included.display()
        ));
        assert_eq!(hosts[0].user.as_deref(), Some("deploy"));
        assert_eq!(hosts[1].user.as_deref(), Some("fallback"));
    }
}
