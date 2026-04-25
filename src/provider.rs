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
    #[serde(default)]
    pub exceptions: Vec<String>,
}

pub struct CompiledRules {
    literals: HashSet<String>,
    patterns: Vec<Regex>,
}

const REGEX_CHARS: &[char] = &[
    '[', ']', '(', ')', '{', '}', '*', '+', '?', '\\', '|', '^', '$',
];

// ClearURLs wraps every rule as `^(?:rule)$` with the `i` flag before testing
// against a parameter name (see ClearURLs/Addon clearurls.js:122). The rules
// data is authored assuming that wrapping. clink consumes the same data, so
// it must wrap identically — without it, `(?:ref_?)?src` substring-matches
// `srcset`, and case-permuted params like `?Fbclid=` slip through.
fn wrap_rule(rule: &str) -> String {
    format!("(?i)^(?:{rule})$")
}

fn case_insensitive(pattern: &str) -> String {
    format!("(?i){pattern}")
}

impl CompiledRules {
    pub fn new(rules: &[String]) -> Self {
        let mut literals = HashSet::new();
        let mut patterns = Vec::new();

        for rule in rules {
            if rule.contains(REGEX_CHARS) {
                if let Ok(re) = Regex::new(&wrap_rule(rule)) {
                    patterns.push(re);
                }
            } else {
                literals.insert(rule.to_lowercase());
            }
        }

        Self { literals, patterns }
    }

    pub fn is_tracked(&self, param: &str) -> bool {
        let lower = param.to_lowercase();
        self.literals.contains(&lower) || self.patterns.iter().any(|re| re.is_match(param))
    }
}

pub struct CompiledProvider {
    url_pattern: Regex,
    pub rules: CompiledRules,
    redirections: Vec<Regex>,
    exceptions: Vec<Regex>,
}

pub fn check_provider(name: &str, config: &ProviderConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    if let Some(pattern) = &config.url_pattern {
        if let Err(e) = Regex::new(&case_insensitive(pattern)) {
            warnings.push(format!(
                "[providers.{name}] url_pattern '{pattern}' failed to compile: {e}"
            ));
        }
    }
    for rule in &config.rules {
        if rule.contains(REGEX_CHARS) {
            if let Err(e) = Regex::new(&wrap_rule(rule)) {
                warnings.push(format!(
                    "[providers.{name}] rule '{rule}' failed to compile: {e}"
                ));
            }
        }
    }
    for redir in &config.redirections {
        if let Err(e) = Regex::new(&case_insensitive(redir)) {
            warnings.push(format!(
                "[providers.{name}] redirection '{redir}' failed to compile: {e}"
            ));
        }
    }
    for exc in &config.exceptions {
        if let Err(e) = Regex::new(&case_insensitive(exc)) {
            warnings.push(format!(
                "[providers.{name}] exception '{exc}' failed to compile: {e}"
            ));
        }
    }
    warnings
}

impl CompiledProvider {
    pub fn new(config: &ProviderConfig) -> Option<Self> {
        let pattern_str = config.url_pattern.as_ref()?;
        let url_pattern = Regex::new(&case_insensitive(pattern_str)).ok()?;

        let rules = CompiledRules::new(&config.rules);

        let redirections = config
            .redirections
            .iter()
            .filter_map(|r| Regex::new(&case_insensitive(r)).ok())
            .collect();

        let exceptions = config
            .exceptions
            .iter()
            .filter_map(|r| Regex::new(&case_insensitive(r)).ok())
            .collect();

        Some(Self {
            url_pattern,
            rules,
            redirections,
            exceptions,
        })
    }

    pub fn matches_url(&self, url: &str) -> bool {
        self.url_pattern.is_match(url) && !self.exceptions.iter().any(|re| re.is_match(url))
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
                exceptions: vec![],
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
            exceptions: vec![],
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
        // Rules are wrapped as ^(?:rule)$ (ClearURLs semantics), so a
        // user-authored rule must spell out what comes after the prefix.
        let rules = CompiledRules::new(&["utm_[a-z]+".to_string()]);

        assert!(rules.is_tracked("utm_source"));
        assert!(rules.is_tracked("utm_campaign"));
        assert!(rules.is_tracked("utm_medium"));
        assert!(!rules.is_tracked("page"));
        assert!(!rules.is_tracked("not_utm_source"));
    }

