use std::collections::HashSet;

use percent_encoding::percent_decode_str;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProviderConfig {
    #[serde(default)]
    pub url_pattern: Option<String>,
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub redirections: Vec<String>,
}

pub struct CompiledRules {
    literals: HashSet<String>,
    patterns: Vec<Regex>,
}

const REGEX_CHARS: &[char] = &[
    '[', ']', '(', ')', '{', '}', '*', '+', '?', '\\', '|', '^', '$',
];

impl CompiledRules {
    pub fn new(rules: &[String]) -> Self {
        let mut literals = HashSet::new();
        let mut patterns = Vec::new();

        for rule in rules {
            if rule.contains(REGEX_CHARS) {
                if let Ok(re) = Regex::new(rule) {
                    patterns.push(re);
                }
            } else {
                literals.insert(rule.clone());
            }
        }

        Self { literals, patterns }
    }

    pub fn is_tracked(&self, param: &str) -> bool {
        self.literals.contains(param) || self.patterns.iter().any(|re| re.is_match(param))
    }
}

pub struct CompiledProvider {
    url_pattern: Regex,
    pub rules: CompiledRules,
    redirections: Vec<Regex>,
}

impl CompiledProvider {
    pub fn new(config: &ProviderConfig) -> Option<Self> {
        let pattern_str = config.url_pattern.as_ref()?;
        let url_pattern = Regex::new(pattern_str).ok()?;

        let rules = CompiledRules::new(&config.rules);

        let redirections = config
            .redirections
            .iter()
            .filter_map(|r| Regex::new(r).ok())
            .collect();

        Some(Self {
            url_pattern,
            rules,
            redirections,
        })
    }

    pub fn matches_url(&self, url: &str) -> bool {
        self.url_pattern.is_match(url)
    }

    pub fn try_redirect(&self, url: &str) -> Option<String> {
        for re in &self.redirections {
            if let Some(caps) = re.captures(url) {
                if let Some(m) = caps.get(1) {
                    return Some(
                        percent_decode_str(m.as_str())
                            .decode_utf8_lossy()
                            .into_owned(),
                    );
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn provider_config_serde_roundtrip() {
        let mut providers = HashMap::new();
        providers.insert(
            "google".to_string(),
            ProviderConfig {
                url_pattern: Some(r"google\.com".to_string()),
                rules: vec!["utm_source".to_string(), "fbclid".to_string()],
                redirections: vec![],
            },
        );

        let toml_str = toml::to_string(&providers).unwrap();
        let deserialized: HashMap<String, ProviderConfig> = toml::from_str(&toml_str).unwrap();

        assert_eq!(
            deserialized["google"].url_pattern,
            Some(r"google\.com".to_string())
        );
        assert_eq!(deserialized["google"].rules.len(), 2);
    }

    #[test]
    fn provider_config_with_redirections() {
        let config = ProviderConfig {
            url_pattern: Some(r"exit\.sc".to_string()),
            rules: vec![],
            redirections: vec![r"url=([^&]+)".to_string()],
        };

        let mut providers = HashMap::new();
        providers.insert("exit_sc".to_string(), config);

        let toml_str = toml::to_string(&providers).unwrap();
        let deserialized: HashMap<String, ProviderConfig> = toml::from_str(&toml_str).unwrap();

        assert_eq!(deserialized["exit_sc"].redirections.len(), 1);
        assert_eq!(deserialized["exit_sc"].redirections[0], r"url=([^&]+)");
    }

    #[test]
    fn provider_config_minimal_deserialize() {
        let toml_str = r#"
[minimal]
rules = ["fbclid", "gclid"]
"#;
        let deserialized: HashMap<String, ProviderConfig> = toml::from_str(toml_str).unwrap();
        let config = &deserialized["minimal"];

        assert!(config.url_pattern.is_none());
        assert_eq!(config.rules.len(), 2);
        assert!(config.redirections.is_empty());
    }

    #[test]
    fn compiled_rules_matches_literal() {
        let rules = CompiledRules::new(&[
            "fbclid".to_string(),
            "gclid".to_string(),
            "utm_source".to_string(),
        ]);

        assert!(rules.is_tracked("fbclid"));
        assert!(rules.is_tracked("gclid"));
        assert!(rules.is_tracked("utm_source"));
        assert!(!rules.is_tracked("page"));
        assert!(!rules.is_tracked("id"));
    }

    #[test]
    fn compiled_rules_matches_regex() {
        let rules = CompiledRules::new(&["^utm_".to_string()]);

        assert!(rules.is_tracked("utm_source"));
        assert!(rules.is_tracked("utm_campaign"));
        assert!(rules.is_tracked("utm_medium"));
        assert!(!rules.is_tracked("page"));
        assert!(!rules.is_tracked("not_utm_source"));
    }

    #[test]
    fn compiled_rules_invalid_regex_skipped() {
        let rules = CompiledRules::new(&["[invalid".to_string(), "fbclid".to_string()]);

        assert!(rules.is_tracked("fbclid"));
        assert!(!rules.is_tracked("[invalid"));
        assert_eq!(rules.patterns.len(), 0);
    }

    #[test]
    fn compiled_provider_matches_url_pattern() {
        let config = ProviderConfig {
            url_pattern: Some(r"youtube\.com|youtu\.be".to_string()),
            rules: vec!["si".to_string()],
            redirections: vec![],
        };

        let provider = CompiledProvider::new(&config).unwrap();

        assert!(provider.matches_url("https://www.youtube.com/watch?v=abc"));
        assert!(provider.matches_url("https://youtu.be/abc"));
        assert!(!provider.matches_url("https://example.com"));
    }

    #[test]
    fn compiled_provider_extracts_redirect() {
        let config = ProviderConfig {
            url_pattern: Some(r"exit\.sc".to_string()),
            rules: vec![],
            redirections: vec![r"url=([^&]+)".to_string()],
        };

        let provider = CompiledProvider::new(&config).unwrap();
        let result = provider
            .try_redirect("https://exit.sc/?url=https%3A%2F%2Fexample.com%2Fpage%3Fid%3D1")
            .unwrap();

        assert_eq!(result, "https://example.com/page?id=1");
    }

    #[test]
    fn compiled_provider_no_redirect_match() {
        let config = ProviderConfig {
            url_pattern: Some(r"exit\.sc".to_string()),
            rules: vec![],
            redirections: vec![r"url=([^&]+)".to_string()],
        };

        let provider = CompiledProvider::new(&config).unwrap();
        let result = provider.try_redirect("https://exit.sc/?other=value");

        assert!(result.is_none());
    }

    #[test]
    fn compiled_provider_returns_none_for_global() {
        let config = ProviderConfig {
            url_pattern: None,
            rules: vec!["fbclid".to_string()],
            redirections: vec![],
        };

        assert!(CompiledProvider::new(&config).is_none());
    }
}
