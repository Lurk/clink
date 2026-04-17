use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::migration;
use crate::mode::Mode;

#[derive(Serialize, Deserialize, Debug)]
pub struct RemotePatterns {
    pub providers: HashMap<String, crate::provider::ProviderConfig>,
}

pub fn resolve_patterns(config: &mut ClinkConfig, data_dir: &Path) {
    let cache_path = data_dir.join("remote_patterns.toml");
    let Ok(content) = std::fs::read_to_string(&cache_path) else {
        return;
    };
    let Ok(remote) = toml::from_str::<RemotePatterns>(&content) else {
        return;
    };

    for (name, remote_provider) in remote.providers {
        config
            .providers
            .entry(name)
            .and_modify(|local| {
                local.rules.extend(remote_provider.rules.clone());
                local
                    .redirections
                    .extend(remote_provider.redirections.clone());
                if local.url_pattern.is_none() {
                    local.url_pattern.clone_from(&remote_provider.url_pattern);
                }
            })
            .or_insert(remote_provider);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RemoteFormat {
    #[serde(rename = "clearurls")]
    ClearUrls,
    #[serde(rename = "clink")]
    Clink,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Remote {
    pub url: String,
    pub format: RemoteFormat,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClinkConfig {
    pub mode: Mode,
    pub replace_to: String,
    pub sleep_duration: u64,
    pub providers: HashMap<String, crate::provider::ProviderConfig>,
    #[serde(skip)]
    pub verbose: bool,
    #[serde(default)]
    pub remote: Option<Remote>,
}

impl ClinkConfig {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            replace_to: "clink".into(),
            sleep_duration: 150,
            providers: HashMap::new(),
            verbose: false,
            remote: None,
        }
    }

    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.sleep_duration == 0 {
            warnings.push("sleep_duration is 0, this will cause 100% CPU usage".to_string());
        }
        let has_rules = self.providers.values().any(|p| !p.rules.is_empty());
        if !has_rules {
            warnings.push("No tracking params configured — clink won't clean anything".to_string());
        }
        warnings
    }
}

impl Default for ClinkConfig {
    fn default() -> Self {
        Self::new(Mode::Remove)
    }
}

pub fn load_config(config_path: &Path) -> Result<ClinkConfig, String> {
    let path = config_path.display();

    if config_path.exists() {
        let content = std::fs::read_to_string(config_path)
            .map_err(|e| format!("Failed to read config at {path}: {e}"))?;

        let raw: toml::Value =
            toml::from_str(&content).map_err(|e| format!("Config error at {path}: {e}"))?;

        if raw.get("params").is_some() || raw.get("exit").is_some() {
            return migrate_old_config(config_path, &raw);
        }
    }

    let config: ClinkConfig = confy::load_path(config_path).map_err(|e| {
        format!(
            "Config error at {path}: {e}\n\n\
             Looks like you have a bad config or config for an old version.\n\
             Config should look like this:\n\n{}",
            toml::to_string_pretty(&ClinkConfig::default()).unwrap()
        )
    })?;
    Ok(config)
}

fn migrate_old_config(config_path: &Path, raw: &toml::Value) -> Result<ClinkConfig, String> {
    let mode: Mode = raw
        .get("mode")
        .and_then(|v| v.as_str())
        .and_then(|s| toml::from_str(&format!("\"{s}\"")).ok())
        .unwrap_or(Mode::Remove);

    let replace_to = raw
        .get("replace_to")
        .and_then(|v| v.as_str())
        .unwrap_or("clink")
        .to_string();

    #[allow(clippy::cast_sign_loss)]
    let sleep_duration = raw
        .get("sleep_duration")
        .and_then(toml::Value::as_integer)
        .map_or(150, |v| v as u64);

    let remote: Option<Remote> = raw.get("remote").and_then(|v| {
        let url = v.get("url")?.as_str()?.to_string();
        let format_str = v.get("format")?.as_str()?;
        let format: RemoteFormat = toml::from_str(&format!("\"{format_str}\"")).ok()?;
        Some(Remote { url, format })
    });

    let params: Vec<String> = raw
        .get("params")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let exits: Vec<Vec<String>> = raw
        .get("exit")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v.as_array().map(|inner| {
                        inner
                            .iter()
                            .filter_map(|s| s.as_str().map(String::from))
                            .collect()
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let mut providers = migration::migrate_params(&params);
    let exit_providers = migration::migrate_exits(&exits);

    for (name, exit_provider) in exit_providers {
        providers
            .entry(name)
            .and_modify(|existing| {
                existing
                    .redirections
                    .extend(exit_provider.redirections.clone());
                if existing.url_pattern.is_none() {
                    existing.url_pattern.clone_from(&exit_provider.url_pattern);
                }
            })
            .or_insert(exit_provider);
    }

    let config = ClinkConfig {
        mode,
        replace_to,
        sleep_duration,
        providers,
        verbose: false,
        remote,
    };

    let backup_path = config_path.with_extension("toml.backup");
    std::fs::copy(config_path, &backup_path)
        .map_err(|e| format!("Failed to back up config to {}: {e}", backup_path.display()))?;

    let new_toml = toml::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize migrated config: {e}"))?;
    std::fs::write(config_path, &new_toml)
        .map_err(|e| format!("Failed to write migrated config: {e}"))?;

    eprintln!(
        "Config migrated to new provider format. Backup saved to {}",
        backup_path.display()
    );

    Ok(config)
}

pub fn fallback_config_path(path: Option<PathBuf>) -> PathBuf {
    let p = match path {
        Some(p) => p.join("clink"),
        None => std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from(".")),
    };

    p.join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_default_config() {
        let cfg = ClinkConfig::default();
        let warnings = cfg.validate();
        assert!(
            warnings.iter().any(|w| w.contains("params")),
            "default config with no providers should warn"
        );
    }

    #[test]
    fn test_validate_zero_sleep_duration() {
        let mut cfg = ClinkConfig::default();
        cfg.sleep_duration = 0;
        let warnings = cfg.validate();
        assert!(warnings.iter().any(|w| w.contains("sleep_duration")));
    }

    #[test]
    fn test_validate_empty_params() {
        let cfg = ClinkConfig::default();
        let warnings = cfg.validate();
        assert!(warnings.iter().any(|w| w.contains("params")));
    }

    #[test]
    fn test_load_config_returns_result() {
        let tmp = std::env::temp_dir().join("clink_test_bad_config.toml");
        std::fs::write(&tmp, "this is not valid [[[ toml").unwrap();
        let result = load_config(&tmp);
        assert!(result.is_err());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_remote_config_serde_roundtrip() {
        let cfg = ClinkConfig {
            remote: Some(Remote {
                url: "https://example.com/data.json".into(),
                format: RemoteFormat::ClearUrls,
            }),
            ..ClinkConfig::default()
        };
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        let loaded: ClinkConfig = toml::from_str(&toml_str).unwrap();
        let remote = loaded.remote.unwrap();
        assert_eq!(remote.url, "https://example.com/data.json");
        assert_eq!(remote.format, RemoteFormat::ClearUrls);
    }

    #[test]
    fn test_config_without_remote_section() {
        let cfg = ClinkConfig::default();
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        let loaded: ClinkConfig = toml::from_str(&toml_str).unwrap();
        assert!(loaded.remote.is_none());
    }

    #[test]
    fn test_remote_patterns_serde_roundtrip() {
        let mut providers = HashMap::new();
        providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["fbclid".into(), "gclid".into()],
                ..Default::default()
            },
        );
        providers.insert(
            "exitsc".to_string(),
            crate::provider::ProviderConfig {
                url_pattern: Some(r"^https?://exit\.sc".into()),
                redirections: vec![r"url=([^&]+)".into()],
                ..Default::default()
            },
        );
        let patterns = RemotePatterns { providers };
        let toml_str = toml::to_string_pretty(&patterns).unwrap();
        let loaded: RemotePatterns = toml::from_str(&toml_str).unwrap();
        assert_eq!(loaded.providers.len(), 2);
        assert_eq!(loaded.providers["global"].rules.len(), 2);
        assert_eq!(loaded.providers["exitsc"].redirections.len(), 1);
    }

    #[test]
    fn test_resolve_merges_remote_and_local() {
        let dir = std::env::temp_dir().join("clink_test_resolve");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut remote_providers = HashMap::new();
        remote_providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["remote_param".into(), "shared".into()],
                ..Default::default()
            },
        );
        remote_providers.insert(
            "remote_only".to_string(),
            crate::provider::ProviderConfig {
                url_pattern: Some(r"^https?://remote\.com".into()),
                redirections: vec![r"url=([^&]+)".into()],
                ..Default::default()
            },
        );
        let remote = RemotePatterns {
            providers: remote_providers,
        };
        let cache_path = dir.join("remote_patterns.toml");
        let content = toml::to_string(&remote).unwrap();
        std::fs::write(&cache_path, content).unwrap();

        let mut local_providers = HashMap::new();
        local_providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["local_param".into(), "shared".into()],
                ..Default::default()
            },
        );
        let mut cfg = ClinkConfig {
            providers: local_providers,
            ..ClinkConfig::default()
        };

        resolve_patterns(&mut cfg, &dir);

        let global = &cfg.providers["global"];
        assert!(
            global.rules.contains(&"local_param".to_string()),
            "should have local param"
        );
        assert!(
            global.rules.contains(&"remote_param".to_string()),
            "should have remote param"
        );
        assert!(
            global.rules.contains(&"shared".to_string()),
            "should have shared param"
        );
        assert!(
            cfg.providers.contains_key("remote_only"),
            "should have remote-only provider"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_no_cache_keeps_local() {
        let dir = std::env::temp_dir().join("clink_test_resolve_no_cache");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut local_providers = HashMap::new();
        local_providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["local_param".into()],
                ..Default::default()
            },
        );
        let mut cfg = ClinkConfig {
            providers: local_providers,
            ..ClinkConfig::default()
        };

        resolve_patterns(&mut cfg, &dir);

        assert_eq!(cfg.providers.len(), 1);
        assert!(
            cfg.providers["global"]
                .rules
                .contains(&"local_param".to_string())
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_auto_migrate_old_config() {
        let dir = std::env::temp_dir().join("clink_test_migrate_old");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let config_path = dir.join("config.toml");
        let old_config = r#"
mode = 'remove'
replace_to = 'clink'
sleep_duration = 150
params = ['fbclid', 'gclid', 'youtube.com``si']
exit = [['exit.sc/', 'url']]
"#;
        std::fs::write(&config_path, old_config).unwrap();

        let config = load_config(&config_path).unwrap();

        assert!(config.providers.contains_key("global"));
        assert!(
            config.providers["global"]
                .rules
                .contains(&"fbclid".to_string())
        );
        assert!(
            config.providers["global"]
                .rules
                .contains(&"gclid".to_string())
        );

        assert!(config.providers.contains_key("youtube_com"));
        assert!(
            config.providers["youtube_com"]
                .rules
                .contains(&"si".to_string())
        );

        assert!(config.providers.contains_key("exit_sc"));
        assert!(!config.providers["exit_sc"].redirections.is_empty());

        let backup_path = config_path.with_extension("toml.backup");
        assert!(backup_path.exists(), "backup file should be created");

        let migrated_content = std::fs::read_to_string(&config_path).unwrap();
        assert!(
            !migrated_content.contains("params ="),
            "migrated config should not have old params key"
        );
        assert!(
            migrated_content.contains("[providers"),
            "migrated config should have providers section"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_new_format_no_migration() {
        let dir = std::env::temp_dir().join("clink_test_no_migrate");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let config_path = dir.join("config.toml");
        let new_config = r#"
mode = 'remove'
replace_to = 'clink'
sleep_duration = 150

[providers.global]
rules = ['fbclid', 'gclid']
"#;
        std::fs::write(&config_path, new_config).unwrap();

        let config = load_config(&config_path).unwrap();

        assert!(config.providers.contains_key("global"));
        assert_eq!(config.providers["global"].rules.len(), 2);

        let backup_path = config_path.with_extension("toml.backup");
        assert!(
            !backup_path.exists(),
            "no backup should be created for new format"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
