use crate::config::{ClinkConfig, get_default_raw_params, load_config};
use crate::expand_string::expand_string;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::io::{self, BufRead};
use std::path::Path;
use std::rc::Rc;

fn diff_params(user_params: &HashSet<String>, out: &mut String) -> bool {
    let raw_defaults = get_default_raw_params();
    let mut all_expanded_defaults: HashSet<String> = HashSet::new();
    let mut missing_raw: Vec<&str> = Vec::new();

    for &raw in raw_defaults {
        let expanded: Vec<String> = expand_string(raw);
        let any_missing = expanded.iter().any(|e| !user_params.contains(e));
        for e in &expanded {
            all_expanded_defaults.insert(e.clone());
        }
        if any_missing {
            missing_raw.push(raw);
        }
    }

    let extra_params: Vec<&String> = user_params
        .iter()
        .filter(|p| !all_expanded_defaults.contains(*p))
        .collect();

    let mut has_diff = false;

    if !missing_raw.is_empty() {
        has_diff = true;
        writeln!(
            out,
            "\nParams missing from your config ({}):",
            missing_raw.len()
        )
        .unwrap();
        missing_raw.sort_unstable();
        for p in &missing_raw {
            writeln!(out, "  + {p}").unwrap();
        }
    }

    if !extra_params.is_empty() {
        has_diff = true;
        writeln!(
            out,
            "\nExtra params in your config (not in defaults) ({}):",
            extra_params.len()
        )
        .unwrap();
        let mut sorted = extra_params;
        sorted.sort();
        for p in &sorted {
            writeln!(out, "  - {p}").unwrap();
        }
    }

    has_diff
}

