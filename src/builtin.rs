use std::sync::OnceLock;

use crate::remote::RemotePatterns;

const BUILTIN_TOML: &str = include_str!("builtin_patterns.toml");

pub fn patterns() -> &'static RemotePatterns {
    static PATTERNS: OnceLock<RemotePatterns> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        toml::from_str(BUILTIN_TOML)
            .expect("src/builtin_patterns.toml is not valid RemotePatterns TOML; regenerate it with scripts/refresh-snapshot.sh")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_patterns_parses() {
        let p = patterns();
        assert!(
            !p.providers.is_empty(),
            "builtin snapshot must be non-empty"
        );
    }

    #[test]
    fn test_builtin_contains_common_trackers() {
        // ClearURLs uses regex forms for global rules (e.g. "(?:%3F)?fbclid"),
        // so we look for rules whose pattern text contains the tracker name.
        let p = patterns();
        let contains_rule = |needle: &str| {
            p.providers
                .values()
                .any(|prov| prov.rules.iter().any(|r| r.contains(needle)))
        };

        assert!(
            contains_rule("fbclid"),
            "builtin snapshot must cover fbclid"
        );
        assert!(contains_rule("gclid"), "builtin snapshot must cover gclid");
        assert!(
            contains_rule("utm"),
            "builtin snapshot must cover utm_* params"
        );
    }
}
