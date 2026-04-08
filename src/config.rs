use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    rc::Rc,
};

use serde::{Deserialize, Serialize};

use crate::mode::Mode;

/// add query param, that must be replaced within any domain
/// to specify domain specific params use format
/// "{domain}``{param}"
fn get_default_params() -> HashSet<String> {
    HashSet::from([
        "dclid".into(),
        "fbclid".into(),
        "gclid".into(),
        "gclsrc".into(),
        "dclid".into(),
        "utm_id".into(),
        "utm_source_platform".into(),
        "utm_Creative_format".into(),
        "utm_medium".into(),
        "utm_source".into(),
        "utm_term".into(),
        "utm_campaign".into(),
        "utm_content".into(),
        "zanpid".into(),
        "youtube.com``si".into(),
        "youtu.be``si".into(),
        "youtu.be``si".into(),
        "amazon.de``sp_csd".into(),
        "amazon.de``pd_rd_w".into(),
        "amazon.de``pd_rd_wg".into(),
        "amazon.de``pd_rd_i".into(),
        "amazon.de``pd_rd_r".into(),
        "amazon.de``pf_rd_r".into(),
        "amazon.de``pf_rd_p".into(),
        "amazon.de``t".into(),
        "amazon.de``psc".into(),
        "amazon.de``content-id".into(),
    ])
}

fn get_default_exit() -> Vec<Vec<Rc<str>>> {
    vec![
    vec!["vk.com/away.php".into(), "to".into()],
    vec!["exit.sc/".into(), "url".into()],
    vec!["facebook.com/(l|confirmemail|login).php".into(), "u".into(), "next".into()],
    vec!["(www.|)(encrypted.|)google.(at|be|ca|ch|co.(bw|il|uk)|com(|.(ar|au|br|eg|tr|tw))|cl|de|dk|es|fr|nl|pl|se)/url".into(), "url".into()],
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
    confy::load_path(config_path).map_err(|e| {
        format!(
            "Config error at {config_path:?}: {e}\n\n\
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
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf(),
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
    fn test_load_config_returns_result() {
        let tmp = std::env::temp_dir().join("clink_test_bad_config.toml");
        std::fs::write(&tmp, "this is not valid [[[ toml").unwrap();
        let result = load_config(&tmp);
        assert!(result.is_err());
        let _ = std::fs::remove_file(&tmp);
    }
}
