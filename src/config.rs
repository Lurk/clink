use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    rc::Rc,
};

use serde::{Deserialize, Serialize};

use crate::expand_string::expand_string;
use crate::mode::Mode;

/// Add query param, that must be replaced within any domain.
/// To specify domain specific params use format
/// `"{domain}``{param}"`.
///
/// Patterns may use `expand_string` notation (e.g. `(a|b)`) which is
/// expanded at runtime. Use `get_default_raw_params` when you need the
/// compact, pre-expansion form (e.g. for display in diffs).
const DEFAULT_RAW_PARAMS: &[&str] = &[
    // Google
    "dclid",
    "gclid",
    "gclsrc",
    "_ga",
    "_gl",
    // Meta (Facebook/Instagram)
    "fbclid",
    "igshid",
    "igsh",
    // Microsoft/Bing
    "msclkid",
    // Twitter/X
    "twclid",
    // TikTok
    "ttclid",
    // LinkedIn
    "li_fat_id",
    // Yandex
    "yclid",
    // UTM family
    "utm_id",
    "utm_source",
    "utm_source_platform",
    "utm_creative_format",
    "utm_medium",
    "utm_term",
    "utm_campaign",
    "utm_content",
    // Awin (formerly Zanox)
    "zanpid",
    // Mailchimp
    "mc_cid",
    "mc_eid",
    // HubSpot
    "_hsenc",
    "_hsmi",
    // Marketo
    "mkt_tok",
    // Drip
    "__s",
    // Openstat
    "_openstat",
    // Vero
    "vero_id",
    // Alibaba/AliExpress
    "spm",
    // YouTube (domain-specific)
    "youtube.com``si",
    "youtu.be``si",
    "music.youtube.com``si",
    // Amazon tracking params across all domains
    "amazon.(com|de|co.uk|co.jp|fr|it|es|ca|com.au|com.br|com.mx|nl|pl|se|sg|in|com.be|com.tr|eg|sa|ae)``(sp_csd|pd_rd_w|pd_rd_wg|pd_rd_i|pd_rd_r|pf_rd_r|pf_rd_p|t|psc|content-id)",
];

pub fn get_default_raw_params() -> &'static [&'static str] {
    DEFAULT_RAW_PARAMS
}

fn get_default_params() -> HashSet<String> {
    DEFAULT_RAW_PARAMS
        .iter()
        .flat_map(|entry| expand_string(entry))
        .collect()
}

fn get_default_exit() -> Vec<Vec<Rc<str>>> {
    vec![
    vec!["vk.com/away.php".into(), "to".into()],
    vec!["exit.sc/".into(), "url".into()],
    vec!["facebook.com/(l|confirmemail|login).php".into(), "u".into(), "next".into()],
    vec!["(www.|)google.(at|be|ca|ch|co.(bw|id|il|in|jp|kr|nz|th|uk|za)|com(|.(ar|au|br|co|eg|hk|mx|sg|tr|tw|ua))|cl|cz|de|dk|es|fi|fr|gr|hu|ie|it|nl|no|pl|pt|ro|se)/url".into(), "url".into(), "q".into()],
    vec!["bing.com/ck/a".into(), "u".into()],
    vec!["l.instagram.com/".into(), "u".into()],
    vec!["youtube.com/redirect".into(), "q".into()],
    vec!["linkedin.com/authwall".into(), "sessionRedirect".into()],
    vec!["mora.jp/cart".into(), "returnUrl".into()],
    ]
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClinkConfig {
    pub mode: Mode,
    pub replace_to: String,
    pub sleep_duration: u64,
    pub params: HashSet<String>,
    pub exit: Vec<Vec<Rc<str>>>,
    #[serde(skip)]
    pub verbose: bool,
}

impl ClinkConfig {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            replace_to: "clink".into(),
            sleep_duration: 150,
            params: get_default_params(),
            exit: get_default_exit(),
            verbose: false,
        }
    }

    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.sleep_duration == 0 {
            warnings.push("sleep_duration is 0, this will cause 100% CPU usage".to_string());
        }
        if self.params.is_empty() {
            warnings.push("No tracking params configured — clink won't clean anything".to_string());
        }
        warnings
    }
}

impl Default for ClinkConfig {
    fn default() -> Self {
        Self::new(Mode::Remove)
    }
}

pub fn load_config(config_path: &Path) -> Result<ClinkConfig, String> {
    let path = config_path.display();
    let mut config: ClinkConfig = confy::load_path(config_path).map_err(|e| {
        format!(
            "Config error at {path}: {e}\n\n\
             Looks like you have a bad config or config for an old version.\n\
             Config should look like this:\n\n{}",
            toml::to_string_pretty(&ClinkConfig::default()).unwrap()
        )
    })?;

    config.params = config
        .params
        .iter()
        .flat_map(|entry| expand_string(entry))
        .collect();

    Ok(config)
}

