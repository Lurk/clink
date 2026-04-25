use std::collections::HashMap;

use crate::expand_string::expand_string;
use crate::provider::ProviderConfig;

fn domain_to_provider_name(domain: &str) -> String {
    domain.replace(['.', '/'], "_")
}

fn domain_to_url_pattern(domain: &str) -> String {
    let escaped = regex::escape(domain);
    format!(r"^https?://([a-z0-9-]+\.)*?{escaped}(?:[/:?#]|$)")
}

pub fn migrate_params(params: &[String]) -> HashMap<String, ProviderConfig> {
    let mut providers: HashMap<String, ProviderConfig> = HashMap::new();

    for param in params {
        if let Some((domain_pattern, param_pattern)) = param.split_once("``") {
            let expanded_domains = expand_string(domain_pattern);
            let expanded_params = expand_string(param_pattern);

            for domain in &expanded_domains {
                let name = domain_to_provider_name(domain);
                let provider = providers.entry(name).or_insert_with(|| ProviderConfig {
                    url_pattern: Some(domain_to_url_pattern(domain)),
                    ..Default::default()
                });
                for p in &expanded_params {
                    if !provider.rules.contains(p) {
                        provider.rules.push(p.clone());
                    }
                }
            }
        } else {
            let global = providers.entry("global".to_string()).or_default();
            if !global.rules.contains(param) {
                global.rules.push(param.clone());
            }
        }
    }

    providers
}

