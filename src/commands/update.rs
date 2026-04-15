use crate::config::{RemoteFormat, RemotePatterns, load_config};
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

    let param_count = patterns.params.len();
    let exit_count = patterns.exit.len();

    let cache_path = runtime::data_dir().join("remote_patterns.toml");
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create cache directory: {e}"))?;
    }

    let content = toml::to_string_pretty(&patterns)
        .map_err(|e| format!("Failed to serialize patterns: {e}"))?;
    std::fs::write(&cache_path, &content).map_err(|e| format!("Failed to write cache: {e}"))?;

    println!(
        "Cached {param_count} params and {exit_count} exit rules to {}",
        cache_path.display()
    );

    Ok(())
}

fn translate_clearurls(body: &str) -> Result<RemotePatterns, String> {
    let result = crate::clearurls::translate(body)?;

    if result.rules_skipped > 0 {
        println!(
            "Note: {}/{} rules were regex patterns and skipped (only literal param names are supported)",
            result.rules_skipped,
            result.rules_translated + result.rules_skipped
        );
    }
    if result.redirections_skipped > 0 {
        println!(
            "Note: {} redirections skipped (not yet supported)",
            result.redirections_skipped
        );
    }

    println!(
        "ClearURLs data provided by the ClearURLs project (LGPLv3) — https://docs.clearurls.xyz"
    );

    Ok(RemotePatterns {
        params: result.params,
        exit: result.exit,
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
        let toml = "params = ['fbclid', 'gclid']\nexit = [['exit.sc/', 'url']]";
        let result = parse_clink_toml(toml).unwrap();
        assert_eq!(result.params.len(), 2);
        assert_eq!(result.exit.len(), 1);
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
        assert!(result.params.contains("fbclid"));
        assert!(result.params.contains("gclid"));
    }

    #[test]
    fn test_translate_clearurls_invalid() {
        let result = translate_clearurls("not json");
        assert!(result.is_err());
    }
}