pub fn fallback_config_path(path: Option<PathBuf>) -> PathBuf {
    let p = match path {
        Some(p) => p.join("clink"),
        None => std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from(".")),
    };

    p.join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_default_config() {
        let cfg = ClinkConfig::default();
        let warnings = cfg.validate();
        assert!(
            warnings.is_empty(),
            "default config should have no warnings"
        );
    }

    #[test]
    fn test_validate_zero_sleep_duration() {
        let mut cfg = ClinkConfig::default();
        cfg.sleep_duration = 0;
        let warnings = cfg.validate();
        assert!(warnings.iter().any(|w| w.contains("sleep_duration")));
    }

    #[test]
    fn test_validate_empty_params() {
        let mut cfg = ClinkConfig::default();
        cfg.params.clear();
        let warnings = cfg.validate();
        assert!(warnings.iter().any(|w| w.contains("params")));
    }

    #[test]
    fn test_default_params_include_common_trackers() {
        let cfg = ClinkConfig::default();
        for param in [
            "fbclid",
            "gclid",
            "gclsrc",
            "dclid",
            "zanpid",
            "msclkid",
            "twclid",
            "ttclid",
            "igshid",
            "igsh",
            "li_fat_id",
            "mc_cid",
            "mc_eid",
            "_ga",
            "_gl",
            "yclid",
            "_hsenc",
            "_hsmi",
            "mkt_tok",
            "__s",
            "_openstat",
            "vero_id",
            "spm",
        ] {
            assert!(cfg.params.contains(param), "missing default param: {param}");
        }
    }

    #[test]
    fn test_default_params_utm_creative_format_lowercase() {
        let cfg = ClinkConfig::default();
        assert!(
            cfg.params.contains("utm_creative_format"),
            "utm_creative_format should be lowercase"
        );
        assert!(
            !cfg.params.contains("utm_Creative_format"),
            "utm_Creative_format (capital C) should not exist"
        );
    }

    #[test]
    fn test_default_params_amazon_com() {
        let cfg = ClinkConfig::default();
        // Amazon tracking params should cover .com, not just .de
        assert!(
            cfg.params.contains("amazon.com``sp_csd"),
            "missing amazon.com tracking params"
        );
    }

    #[test]
    fn test_default_params_youtube_music() {
        let cfg = ClinkConfig::default();
        assert!(
            cfg.params.contains("music.youtube.com``si"),
            "missing music.youtube.com si param"
        );
    }

    #[test]
    fn test_default_params_count_unchanged() {
        let params = get_default_params();
        // 34 global/YouTube params + 210 Amazon domain-specific params = 244
        assert_eq!(
            params.len(),
            244,
            "total default param count must not change"
        );
    }

    #[test]
    fn test_amazon_pattern_expands_to_all_domain_param_combinations() {
        use crate::expand_string::expand_string;

        let pattern = "amazon.(com|de|co.uk|co.jp|fr|it|es|ca|com.au|com.br|com.mx|nl|pl|se|sg|in|com.be|com.tr|eg|sa|ae)``(sp_csd|pd_rd_w|pd_rd_wg|pd_rd_i|pd_rd_r|pf_rd_r|pf_rd_p|t|psc|content-id)";
        let expanded: HashSet<String> = expand_string(pattern).into_iter().collect();

        let domains = [
            "amazon.com",
            "amazon.de",
            "amazon.co.uk",
            "amazon.co.jp",
            "amazon.fr",
            "amazon.it",
            "amazon.es",
            "amazon.ca",
            "amazon.com.au",
            "amazon.com.br",
            "amazon.com.mx",
            "amazon.nl",
            "amazon.pl",
            "amazon.se",
            "amazon.sg",
            "amazon.in",
            "amazon.com.be",
            "amazon.com.tr",
            "amazon.eg",
            "amazon.sa",
            "amazon.ae",
        ];
        let params = [
            "sp_csd",
            "pd_rd_w",
            "pd_rd_wg",
            "pd_rd_i",
            "pd_rd_r",
            "pf_rd_r",
            "pf_rd_p",
            "t",
            "psc",
            "content-id",
        ];
        let mut expected: HashSet<String> = HashSet::new();
        for domain in &domains {
            for param in &params {
                expected.insert(format!("{domain}``{param}"));
            }
        }

        assert_eq!(expanded.len(), 210, "should produce 210 combinations");
        assert_eq!(
            expanded, expected,
            "pattern must produce same entries as nested loop"
        );
    }

    #[test]
    fn test_load_config_expands_param_patterns() {
        use std::io::Write;

        let tmp = std::env::temp_dir().join("clink_test_pattern_config.toml");
        let cfg = ClinkConfig {
            params: HashSet::from(["foo.(bar|baz)``(a|b)".into()]),
            ..ClinkConfig::default()
        };
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        {
            let mut f = std::fs::File::create(&tmp).unwrap();
            f.write_all(toml_str.as_bytes()).unwrap();
        }

        let loaded = load_config(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);

        assert!(loaded.params.contains("foo.bar``a"), "missing foo.bar``a");
        assert!(loaded.params.contains("foo.bar``b"), "missing foo.bar``b");
        assert!(loaded.params.contains("foo.baz``a"), "missing foo.baz``a");
        assert!(loaded.params.contains("foo.baz``b"), "missing foo.baz``b");
        assert!(
            !loaded.params.contains("foo.(bar|baz)``(a|b)"),
            "raw pattern should not remain"
        );
        assert_eq!(
            loaded.params.len(),
            4,
            "should have exactly 4 expanded entries"
        );
    }

    #[test]
    fn test_load_config_returns_result() {
        let tmp = std::env::temp_dir().join("clink_test_bad_config.toml");
        std::fs::write(&tmp, "this is not valid [[[ toml").unwrap();
        let result = load_config(&tmp);
        assert!(result.is_err());
        let _ = std::fs::remove_file(&tmp);
    }
}
