use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::ClinkConfig;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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
    fn test_default_config_has_clearurls_remote() {
        let cfg = ClinkConfig::default();
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        let loaded: ClinkConfig = toml::from_str(&toml_str).unwrap();
        let remote = loaded.remote.unwrap();
        assert!(remote.url.contains("clearurls"));
        assert_eq!(remote.format, RemoteFormat::ClearUrls);
    }

    #[test]
    fn test_config_without_remote_section() {
        let toml_str = r#"
mode = 'remove'
replace_to = 'clink'
sleep_duration = 150

[providers]
"#;
        let loaded: ClinkConfig = toml::from_str(toml_str).unwrap();
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
}
