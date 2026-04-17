use crate::config::load_config;
use crate::remote::{RemoteFormat, RemotePatterns};
use crate::runtime;
use std::path::Path;

pub fn execute(config_path: &Path) -> Result<(), String> {
    let cfg = load_config(config_path)?;

    let remote = cfg.remote.ok_or(
        "No [remote] section in config.\n\
         Add a [remote] section with url and format to use `clink update`.\n\
         Example:\n\n\
         [remote]\n\
         url = 'https://raw.githubusercontent.com/AMinber/ClearUrls/master/data/data.min.json'\n\
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

    let provider_count = patterns.providers.len();
    let rule_count: usize = patterns.providers.values().map(|p| p.rules.len()).sum();

    let cache_path = runtime::data_dir().join("remote_patterns.toml");
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create cache directory: {e}"))?;
    }

    let content = toml::to_string_pretty(&patterns)
        .map_err(|e| format!("Failed to serialize patterns: {e}"))?;
    std::fs::write(&cache_path, &content).map_err(|e| format!("Failed to write cache: {e}"))?;

    println!(
        "Cached {provider_count} providers with {rule_count} rules to {}",
        cache_path.display()
    );

    Ok(())
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
}
