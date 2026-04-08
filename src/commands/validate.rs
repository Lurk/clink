use crate::config::load_config;
use std::path::PathBuf;

pub fn execute(config_path: PathBuf) -> Result<(), String> {
    if !config_path.is_file() {
        return Err(format!(
            "Config file not found at {config_path:?}. Run `clink init` to create one."
        ));
    }

    let cfg = load_config(&config_path)?;
    let warnings = cfg.validate();

    println!("Config at {config_path:?}:");
    println!("  Mode: {}", cfg.mode);
    println!("  Sleep duration: {}ms", cfg.sleep_duration);
    println!("  Tracked params: {}", cfg.params.len());
    println!("  Exit patterns: {}", cfg.exit.len());

    if warnings.is_empty() {
        println!("\nConfig is valid.");
    } else {
        println!("\nWarnings:");
        for w in &warnings {
            println!("  - {w}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ClinkConfig;

    #[test]
    fn test_validate_good_config() {
        let tmp = std::env::temp_dir().join("clink_test_validate_good.toml");
        let cfg = ClinkConfig::default();
        confy::store_path(&tmp, &cfg).unwrap();

        let result = execute(tmp.clone());
        assert!(result.is_ok(), "validate should succeed: {:?}", result);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_validate_bad_config() {
        let tmp = std::env::temp_dir().join("clink_test_validate_bad.toml");
        std::fs::write(&tmp, "this is not valid toml for clink config [[[").unwrap();

        let result = execute(tmp.clone());
        assert!(result.is_err(), "validate should fail for bad TOML");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_validate_missing_config() {
        let tmp = std::env::temp_dir().join("clink_test_validate_missing.toml");
        let _ = std::fs::remove_file(&tmp);

        let result = execute(tmp);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}
