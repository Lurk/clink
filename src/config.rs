use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    rc::Rc,
};

use serde::{Deserialize, Serialize};

use crate::expand_string::expand_string;
use crate::mode::Mode;

#[derive(Serialize, Deserialize, Debug)]
pub struct RemotePatterns {
    pub params: HashSet<String>,
    pub exit: Vec<Vec<Rc<str>>>,
}

pub fn resolve_patterns(config: &mut ClinkConfig, data_dir: &Path) {
    let cache_path = data_dir.join("remote_patterns.toml");
    let Ok(content) = std::fs::read_to_string(&cache_path) else {
        return;
    };
    let Ok(remote) = toml::from_str::<RemotePatterns>(&content) else {
        return;
    };

    let local_params = std::mem::take(&mut config.params);
    let local_exit = std::mem::take(&mut config.exit);

    config.params = remote.params;
    config.exit = remote.exit;

    config.params.extend(local_params);
    config.exit.extend(local_exit);
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RemoteFormat {
    #[serde(rename = "clearurls")]
    ClearUrls,
    #[serde(rename = "clink")]
    Clink,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Remote {
    pub url: String,
    pub format: RemoteFormat,
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
    #[serde(default)]
    pub remote: Option<Remote>,
}

impl ClinkConfig {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            replace_to: "clink".into(),
            sleep_duration: 150,
            params: HashSet::new(),
            exit: Vec::new(),
            verbose: false,
            remote: None,
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
            warnings.iter().any(|w| w.contains("params")),
            "default config with no params should warn"
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

    #[test]
    fn test_remote_config_serde_roundtrip() {
        let cfg = ClinkConfig {
            remote: Some(Remote {
                url: "https://example.com/data.json".into(),
                format: RemoteFormat::ClearUrls,
            }),
            ..ClinkConfig::default()
        };
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        let loaded: ClinkConfig = toml::from_str(&toml_str).unwrap();
        let remote = loaded.remote.unwrap();
        assert_eq!(remote.url, "https://example.com/data.json");
        assert_eq!(remote.format, RemoteFormat::ClearUrls);
    }

    #[test]
    fn test_config_without_remote_section() {
        let cfg = ClinkConfig::default();
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        let loaded: ClinkConfig = toml::from_str(&toml_str).unwrap();
        assert!(loaded.remote.is_none());
    }

    #[test]
    fn test_remote_patterns_serde_roundtrip() {
        let patterns = RemotePatterns {
            params: HashSet::from(["fbclid".into(), "gclid".into()]),
            exit: vec![vec!["exit.sc/".into(), "url".into()]],
        };
        let toml_str = toml::to_string_pretty(&patterns).unwrap();
        let loaded: RemotePatterns = toml::from_str(&toml_str).unwrap();
        assert_eq!(loaded.params.len(), 2);
        assert!(loaded.params.contains("fbclid"));
        assert_eq!(loaded.exit.len(), 1);
    }

    #[test]
    fn test_resolve_merges_remote_and_local() {
        let dir = std::env::temp_dir().join("clink_test_resolve");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let remote = RemotePatterns {
            params: HashSet::from(["remote_param".into(), "shared".into()]),
            exit: vec![vec!["remote.com/".into(), "url".into()]],
        };
        let cache_path = dir.join("remote_patterns.toml");
        let content = toml::to_string(&remote).unwrap();
        std::fs::write(&cache_path, content).unwrap();

        let mut cfg = ClinkConfig {
            params: HashSet::from(["local_param".into(), "shared".into()]),
            exit: vec![vec!["local.com/".into(), "u".into()]],
            ..ClinkConfig::default()
        };

        resolve_patterns(&mut cfg, &dir);

        assert!(
            cfg.params.contains("remote_param"),
            "should have remote param"
        );
        assert!(
            cfg.params.contains("local_param"),
            "should have local param"
        );
        assert!(cfg.params.contains("shared"), "should have shared param");
        assert_eq!(cfg.params.len(), 3);
        assert_eq!(
            cfg.exit.len(),
            2,
            "should have both remote and local exit rules"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_no_cache_keeps_local() {
        let dir = std::env::temp_dir().join("clink_test_resolve_no_cache");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut cfg = ClinkConfig {
            params: HashSet::from(["local_param".into()]),
            exit: vec![vec!["local.com/".into(), "u".into()]],
            ..ClinkConfig::default()
        };

        resolve_patterns(&mut cfg, &dir);

        assert_eq!(cfg.params.len(), 1);
        assert!(cfg.params.contains("local_param"));
        assert_eq!(cfg.exit.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
