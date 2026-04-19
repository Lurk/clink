use crate::config::load_config;
use crate::remote::{RemoteFormat, RemotePatterns};
use crate::runtime;
use std::path::Path;

pub fn execute(config_path: &Path, write_snapshot: Option<&Path>) -> Result<(), String> {
    let cfg = load_config(config_path)?;

    let remote = cfg.remote.ok_or(
        "No [remote] section in config.\n\
         Add a [remote] section with url and format to use `clink update`.\n\
         Example:\n\n\
         [remote]\n\
         url = 'https://rules2.clearurls.xyz/data.min.json'\n\
         format = 'clearurls'"
            .to_string(),
    )?;

    println!("Fetching patterns from {}", remote.url);

    let body = ureq::get(&remote.url)
        .call()
        .map_err(|e| format!("Failed to fetch remote patterns: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    let patterns = match remote.format {
        RemoteFormat::ClearUrls => translate_clearurls(&body)?,
        RemoteFormat::Clink => parse_clink_toml(&body)?,
    };

    if let Some(snapshot_path) = write_snapshot {
        let (provider_count, rule_count) = write_patterns_to(snapshot_path, &patterns)?;
        println!(
            "Wrote snapshot with {provider_count} providers and {rule_count} rules to {}",
            snapshot_path.display()
        );
    } else {
        let cache_path = runtime::data_dir().join("remote_patterns.toml");
        let (provider_count, rule_count) = write_patterns_to(&cache_path, &patterns)?;
        println!(
            "Cached {provider_count} providers with {rule_count} rules to {}",
            cache_path.display()
        );
    }

    Ok(())
}

fn write_patterns_to(path: &Path, patterns: &RemotePatterns) -> Result<(usize, usize), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
        }
    }
    let content = toml::to_string_pretty(patterns)
        .map_err(|e| format!("Failed to serialize patterns: {e}"))?;
    std::fs::write(path, &content)
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;

    let provider_count = patterns.providers.len();
    let rule_count: usize = patterns.providers.values().map(|p| p.rules.len()).sum();
    Ok((provider_count, rule_count))
}

fn translate_clearurls(body: &str) -> Result<RemotePatterns, String> {
    let result = crate::clearurls::translate(body)?;

    println!(
        "Translated {} providers with {} rules",
        result.providers.len(),
        result.rules_translated
    );
    println!(
        "ClearURLs data provided by the ClearURLs project (LGPLv3) — https://docs.clearurls.xyz"
    );

    Ok(RemotePatterns {
        providers: result.providers,
    })
}

fn parse_clink_toml(body: &str) -> Result<RemotePatterns, String> {
    toml::from_str::<RemotePatterns>(body).map_err(|e| format!("Failed to parse clink TOML: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clink_toml_valid() {
        let toml = r#"
[providers.global]
rules = ['fbclid', 'gclid']

[providers.exitsc]
url_pattern = '^https?://exit\.sc'
redirections = ['^https?://exit\.sc/\?.*?url=([^&]+)']
"#;
        let result = parse_clink_toml(toml).unwrap();
        assert_eq!(result.providers.len(), 2);
        assert_eq!(result.providers["global"].rules.len(), 2);
    }

    #[test]
    fn test_parse_clink_toml_invalid() {
        let result = parse_clink_toml("not valid toml [[[");
        assert!(result.is_err());
    }

    #[test]
    fn test_translate_clearurls_valid() {
        let json = r#"{
            "providers": {
                "test": {
                    "urlPattern": "^https?://test\\.com",
                    "completeProvider": false,
                    "rules": ["fbclid", "gclid"],
                    "referralMarketing": [],
                    "rawRules": [],
                    "exceptions": [],
                    "redirections": [],
                    "forceRedirection": false
                }
            }
        }"#;
        let result = translate_clearurls(json).unwrap();
        let test_provider = &result.providers["test"];
        assert!(test_provider.rules.contains(&"fbclid".to_string()));
        assert!(test_provider.rules.contains(&"gclid".to_string()));
        assert_eq!(
            test_provider.url_pattern.as_deref(),
            Some("^https?://test\\.com")
        );
    }

    #[test]
    fn test_translate_clearurls_invalid() {
        let result = translate_clearurls("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_write_patterns_to_writes_toml() {
        let dir = std::env::temp_dir().join("clink_test_write_patterns_to");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.toml");

        let mut providers = std::collections::HashMap::new();
        providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["fbclid".into(), "gclid".into()],
                ..Default::default()
            },
        );
        let patterns = RemotePatterns { providers };

        let (provider_count, rule_count) = write_patterns_to(&path, &patterns).unwrap();

        assert_eq!(provider_count, 1);
        assert_eq!(rule_count, 2);
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("fbclid"));
        assert!(content.contains("gclid"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_patterns_to_creates_parent_dir() {
        let dir = std::env::temp_dir().join("clink_test_write_patterns_parent/nested");
        let _ =
            std::fs::remove_dir_all(std::env::temp_dir().join("clink_test_write_patterns_parent"));
        let path = dir.join("out.toml");

        let patterns = RemotePatterns {
            providers: std::collections::HashMap::new(),
        };
        write_patterns_to(&path, &patterns).unwrap();
        assert!(path.is_file());

        let _ =
            std::fs::remove_dir_all(std::env::temp_dir().join("clink_test_write_patterns_parent"));
    }
}