fn diff_exits(user_exit: &[Vec<Rc<str>>], out: &mut String) -> bool {
    let default_cfg = ClinkConfig::default();

    let default_exit_strs: Vec<String> = default_cfg
        .exit
        .iter()
        .map(|v| v.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(" → "))
        .collect();
    let user_exit_strs: Vec<String> = user_exit
        .iter()
        .map(|v| v.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(" → "))
        .collect();

    let missing_exits: Vec<&String> = default_exit_strs
        .iter()
        .filter(|e| !user_exit_strs.contains(e))
        .collect();
    let extra_exits: Vec<&String> = user_exit_strs
        .iter()
        .filter(|e| !default_exit_strs.contains(e))
        .collect();

    let mut has_diff = false;

    if !missing_exits.is_empty() {
        has_diff = true;
        writeln!(out, "\nExit rules missing from your config:").unwrap();
        for e in &missing_exits {
            writeln!(out, "  + {e}").unwrap();
        }
    }
    if !extra_exits.is_empty() {
        has_diff = true;
        writeln!(out, "\nExtra exit rules in your config (not in defaults):").unwrap();
        for e in &extra_exits {
            writeln!(out, "  - {e}").unwrap();
        }
    }

    has_diff
}

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

    if !config_path.is_file() {
        writeln!(
            out,
            "\nConfig file does not exist. Run `clink init` to create one."
        )
        .unwrap();
        return Ok(out);
    }

    let user_cfg = load_config(config_path)?;
    let default_cfg = ClinkConfig::default();
    let mut has_diff = false;

    if user_cfg.mode != default_cfg.mode {
        has_diff = true;
        writeln!(
            out,
            "\nMode: {} (default: {})",
            user_cfg.mode, default_cfg.mode
        )
        .unwrap();
    }
    if user_cfg.replace_to != default_cfg.replace_to {
        has_diff = true;
        writeln!(
            out,
            "Replace to: {} (default: {})",
            user_cfg.replace_to, default_cfg.replace_to
        )
        .unwrap();
    }
    if user_cfg.sleep_duration != default_cfg.sleep_duration {
        has_diff = true;
        writeln!(
            out,
            "Sleep duration: {} (default: {})",
            user_cfg.sleep_duration, default_cfg.sleep_duration
        )
        .unwrap();
    }

    has_diff |= diff_params(&user_cfg.params, &mut out);
    has_diff |= diff_exits(&user_cfg.exit, &mut out);

    if !has_diff {
        writeln!(out, "\nConfig matches defaults.").unwrap();
    }

    Ok(out)
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
        let default_cfg = ClinkConfig::default();
        confy::store_path(config_path, &default_cfg)
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
    use std::rc::Rc;

    #[test]
    fn test_execute_prints_path() {
        let tmp = std::env::temp_dir().join("clink_test_config_path.toml");
        let cfg = ClinkConfig::default();
        confy::store_path(&tmp, &cfg).unwrap();

        let result = execute(&tmp, false, false);
        assert!(result.is_ok());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_diff_identical_config() {
        let tmp = std::env::temp_dir().join("clink_test_config_diff_same.toml");
        let cfg = ClinkConfig::default();
        confy::store_path(&tmp, &cfg).unwrap();

        let output = build_diff(&tmp).unwrap();
        assert!(output.contains("Config matches defaults."));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_diff_missing_config() {
        let tmp = std::env::temp_dir().join("clink_test_config_diff_missing.toml");
        let _ = std::fs::remove_file(&tmp);

        let output = build_diff(&tmp).unwrap();
        assert!(output.contains("does not exist"));
    }

    #[test]
    fn test_diff_detects_missing_params() {
        let tmp = std::env::temp_dir().join("clink_test_config_diff_params.toml");
        let mut cfg = ClinkConfig::default();
        cfg.params = HashSet::from(["fbclid".into()]);
        confy::store_path(&tmp, &cfg).unwrap();

        let output = build_diff(&tmp).unwrap();
        assert!(output.contains("Params missing"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_diff_detects_extra_params() {
        let tmp = std::env::temp_dir().join("clink_test_config_diff_extra.toml");
        let mut cfg = ClinkConfig::default();
        cfg.params.insert("custom_tracker".into());
        confy::store_path(&tmp, &cfg).unwrap();

        let output = build_diff(&tmp).unwrap();
        assert!(output.contains("Extra params"));
        assert!(output.contains("custom_tracker"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_diff_params_no_differences() {
        let cfg = ClinkConfig::default();
        let mut out = String::new();
        let has_diff = diff_params(&cfg.params, &mut out);
        assert!(!has_diff);
        assert!(out.is_empty());
    }

    #[test]
    fn test_diff_params_missing_params() {
        let params = HashSet::from(["fbclid".into()]);
        let mut out = String::new();
        let has_diff = diff_params(&params, &mut out);
        assert!(has_diff);
        assert!(out.contains("missing"));
    }

    #[test]
    fn test_diff_params_extra_params() {
        let mut params = ClinkConfig::default().params;
        params.insert("custom_tracker".into());
        let mut out = String::new();
        let has_diff = diff_params(&params, &mut out);
        assert!(has_diff);
        assert!(out.contains("Extra params"));
        assert!(out.contains("custom_tracker"));
    }

    #[test]
    fn test_diff_params_both_missing_and_extra() {
        let params = HashSet::from(["fbclid".into(), "my_param".into()]);
        let mut out = String::new();
        let has_diff = diff_params(&params, &mut out);
        assert!(has_diff);
        assert!(out.contains("missing"));
        assert!(out.contains("Extra params"));
        assert!(out.contains("my_param"));
    }

    #[test]
    fn test_diff_exits_no_differences() {
        let cfg = ClinkConfig::default();
        let mut out = String::new();
        let has_diff = diff_exits(&cfg.exit, &mut out);
        assert!(!has_diff);
        assert!(out.is_empty());
    }

    #[test]
    fn test_diff_exits_missing_rules() {
        let exit: Vec<Vec<Rc<str>>> = vec![];
        let mut out = String::new();
        let has_diff = diff_exits(&exit, &mut out);
        assert!(has_diff);
        assert!(out.contains("missing"));
    }

    #[test]
    fn test_diff_exits_extra_rules() {
        let mut exit = ClinkConfig::default().exit;
        exit.push(vec!["example.com".into(), "url".into()]);
        let mut out = String::new();
        let has_diff = diff_exits(&exit, &mut out);
        assert!(has_diff);
        assert!(out.contains("Extra exit rules"));
    }

    #[test]
    fn test_diff_shows_compact_amazon_pattern() {
        let tmp = std::env::temp_dir().join("clink_test_config_diff_amazon.toml");
        // Config with only fbclid — all Amazon params are missing
        let mut cfg = ClinkConfig::default();
        cfg.params = HashSet::from(["fbclid".into()]);
        confy::store_path(&tmp, &cfg).unwrap();

        let output = build_diff(&tmp).unwrap();

        // Should show the compact pattern, not individual expanded entries
        assert!(
            output.contains("amazon.(com|de|co.uk|"),
            "diff should show the compact Amazon pattern, got:\n{output}"
        );
        assert!(
            !output.contains("amazon.com``sp_csd"),
            "diff should NOT show individual expanded Amazon params, got:\n{output}"
        );

        let _ = std::fs::remove_file(&tmp);
    }
}
