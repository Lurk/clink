use crate::config::{ClinkConfig, load_config};
use crate::remote::resolve_patterns;
use crate::runtime;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::io::{self, BufRead};
use std::path::Path;

pub fn execute(config_path: &Path, diff: bool, reset: bool) -> Result<(), String> {
    println!("Config path: {}", config_path.display());

    if diff {
        let output = build_diff(config_path)?;
        print!("{output}");
    }

    if reset {
        do_reset(config_path)?;
    }

    Ok(())
}

fn build_diff(config_path: &Path) -> Result<String, String> {
    let mut out = String::new();

    let pid = runtime::read_pid();
    let is_running = pid.is_some_and(runtime::is_running);

    if !is_running {
        writeln!(
            out,
            "\nclink is not running. Diff shows loaded vs current resolved config."
        )
        .unwrap();
        writeln!(out, "Start clink first, or the loaded config may be stale.").unwrap();
    }

    let loaded_path = runtime::loaded_config_path();
    if !loaded_path.is_file() {
        writeln!(out, "\nNo loaded config state file found.").unwrap();
        writeln!(
            out,
            "Start clink to generate it, then run `clink config --diff` again."
        )
        .unwrap();
        return Ok(out);
    }

    let loaded: ClinkConfig = {
        let content = std::fs::read_to_string(&loaded_path)
            .map_err(|e| format!("Failed to read loaded config: {e}"))?;
        toml::from_str(&content).map_err(|e| format!("Failed to parse loaded config: {e}"))?
    };

    if !config_path.is_file() {
        writeln!(
            out,
            "\nConfig file does not exist at {}.",
            config_path.display()
        )
        .unwrap();
        return Ok(out);
    }

    let mut current = load_config(config_path)?;
    resolve_patterns(&mut current, &runtime::data_dir());

    let has_diff = diff_configs(&loaded, &current, &mut out);

    if has_diff {
        writeln!(out, "\nRun `clink reload` to apply these changes.").unwrap();
    } else {
        writeln!(
            out,
            "\nLoaded config matches current config. No changes pending."
        )
        .unwrap();
    }

    Ok(out)
}

fn collect_all<F>(config: &ClinkConfig, field: F) -> HashSet<String>
where
    F: Fn(&crate::provider::ProviderConfig) -> &[String],
{
    config
        .providers
        .iter()
        .flat_map(|(name, p)| field(p).iter().map(move |r| format!("{name}:{r}")))
        .collect()
}

fn diff_configs(loaded: &ClinkConfig, current: &ClinkConfig, out: &mut String) -> bool {
    let mut has_diff = false;

    if loaded.mode != current.mode {
        has_diff = true;
        writeln!(out, "\nMode: {} -> {}", loaded.mode, current.mode).unwrap();
    }
    if loaded.replace_to != current.replace_to {
        has_diff = true;
        writeln!(
            out,
            "Replace to: {} -> {}",
            loaded.replace_to, current.replace_to
        )
        .unwrap();
    }
    if loaded.sleep_duration != current.sleep_duration {
        has_diff = true;
        writeln!(
            out,
            "Sleep duration: {} -> {}",
            loaded.sleep_duration, current.sleep_duration
        )
        .unwrap();
    }

    let loaded_rules = collect_all(loaded, |p| &p.rules);
    let current_rules = collect_all(current, |p| &p.rules);

    let added_rules: Vec<&String> = current_rules.difference(&loaded_rules).collect();
    let removed_rules: Vec<&String> = loaded_rules.difference(&current_rules).collect();

    if !added_rules.is_empty() {
        has_diff = true;
        writeln!(out, "\nRules added ({}):", added_rules.len()).unwrap();
        let mut sorted = added_rules;
        sorted.sort();
        for r in &sorted {
            writeln!(out, "  + {r}").unwrap();
        }
    }
    if !removed_rules.is_empty() {
        has_diff = true;
        writeln!(out, "\nRules removed ({}):", removed_rules.len()).unwrap();
        let mut sorted = removed_rules;
        sorted.sort();
        for r in &sorted {
            writeln!(out, "  - {r}").unwrap();
        }
    }

    let loaded_redirections = collect_all(loaded, |p| &p.redirections);
    let current_redirections = collect_all(current, |p| &p.redirections);

    let added_redirections: Vec<&String> = current_redirections
        .difference(&loaded_redirections)
        .collect();
    let removed_redirections: Vec<&String> = loaded_redirections
        .difference(&current_redirections)
        .collect();

    if !added_redirections.is_empty() {
        has_diff = true;
        writeln!(out, "\nRedirections added:").unwrap();
        for r in &added_redirections {
            writeln!(out, "  + {r}").unwrap();
        }
    }
    if !removed_redirections.is_empty() {
        has_diff = true;
        writeln!(out, "\nRedirections removed:").unwrap();
        for r in &removed_redirections {
            writeln!(out, "  - {r}").unwrap();
        }
    }

    has_diff
}

