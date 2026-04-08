use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    rc::Rc,
};

use serde::{Deserialize, Serialize};

use crate::mode::Mode;

/// Add query param, that must be replaced within any domain.
/// To specify domain specific params use format
/// `"{domain}``{param}"`.
const AMAZON_DOMAINS: &[&str] = &[
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

const AMAZON_PARAMS: &[&str] = &[
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

/// Add query param, that must be replaced within any domain.
/// To specify domain specific params use format
/// `"{domain}``{param}"`.
fn get_default_params() -> HashSet<String> {
    let mut params = HashSet::from([
        // Google
        "dclid".into(),
        "gclid".into(),
        "gclsrc".into(),
        "_ga".into(),
        "_gl".into(),
        // Meta (Facebook/Instagram)
        "fbclid".into(),
        "igshid".into(),
        // Microsoft/Bing
        "msclkid".into(),
        // Twitter/X
        "twclid".into(),
        // TikTok
        "ttclid".into(),
        // LinkedIn
        "li_fat_id".into(),
        // Yandex
        "yclid".into(),
        // UTM family
        "utm_id".into(),
        "utm_source".into(),
        "utm_source_platform".into(),
        "utm_creative_format".into(),
        "utm_medium".into(),
        "utm_term".into(),
        "utm_campaign".into(),
        "utm_content".into(),
        // Awin (formerly Zanox)
        "zanpid".into(),
        // Mailchimp
        "mc_cid".into(),
        "mc_eid".into(),
        // HubSpot
        "_hsenc".into(),
        "_hsmi".into(),
        // Marketo
        "mkt_tok".into(),
        // Drip
        "__s".into(),
        // Openstat
        "_openstat".into(),
        // Vero
        "vero_id".into(),
        // Alibaba/AliExpress
        "spm".into(),
        // YouTube
        "youtube.com``si".into(),
        "youtu.be``si".into(),
        "music.youtube.com``si".into(),
    ]);

    // Amazon tracking params across all domains
    for domain in AMAZON_DOMAINS {
        for param in AMAZON_PARAMS {
            params.insert(format!("{domain}``{param}"));
        }
    }

    params
}

fn get_default_exit() -> Vec<Vec<Rc<str>>> {
    vec![
    vec!["vk.com/away.php".into(), "to".into()],
    vec!["exit.sc/".into(), "url".into()],
    vec!["facebook.com/(l|confirmemail|login).php".into(), "u".into(), "next".into()],
    vec!["(www.|)(encrypted.|)google.(at|be|ca|ch|co.(bw|il|in|jp|nz|uk|za)|com(|.(ar|au|br|eg|mx|sg|tr|tw))|cl|de|dk|es|fr|it|nl|pl|pt|ru|se)/url".into(), "url".into()],
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
    confy::load_path(config_path).map_err(|e| {
        format!(
            "Config error at {path}: {e}\n\n\
             Looks like you have a bad config or config for an old version.\n\
             Config should look like this:\n\n{}",
            toml::to_string_pretty(&ClinkConfig::default()).unwrap()
        )
    })
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
    fn test_load_config_returns_result() {
        let tmp = std::env::temp_dir().join("clink_test_bad_config.toml");
        std::fs::write(&tmp, "this is not valid [[[ toml").unwrap();
        let result = load_config(&tmp);
        assert!(result.is_err());
        let _ = std::fs::remove_file(&tmp);
    }
}
