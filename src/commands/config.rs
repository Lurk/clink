use crate::config::{ClinkConfig, load_config, resolve_patterns};
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

    let added_params: Vec<&String> = current.params.difference(&loaded.params).collect();
    let removed_params: Vec<&String> = loaded.params.difference(&current.params).collect();

    if !added_params.is_empty() {
        has_diff = true;
        writeln!(out, "\nParams added ({}):", added_params.len()).unwrap();
        let mut sorted = added_params;
        sorted.sort();
        for p in &sorted {
            writeln!(out, "  + {p}").unwrap();
        }
    }
    if !removed_params.is_empty() {
        has_diff = true;
        writeln!(out, "\nParams removed ({}):", removed_params.len()).unwrap();
        let mut sorted = removed_params;
        sorted.sort();
        for p in &sorted {
            writeln!(out, "  - {p}").unwrap();
        }
    }

    let loaded_exit_strs: HashSet<String> = loaded
        .exit
        .iter()
        .map(|v| v.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(" -> "))
        .collect();
    let current_exit_strs: HashSet<String> = current
        .exit
        .iter()
        .map(|v| v.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(" -> "))
        .collect();

    let added_exits: Vec<&String> = current_exit_strs.difference(&loaded_exit_strs).collect();
    let removed_exits: Vec<&String> = loaded_exit_strs.difference(&current_exit_strs).collect();

    if !added_exits.is_empty() {
        has_diff = true;
        writeln!(out, "\nExit rules added:").unwrap();
        for e in &added_exits {
            writeln!(out, "  + {e}").unwrap();
        }
    }
    if !removed_exits.is_empty() {
        has_diff = true;
        writeln!(out, "\nExit rules removed:").unwrap();
        for e in &removed_exits {
            writeln!(out, "  - {e}").unwrap();
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
    use std::collections::HashSet;

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
    fn test_diff_params_detects_added() {
        let loaded = ClinkConfig {
            params: HashSet::from(["fbclid".into()]),
            ..ClinkConfig::default()
        };
        let current = ClinkConfig {
            params: HashSet::from(["fbclid".into(), "gclid".into()]),
            ..ClinkConfig::default()
        };
        let mut out = String::new();
        let changed = diff_configs(&loaded, &current, &mut out);
        assert!(changed);
        assert!(out.contains("gclid"));
    }

    #[test]
    fn test_diff_params_detects_removed() {
        let loaded = ClinkConfig {
            params: HashSet::from(["fbclid".into(), "gclid".into()]),
            ..ClinkConfig::default()
        };
        let current = ClinkConfig {
            params: HashSet::from(["fbclid".into()]),
            ..ClinkConfig::default()
        };
        let mut out = String::new();
        let changed = diff_configs(&loaded, &current, &mut out);
        assert!(changed);
        assert!(out.contains("gclid"));
    }

    #[test]
    fn test_diff_no_changes() {
        let loaded = ClinkConfig {
            params: HashSet::from(["fbclid".into()]),
            ..ClinkConfig::default()
        };
        let current = ClinkConfig {
            params: HashSet::from(["fbclid".into()]),
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
