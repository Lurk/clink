use std::collections::HashMap;

use serde::Deserialize;

use crate::provider::ProviderConfig;

#[derive(Deserialize)]
struct ClearUrlsData {
    providers: HashMap<String, ClearUrlsProvider>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_field_names)]
struct ClearUrlsProvider {
    url_pattern: String,
    complete_provider: bool,
    #[serde(default)]
    rules: Vec<String>,
    #[serde(default)]
    referral_marketing: Vec<String>,
    #[serde(default)]
    redirections: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)]
    raw_rules: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)]
    exceptions: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)]
    force_redirection: bool,
}

pub struct TranslationResult {
    pub providers: HashMap<String, ProviderConfig>,
    pub rules_translated: usize,
}

pub fn translate(json: &str) -> Result<TranslationResult, String> {
    let data: ClearUrlsData =
        serde_json::from_str(json).map_err(|e| format!("Failed to parse ClearURLs JSON: {e}"))?;

    let mut providers = HashMap::new();
    let mut rules_translated = 0usize;

    for (name, cu_provider) in &data.providers {
        if cu_provider.complete_provider {
            continue;
        }

        let mut rules: Vec<String> = Vec::new();
        for rule in cu_provider
            .rules
            .iter()
            .chain(cu_provider.referral_marketing.iter())
        {
            rules.push(rule.clone());
            rules_translated += 1;
        }

        let (clink_name, url_pattern) = if name == "globalRules" {
            ("global".to_string(), None)
        } else {
            (name.clone(), Some(cu_provider.url_pattern.clone()))
        };

        providers.insert(
            clink_name,
            ProviderConfig {
                url_pattern,
                rules,
                redirections: cu_provider.redirections.clone(),
            },
        );
    }

    Ok(TranslationResult {
        providers,
        rules_translated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_provider_json(
        name: &str,
        url_pattern: &str,
        rules: &[&str],
        referral: &[&str],
        redirections: &[&str],
    ) -> String {
        let rules_json: Vec<String> = rules.iter().map(|r| format!("\"{r}\"")).collect();
        let referral_json: Vec<String> = referral.iter().map(|r| format!("\"{r}\"")).collect();
        let redir_json: Vec<String> = redirections.iter().map(|r| format!("\"{r}\"")).collect();
        format!(
            r#"{{
                "providers": {{
                    "{name}": {{
                        "urlPattern": "{url_pattern}",
                        "completeProvider": false,
                        "rules": [{rules}],
                        "referralMarketing": [{referral}],
                        "rawRules": [],
                        "exceptions": [],
                        "redirections": [{redirections}],
                        "forceRedirection": false
                    }}
                }}
            }}"#,
            rules = rules_json.join(","),
            referral = referral_json.join(","),
            redirections = redir_json.join(",")
        )
    }

    #[test]
    fn translates_rules_to_provider() {
        let json = make_provider_json(
            "test",
            "^https?://example\\\\.com",
            &["utm_source", "fbclid"],
            &["ref"],
            &[],
        );
        let result = translate(&json).unwrap();
        assert!(result.providers.contains_key("test"));
        let provider = &result.providers["test"];
        assert!(provider.rules.contains(&"utm_source".into()));
        assert!(provider.rules.contains(&"fbclid".into()));
        assert!(provider.rules.contains(&"ref".into()));
        assert_eq!(
            provider.url_pattern.as_deref(),
            Some("^https?://example\\.com")
        );
        assert_eq!(result.rules_translated, 3);
    }

    #[test]
    fn includes_regex_rules() {
        let json = make_provider_json(
            "test",
            "^https?://example\\\\.com",
            &["utm_source", "gfe_[a-z]*"],
            &[],
            &[],
        );
        let result = translate(&json).unwrap();
        let provider = &result.providers["test"];
        assert!(provider.rules.contains(&"utm_source".into()));
        assert!(provider.rules.contains(&"gfe_[a-z]*".into()));
        assert_eq!(result.rules_translated, 2);
    }

    #[test]
    fn translates_redirections() {
        let json = make_provider_json(
            "google",
            "^https?://google\\\\.com",
            &[],
            &[],
            &["^https?://google\\\\.com/url\\\\?.*?q=([^&]+)"],
        );
        let result = translate(&json).unwrap();
        let provider = &result.providers["google"];
        assert_eq!(provider.redirections.len(), 1);
    }

    #[test]
    fn skips_complete_providers() {
        let json = r#"{
            "providers": {
                "blocked": {
                    "urlPattern": "^https?://blocked\\.com",
                    "completeProvider": true,
                    "rules": ["should_be_skipped"],
                    "referralMarketing": [],
                    "rawRules": [],
                    "exceptions": [],
                    "redirections": [],
                    "forceRedirection": false
                }
            }
        }"#;
        let result = translate(json).unwrap();
        assert!(result.providers.is_empty());
    }

    #[test]
    fn invalid_json_returns_error() {
        let result = translate("not json");
        assert!(result.is_err());
    }

    #[test]
    fn multiple_providers_stay_separate() {
        let json = r#"{
            "providers": {
                "google": {
                    "urlPattern": "^https?://google\\.com",
                    "completeProvider": false,
                    "rules": ["gclid", "ved"],
                    "referralMarketing": [],
                    "rawRules": [],
                    "exceptions": [],
                    "redirections": [],
                    "forceRedirection": false
                },
                "facebook": {
                    "urlPattern": "^https?://facebook\\.com",
                    "completeProvider": false,
                    "rules": ["fbclid"],
                    "referralMarketing": ["ref"],
                    "rawRules": [],
                    "exceptions": [],
                    "redirections": [],
                    "forceRedirection": false
                }
            }
        }"#;
        let result = translate(json).unwrap();
        assert_eq!(result.providers.len(), 2);
        assert!(result.providers.contains_key("google"));
        assert!(result.providers.contains_key("facebook"));
        assert!(result.providers["google"].rules.contains(&"gclid".into()));
        assert!(
            result.providers["facebook"]
                .rules
                .contains(&"fbclid".into())
        );
        assert!(result.providers["facebook"].rules.contains(&"ref".into()));
        assert_eq!(result.rules_translated, 4);
    }
}