fn do_reset(config_path: &Path) -> Result<(), String> {
    eprint!(
        "\nThis will overwrite {} with default config. Continue? [y/N] ",
        config_path.display()
    );

    let stdin = io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| format!("Failed to read input: {e}"))?;

    if line.trim().eq_ignore_ascii_case("y") {
        let template = include_str!("../default_config.toml");
        std::fs::write(config_path, template)
            .map_err(|e| format!("Failed to write config: {e}"))?;
        println!("Config reset to defaults.");
    } else {
        println!("Reset cancelled.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ClinkConfig;
    use std::collections::HashMap;

    #[test]
    fn test_execute_prints_path() {
        let tmp = std::env::temp_dir().join("clink_test_config_path_v2.toml");
        let cfg = ClinkConfig::default();
        confy::store_path(&tmp, &cfg).unwrap();

        let result = execute(&tmp, false, false);
        assert!(result.is_ok());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_diff_no_loaded_config() {
        let tmp = std::env::temp_dir().join("clink_test_config_diff_no_loaded.toml");
        let cfg = ClinkConfig::default();
        confy::store_path(&tmp, &cfg).unwrap();

        let output = build_diff(&tmp).unwrap();
        assert!(
            output.contains("No loaded config state file found") || output.contains("not running")
        );

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_diff_rules_detects_added() {
        let mut loaded_providers = HashMap::new();
        loaded_providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["fbclid".into()],
                ..Default::default()
            },
        );
        let loaded = ClinkConfig {
            providers: loaded_providers,
            ..ClinkConfig::default()
        };

        let mut current_providers = HashMap::new();
        current_providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["fbclid".into(), "gclid".into()],
                ..Default::default()
            },
        );
        let current = ClinkConfig {
            providers: current_providers,
            ..ClinkConfig::default()
        };

        let mut out = String::new();
        let changed = diff_configs(&loaded, &current, &mut out);
        assert!(changed);
        assert!(out.contains("gclid"));
    }

    #[test]
    fn test_diff_rules_detects_removed() {
        let mut loaded_providers = HashMap::new();
        loaded_providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["fbclid".into(), "gclid".into()],
                ..Default::default()
            },
        );
        let loaded = ClinkConfig {
            providers: loaded_providers,
            ..ClinkConfig::default()
        };

        let mut current_providers = HashMap::new();
        current_providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["fbclid".into()],
                ..Default::default()
            },
        );
        let current = ClinkConfig {
            providers: current_providers,
            ..ClinkConfig::default()
        };

        let mut out = String::new();
        let changed = diff_configs(&loaded, &current, &mut out);
        assert!(changed);
        assert!(out.contains("gclid"));
    }

    #[test]
    fn test_diff_no_changes() {
        let mut providers = HashMap::new();
        providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["fbclid".into()],
                ..Default::default()
            },
        );
        let loaded = ClinkConfig {
            providers: providers.clone(),
            ..ClinkConfig::default()
        };
        let current = ClinkConfig {
            providers,
            ..ClinkConfig::default()
        };
        let mut out = String::new();
        let changed = diff_configs(&loaded, &current, &mut out);
        assert!(!changed);
    }

    #[test]
    fn test_diff_mode_change() {
        let loaded = ClinkConfig::default();
        let mut current = ClinkConfig::default();
        current.mode = crate::mode::Mode::Replace;
        let mut out = String::new();
        let changed = diff_configs(&loaded, &current, &mut out);
        assert!(changed);
        assert!(out.contains("Mode:"));
    }

    #[test]
    fn test_diff_sleep_duration_change() {
        let loaded = ClinkConfig::default();
        let mut current = ClinkConfig::default();
        current.sleep_duration = 500;
        let mut out = String::new();
        let changed = diff_configs(&loaded, &current, &mut out);
        assert!(changed);
        assert!(out.contains("Sleep duration:"));
    }
}