    #[test]
    fn compiled_rules_regex_is_anchored() {
        // ClearURLs wraps every rule as ^…$ before testing it against a
        // parameter name. Clink consumes the same data and must match. The
        // twitter rule `(?:ref_?)?src` should match `src`, `refsrc`,
        // `ref_src` — but NOT substring-match `source`.
        let rules = CompiledRules::new(&["(?:ref_?)?src".to_string()]);
        assert!(rules.is_tracked("src"));
        assert!(rules.is_tracked("refsrc"));
        assert!(rules.is_tracked("ref_src"));
        assert!(
            !rules.is_tracked("source"),
            "anchored regex must not substring-match 'source'"
        );
        assert!(
            !rules.is_tracked("data-src"),
            "anchored regex must not substring-match"
        );
    }

    #[test]
    fn compiled_rules_match_is_case_insensitive() {
        let rules = CompiledRules::new(&["fbclid".to_string(), "utm_[a-z]+".to_string()]);
        assert!(
            rules.is_tracked("Fbclid"),
            "literal lookup must be case-insensitive"
        );
        assert!(rules.is_tracked("FBCLID"));
        assert!(
            rules.is_tracked("UTM_source"),
            "regex lookup must be case-insensitive"
        );
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
            exceptions: vec![],
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
            exceptions: vec![],
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
            exceptions: vec![],
        };

        let provider = CompiledProvider::new(&config).unwrap();
        let result = provider.try_redirect("https://exit.sc/?other=value");