pub fn migrate_exits(exits: &[Vec<String>]) -> HashMap<String, ProviderConfig> {
    let mut providers: HashMap<String, ProviderConfig> = HashMap::new();

    for entry in exits {
        if entry.len() < 2 {
            continue;
        }

        let url_pattern = &entry[0];
        let params: &[String] = &entry[1..];

        let expanded_urls = expand_string(url_pattern);

        for url in &expanded_urls {
            let domain_part = url.split('/').next().unwrap_or(url);
            let name = domain_to_provider_name(domain_part);

            let escaped_url = regex::escape(url);
            let pattern = format!(r"^https?://{escaped_url}");

            let param_alternation = if params.len() == 1 {
                params[0].clone()
            } else {
                format!("(?:{})", params.join("|"))
            };
            let redirection = format!(r"^https?://{escaped_url}\?.*?{param_alternation}=([^&]+)");

            let provider = providers.entry(name).or_insert_with(|| ProviderConfig {
                url_pattern: Some(pattern.clone()),
                ..Default::default()
            });
            provider.redirections.push(redirection);
        }
    }

    providers
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn migrate_global_params() {
        let params = vec!["fbclid".into(), "gclid".into()];
        let result = migrate_params(&params);

        assert_eq!(result.len(), 1);
        let global = &result["global"];
        assert!(global.url_pattern.is_none());
        assert_eq!(global.rules, vec!["fbclid", "gclid"]);
    }

    #[test]
    fn migrate_global_params_deduplicates() {
        let params = vec!["fbclid".into(), "gclid".into(), "fbclid".into()];
        let result = migrate_params(&params);

        let global = &result["global"];
        assert_eq!(
            global.rules,
            vec!["fbclid", "gclid"],
            "duplicate global params must collapse to a single entry"
        );
    }

    #[test]
    fn migrate_domain_scoped_params() {
        let params = vec!["youtube.com``si".into()];
        let result = migrate_params(&params);

        assert_eq!(result.len(), 1);
        let provider = &result["youtube_com"];
        assert!(provider.url_pattern.as_ref().unwrap().contains("youtube"));
        assert_eq!(provider.rules, vec!["si"]);

        let re = Regex::new(provider.url_pattern.as_ref().unwrap()).unwrap();
        assert!(re.is_match("https://youtube.com/watch?v=abc"));
        assert!(re.is_match("https://www.youtube.com/watch?v=abc"));
    }

    #[test]
    fn migrate_expand_string_domain_params() {
        let params = vec!["amazon.(com|de)``(sp_csd|t)".into()];
        let result = migrate_params(&params);

        assert_eq!(result.len(), 2);

        let com = &result["amazon_com"];
        assert!(com.rules.contains(&"sp_csd".to_string()));
        assert!(com.rules.contains(&"t".to_string()));

        let de = &result["amazon_de"];
        assert!(de.rules.contains(&"sp_csd".to_string()));
        assert!(de.rules.contains(&"t".to_string()));

        let re = Regex::new(com.url_pattern.as_ref().unwrap()).unwrap();
        assert!(re.is_match("https://amazon.com/dp/something"));
        assert!(re.is_match("https://www.amazon.com/dp/something"));
    }

    #[test]
    fn migrate_mixed_params() {
        let params = vec!["fbclid".into(), "youtube.com``si".into(), "gclid".into()];
        let result = migrate_params(&params);

        assert!(result.contains_key("global"));
        assert!(result.contains_key("youtube_com"));

        let global = &result["global"];
        assert_eq!(global.rules, vec!["fbclid", "gclid"]);
    }

    #[test]
    fn migrate_exit_simple() {
        let exits = vec![vec!["exit.sc/".into(), "url".into()]];
        let result = migrate_exits(&exits);

        assert_eq!(result.len(), 1);
        let provider = &result["exit_sc"];
        assert!(provider.url_pattern.is_some());
        assert_eq!(provider.redirections.len(), 1);

        let re = Regex::new(&provider.redirections[0]).unwrap();
        let url = "https://exit.sc/?url=https%3A%2F%2Fexample.com";
        let caps = re.captures(url).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "https%3A%2F%2Fexample.com");
    }

    #[test]
    fn migrate_exit_multiple_params() {
        let exits = vec![vec!["facebook.com/l.php".into(), "u".into(), "next".into()]];
        let result = migrate_exits(&exits);

        let provider = &result["facebook_com"];
        let re = Regex::new(&provider.redirections[0]).unwrap();

        let url_u = "https://facebook.com/l.php?u=https%3A%2F%2Fexample.com&h=abc";
        let caps = re.captures(url_u).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "https%3A%2F%2Fexample.com");

        let url_next = "https://facebook.com/l.php?next=https%3A%2F%2Fexample.com&h=abc";
        let caps = re.captures(url_next).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "https%3A%2F%2Fexample.com");
    }

    #[test]
    fn migrate_exit_with_expand_string() {
        let exits = vec![vec!["(www.|)google.(com|de)/url".into(), "url".into()]];
        let result = migrate_exits(&exits);

        assert!(result.contains_key("www_google_com"));
        assert!(result.contains_key("www_google_de"));
        assert!(result.contains_key("google_com"));
        assert!(result.contains_key("google_de"));

        let provider = &result["google_com"];
        let re = Regex::new(&provider.redirections[0]).unwrap();
        let url = "https://google.com/url?url=https%3A%2F%2Fexample.com";
        let caps = re.captures(url).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "https%3A%2F%2Fexample.com");
    }

    #[test]
    fn migrate_empty_exit_skipped() {
        let exits: Vec<Vec<String>> = vec![vec![], vec!["only_url".into()]];
        let result = migrate_exits(&exits);
        assert!(result.is_empty());
    }

    #[test]
    fn domain_to_url_pattern_anchors_host_end() {
        let result = migrate_params(&["youtube.com``si".into()]);
        let pattern = result["youtube_com"].url_pattern.as_ref().unwrap();
        let re = Regex::new(pattern).unwrap();

        // Legitimate hosts must match.
        assert!(re.is_match("https://youtube.com/watch?v=abc"));
        assert!(re.is_match("https://www.youtube.com/watch?v=abc"));
        assert!(re.is_match("https://music.youtube.com/watch?v=abc"));
        assert!(re.is_match("https://youtube.com:8080/watch"));
        assert!(re.is_match("https://youtube.com?si=foo"));
        assert!(re.is_match("https://youtube.com"));

        // Look-alike hosts must NOT match — this is the host-end-anchor bug.
        assert!(
            !re.is_match("https://youtube.com.attacker.com/?si=foo"),
            "pattern over-matches youtube.com.attacker.com — host-end anchor missing"
        );
        assert!(
            !re.is_match("https://youtube.commerce.com/?si=foo"),
            "pattern over-matches youtube.commerce.com — host-end anchor missing"
        );
    }
}
