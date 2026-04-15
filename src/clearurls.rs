use std::collections::HashSet;
use std::rc::Rc;

use serde::Deserialize;

#[derive(Deserialize)]
struct ClearUrlsData {
    providers: std::collections::HashMap<String, Provider>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_field_names)]
struct Provider {
    #[allow(dead_code)]
    url_pattern: String,
    complete_provider: bool,
    #[serde(default)]
    rules: Vec<String>,
    #[serde(default)]
    referral_marketing: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)]
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
    pub params: HashSet<String>,
    pub exit: Vec<Vec<Rc<str>>>,
    pub rules_translated: usize,
    pub rules_skipped: usize,
    pub redirections_skipped: usize,
}

pub fn translate(json: &str) -> Result<TranslationResult, String> {
    let data: ClearUrlsData =
        serde_json::from_str(json).map_err(|e| format!("Failed to parse ClearURLs JSON: {e}"))?;

    let mut params = HashSet::new();
    let mut rules_translated = 0usize;
    let mut rules_skipped = 0usize;
    let mut redirections_skipped = 0usize;

    for provider in data.providers.values() {
        if provider.complete_provider {
            continue;
        }

        for rule in provider
            .rules
            .iter()
            .chain(provider.referral_marketing.iter())
        {
            if is_literal(rule) {
                params.insert(rule.clone());
                rules_translated += 1;
            } else {
                rules_skipped += 1;
            }
        }

        redirections_skipped += provider.redirections.len();
    }

    Ok(TranslationResult {
        params,
        exit: Vec::new(),
        rules_translated,
        rules_skipped,
        redirections_skipped,
    })
}

fn is_literal(rule: &str) -> bool {
    !rule.chars().any(|c| {
        matches!(
            c,
            '[' | ']' | '(' | ')' | '{' | '}' | '*' | '+' | '?' | '\\' | '|' | '^' | '$'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_provider_json(rules: &[&str], referral: &[&str]) -> String {
        let rules_json: Vec<String> = rules.iter().map(|r| format!("\"{r}\"")).collect();
        let referral_json: Vec<String> = referral.iter().map(|r| format!("\"{r}\"")).collect();
        format!(
            r#"{{
                "providers": {{
                    "test": {{
                        "urlPattern": "^https?://example\\.com",
                        "completeProvider": false,
                        "rules": [{}],
                        "referralMarketing": [{}],
                        "rawRules": [],
                        "exceptions": [],
                        "redirections": [],
                        "forceRedirection": false
                    }}
                }}
            }}"#,
            rules_json.join(","),
            referral_json.join(",")
        )
    }

    #[test]
    fn translates_literal_rules_to_params() {
        let json = make_provider_json(&["utm_source", "fbclid"], &["ref"]);
        let result = translate(&json).unwrap();
        assert!(result.params.contains("utm_source"));
        assert!(result.params.contains("fbclid"));
        assert!(result.params.contains("ref"));
        assert_eq!(result.rules_translated, 3);
        assert_eq!(result.rules_skipped, 0);
    }

    #[test]
    fn skips_regex_rules() {
        let json = make_provider_json(&["utm_source", "gfe_[a-z]*", "bi[a-z]*"], &[]);
        let result = translate(&json).unwrap();
        assert!(result.params.contains("utm_source"));
        assert!(!result.params.contains("gfe_[a-z]*"));
        assert_eq!(result.rules_translated, 1);
        assert_eq!(result.rules_skipped, 2);
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
        assert!(result.params.is_empty());
    }

    #[test]
    fn invalid_json_returns_error() {
        let result = translate("not json");
        assert!(result.is_err());
    }

    #[test]
    fn merges_params_from_multiple_providers() {
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
        assert!(result.params.contains("gclid"));
        assert!(result.params.contains("ved"));
        assert!(result.params.contains("fbclid"));
        assert!(result.params.contains("ref"));
        assert_eq!(result.rules_translated, 4);
    }
}