        assert!(result.is_none());
    }

    #[test]
    fn compiled_provider_respects_exceptions() {
        let config = ProviderConfig {
            url_pattern: Some(r"^https?://youtube\.com".into()),
            rules: vec!["utm_source".into()],
            redirections: vec![],
            exceptions: vec![r"^https?://youtube\.com/redirect".into()],
        };
        let provider = CompiledProvider::new(&config).unwrap();
        assert!(provider.matches_url("https://youtube.com/watch?v=abc"));
        assert!(
            !provider.matches_url("https://youtube.com/redirect?q=abc"),
            "exception must exclude URL from provider match"
        );
    }

    #[test]
    fn compiled_provider_skips_redirect_when_excepted() {
        let config = ProviderConfig {
            url_pattern: Some(r"^https?://exit\.sc".into()),
            rules: vec![],
            redirections: vec![r"url=([^&]+)".into()],
            exceptions: vec![r"^https?://exit\.sc/admin".into()],
        };
        let provider = CompiledProvider::new(&config).unwrap();
        assert!(
            !provider.matches_url("https://exit.sc/admin/?url=https%3A%2F%2Fbar.com"),
            "excepted URL must not be treated as matching"
        );
    }

    #[test]
    fn compiled_provider_url_pattern_is_case_insensitive() {
        let config = ProviderConfig {
            url_pattern: Some(r"^https?://youtube\.com".to_string()),
            ..Default::default()
        };
        let provider = CompiledProvider::new(&config).unwrap();
        assert!(
            provider.matches_url("https://YouTube.com/watch"),
            "url_pattern must match host case-insensitively"
        );
    }

    #[test]
    fn compiled_provider_redirection_is_case_insensitive() {
        let config = ProviderConfig {
            url_pattern: Some(r"^https?://exit\.sc".to_string()),
            redirections: vec![r"url=([^&]+)".to_string()],
            ..Default::default()
        };
        let provider = CompiledProvider::new(&config).unwrap();
        let result = provider
            .try_redirect("https://exit.sc/?URL=https%3A%2F%2Fexample.com")
            .expect("uppercase URL= should still capture");
        assert_eq!(result, "https://example.com");
    }

    #[test]
    fn compiled_provider_exception_is_case_insensitive() {
        // Isolate exception case-sensitivity: url_pattern matches as-is, only
        // the path differs in case. Without (?i) on the exception, the
        // exception silently fails to apply.
        let config = ProviderConfig {
            url_pattern: Some(r"^https?://exit\.sc".to_string()),
            exceptions: vec![r"^https?://exit\.sc/admin".to_string()],
            ..Default::default()
        };
        let provider = CompiledProvider::new(&config).unwrap();
        assert!(
            !provider.matches_url("https://exit.sc/ADMIN/?url=foo"),
            "exception must apply case-insensitively"
        );
    }

    #[test]
    fn compiled_provider_returns_none_for_global() {
        let config = ProviderConfig {
            url_pattern: None,
            rules: vec!["fbclid".to_string()],
            redirections: vec![],
            exceptions: vec![],
        };

        assert!(CompiledProvider::new(&config).is_none());
    }

    #[test]
    fn check_provider_returns_empty_for_clean_config() {
        let config = ProviderConfig {
            url_pattern: Some(r"^https?://example\.com".to_string()),
            rules: vec!["fbclid".to_string(), "^utm_".to_string()],
            redirections: vec![r"url=([^&]+)".to_string()],
            exceptions: vec![],
        };
        assert!(check_provider("test", &config).is_empty());
    }

    #[test]
    fn check_provider_skips_url_pattern_when_absent() {
        let config = ProviderConfig {
            url_pattern: None,
            rules: vec!["fbclid".to_string()],
            redirections: vec![],
            exceptions: vec![],
        };
        assert!(check_provider("global", &config).is_empty());
    }

    #[test]
    fn check_provider_flags_bad_url_pattern() {
        let config = ProviderConfig {
            url_pattern: Some("[invalid".to_string()),
            ..Default::default()
        };
        let warnings = check_provider("oops", &config);
        assert_eq!(
            warnings.len(),
            1,
            "expected a single warning, got {warnings:?}"
        );
        assert!(warnings[0].contains("oops"));
        assert!(warnings[0].contains("url_pattern"));
        assert!(warnings[0].contains("[invalid"));
    }

    #[test]
    fn check_provider_flags_bad_rule_regex() {
        let config = ProviderConfig {
            url_pattern: Some(r"^https?://x\.com".to_string()),
            rules: vec!["fbclid".to_string(), "[bad".to_string()],
            ..Default::default()
        };
        let warnings = check_provider("scoped", &config);
        assert_eq!(
            warnings.len(),
            1,
            "expected a single warning, got {warnings:?}"
        );
        assert!(warnings[0].contains("scoped"));
        assert!(warnings[0].contains("rule"));
        assert!(warnings[0].contains("[bad"));
    }

    #[test]
    fn check_provider_flags_bad_redirection() {
        let config = ProviderConfig {
            url_pattern: Some(r"^https?://x\.com".to_string()),
            redirections: vec!["[bad".to_string()],
            ..Default::default()
        };
        let warnings = check_provider("scoped", &config);
        assert_eq!(
            warnings.len(),
            1,
            "expected a single warning, got {warnings:?}"
        );
        assert!(warnings[0].contains("redirection"));
        assert!(warnings[0].contains("[bad"));
    }

    #[test]
    fn check_provider_flags_bad_exception() {
        let config = ProviderConfig {
            url_pattern: Some(r"^https?://x\.com".to_string()),
            exceptions: vec!["[bad".to_string()],
            ..Default::default()
        };
        let warnings = check_provider("scoped", &config);
        assert_eq!(
            warnings.len(),
            1,
            "expected a single warning, got {warnings:?}"
        );
        assert!(warnings[0].contains("exception"));
        assert!(warnings[0].contains("[bad"));
    }

    #[test]
    fn check_provider_skips_literal_rules() {
        // Literal rules never go through Regex::new, so they can't fail.
        let config = ProviderConfig {
            url_pattern: Some(r"^https?://x\.com".to_string()),
            rules: vec!["fbclid".to_string(), "gclid".to_string()],
            ..Default::default()
        };
        assert!(check_provider("scoped", &config).is_empty());
    }
}
