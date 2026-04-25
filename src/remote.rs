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

pub fn resolve_patterns(config: &mut ClinkConfig, data_dir: &Path) -> Vec<String> {
    let cache_path = data_dir.join("remote_patterns.toml");
    let mut warnings = Vec::new();

    // Distinguish "no cache" (normal first-run state) from "cache present but
    // unreadable/unparseable" — the second case means `clink update` produced
    // a bad file or someone hand-edited the cache, and silently falling back
    // to the builtin would leave the user thinking they have fresh rules.
    let cache = match std::fs::read_to_string(&cache_path) {
        Ok(content) => match toml::from_str::<RemotePatterns>(&content) {
            Ok(parsed) => Some(parsed),
            Err(e) => {
                warnings.push(format!(
                    "failed to parse remote pattern cache at {}: {e} — falling back to built-in patterns; re-run `clink update`",
                    cache_path.display()
                ));
                None
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            warnings.push(format!(
                "failed to read remote pattern cache at {}: {e} — falling back to built-in patterns",
                cache_path.display()
            ));
            None
        }
    };

    if let Some(remote) = cache {
        if config.verbose {
            eprintln!("using cached remote patterns");
        }
        merge_patterns(config, &remote);
    } else {
        if config.verbose {
            eprintln!("using built-in patterns");
        }
        merge_patterns(config, crate::builtin::patterns());
    }

    warnings
}

fn merge_patterns(config: &mut ClinkConfig, source: &RemotePatterns) {
    for (name, source_provider) in &source.providers {
        config
            .providers
            .entry(name.clone())
            .and_modify(|local| local.merge_from(source_provider))
            .or_insert_with(|| source_provider.clone());
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
    fn test_resolve_merges_exceptions() {
        let dir = std::env::temp_dir().join("clink_test_resolve_exceptions");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut remote_providers = HashMap::new();
        remote_providers.insert(
            "youtube".to_string(),
            crate::provider::ProviderConfig {
                url_pattern: Some(r"^https?://youtube\.com".into()),
                rules: vec!["feature".into()],
                exceptions: vec![r"^https?://youtube\.com/redirect".into()],
                ..Default::default()
            },
        );
        let remote = RemotePatterns {
            providers: remote_providers,
        };
        std::fs::write(
            dir.join("remote_patterns.toml"),
            toml::to_string(&remote).unwrap(),
        )
        .unwrap();

        let mut local_providers = HashMap::new();
        local_providers.insert(
            "youtube".to_string(),
            crate::provider::ProviderConfig {
                exceptions: vec![r"^https?://youtube\.com/embed".into()],
                ..Default::default()
            },
        );
        let mut cfg = ClinkConfig {
            providers: local_providers,
            ..ClinkConfig::default()
        };

        resolve_patterns(&mut cfg, &dir);

        let youtube = &cfg.providers["youtube"];
        assert!(
            youtube.exceptions.iter().any(|e| e.contains("redirect")),
            "remote exception must be present"
        );
        assert!(
            youtube.exceptions.iter().any(|e| e.contains("embed")),
            "local exception must be preserved"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_falls_back_to_builtin_when_no_cache() {
        let dir = std::env::temp_dir().join("clink_test_resolve_builtin_fallback");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut cfg = ClinkConfig::default();
        // Sanity: default config ships with no providers now.
        assert!(cfg.providers.is_empty());

        resolve_patterns(&mut cfg, &dir);

        let has_fbclid = cfg
            .providers
            .values()
            .any(|p| p.rules.iter().any(|r| r.contains("fbclid")));
        assert!(
            has_fbclid,
            "resolve_patterns without cache must fall back to builtin and populate fbclid"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_returns_warning_on_corrupt_cache() {
        let dir = std::env::temp_dir().join("clink_test_resolve_corrupt_cache");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let cache_path = dir.join("remote_patterns.toml");
        std::fs::write(&cache_path, "this is not valid [[[ toml").unwrap();

        let mut cfg = ClinkConfig::default();
        let warnings = resolve_patterns(&mut cfg, &dir);

        assert!(
            warnings
                .iter()
                .any(|w| w.contains("remote_patterns.toml") && w.contains("parse")),
            "corrupt cache must surface a parse warning naming the cache file, got {warnings:?}"
        );

        // Falls back to builtin so the daemon still has rules.
        let has_fbclid = cfg
            .providers
            .values()
            .any(|p| p.rules.iter().any(|r| r.contains("fbclid")));
        assert!(
            has_fbclid,
            "corrupt cache must still fall back to builtin patterns"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_silent_when_cache_absent() {
        let dir = std::env::temp_dir().join("clink_test_resolve_silent_no_cache");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut cfg = ClinkConfig::default();
        let warnings = resolve_patterns(&mut cfg, &dir);

        assert!(
            warnings.is_empty(),
            "absent cache is the normal first-run state and must not warn, got {warnings:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_dedupes_overlapping_rules() {
        let dir = std::env::temp_dir().join("clink_test_resolve_dedupes_rules");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut remote_providers = HashMap::new();
        remote_providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["shared".into(), "remote_only".into()],
                ..Default::default()
            },
        );
        let remote = RemotePatterns {
            providers: remote_providers,
        };
        std::fs::write(
            dir.join("remote_patterns.toml"),
            toml::to_string(&remote).unwrap(),
        )
        .unwrap();

        let mut local_providers = HashMap::new();
        local_providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["shared".into(), "local_only".into()],
                ..Default::default()
            },
        );
        let mut cfg = ClinkConfig {
            providers: local_providers,
            ..ClinkConfig::default()
        };

        resolve_patterns(&mut cfg, &dir);

        let count = cfg.providers["global"]
            .rules
            .iter()
            .filter(|r| r.as_str() == "shared")
            .count();
        assert_eq!(count, 1, "shared rule must appear exactly once after merge");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_dedupes_overlapping_redirections() {
        let dir = std::env::temp_dir().join("clink_test_resolve_dedupes_redirections");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let shared_redir = r"^https?://exit\.sc/\?.*?url=([^&]+)".to_string();

        let mut remote_providers = HashMap::new();
        remote_providers.insert(
            "exitsc".to_string(),
            crate::provider::ProviderConfig {
                url_pattern: Some(r"^https?://exit\.sc".into()),
                redirections: vec![shared_redir.clone()],
                ..Default::default()
            },
        );
        let remote = RemotePatterns {
            providers: remote_providers,
        };
        std::fs::write(
            dir.join("remote_patterns.toml"),
            toml::to_string(&remote).unwrap(),
        )
        .unwrap();

        let mut local_providers = HashMap::new();
        local_providers.insert(
            "exitsc".to_string(),
            crate::provider::ProviderConfig {
                url_pattern: Some(r"^https?://exit\.sc".into()),
                redirections: vec![shared_redir.clone()],
                ..Default::default()
            },
        );
        let mut cfg = ClinkConfig {
            providers: local_providers,
            ..ClinkConfig::default()
        };

        resolve_patterns(&mut cfg, &dir);

        let count = cfg.providers["exitsc"]
            .redirections
            .iter()
            .filter(|r| **r == shared_redir)
            .count();
        assert_eq!(
            count, 1,
            "shared redirection must appear exactly once after merge"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_dedupes_overlapping_exceptions() {
        let dir = std::env::temp_dir().join("clink_test_resolve_dedupes_exceptions");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let shared_exc = r"^https?://youtube\.com/redirect".to_string();

        let mut remote_providers = HashMap::new();
        remote_providers.insert(
            "youtube".to_string(),
            crate::provider::ProviderConfig {
                url_pattern: Some(r"^https?://youtube\.com".into()),
                exceptions: vec![shared_exc.clone()],
                ..Default::default()
            },
        );
        let remote = RemotePatterns {
            providers: remote_providers,
        };
        std::fs::write(
            dir.join("remote_patterns.toml"),
            toml::to_string(&remote).unwrap(),
        )
        .unwrap();

        let mut local_providers = HashMap::new();
        local_providers.insert(
            "youtube".to_string(),
            crate::provider::ProviderConfig {
                exceptions: vec![shared_exc.clone()],
                ..Default::default()
            },
        );
        let mut cfg = ClinkConfig {
            providers: local_providers,
            ..ClinkConfig::default()
        };

        resolve_patterns(&mut cfg, &dir);

        let count = cfg.providers["youtube"]
            .exceptions
            .iter()
            .filter(|e| **e == shared_exc)
            .count();
        assert_eq!(
            count, 1,
            "shared exception must appear exactly once after merge"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_cache_replaces_builtin() {
        let dir = std::env::temp_dir().join("clink_test_resolve_cache_replaces_builtin");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Cache provides ONLY `only_in_cache`, not any of the builtin rules.
        let mut cache_providers = HashMap::new();
        cache_providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["only_in_cache".into()],
                ..Default::default()
            },
        );
        let cache = RemotePatterns {
            providers: cache_providers,
        };
        let cache_path = dir.join("remote_patterns.toml");
        std::fs::write(&cache_path, toml::to_string(&cache).unwrap()).unwrap();

        let mut cfg = ClinkConfig::default();
        resolve_patterns(&mut cfg, &dir);

        let has_cache_rule = cfg
            .providers
            .values()
            .any(|p| p.rules.iter().any(|r| r == "only_in_cache"));
        let has_builtin_rule = cfg
            .providers
            .values()
            .any(|p| p.rules.iter().any(|r| r.contains("fbclid")));

        assert!(has_cache_rule, "cache rule must be present");
        assert!(
            !has_builtin_rule,
            "builtin must not merge in when cache is present"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
