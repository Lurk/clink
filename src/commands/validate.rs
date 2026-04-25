use crate::config::load_config;
use crate::provider::check_provider;
use crate::remote::resolve_patterns;
use crate::runtime;
use std::path::Path;

pub fn execute(config_path: &Path) -> Result<(), String> {
    if !config_path.is_file() {
        return Err(format!(
            "Config file not found at {}. Run `clink init` to create one.",
            config_path.display()
        ));
    }

    let mut cfg = load_config(config_path)?;
    let mut warnings = cfg.validate();

    warnings.extend(resolve_patterns(&mut cfg, &runtime::data_dir()));
    for (name, p) in &cfg.providers {
        warnings.extend(check_provider(name, p));
    }

    let rule_count: usize = cfg.providers.values().map(|p| p.rules.len()).sum();
    let redirect_count: usize = cfg.providers.values().map(|p| p.redirections.len()).sum();

    println!("Config at {}:", config_path.display());
    println!("  Mode: {}", cfg.mode);
    println!("  Sleep duration: {}ms", cfg.sleep_duration);
    println!("  Providers: {}", cfg.providers.len());
    println!("  Total rules: {rule_count}");
    println!("  Total redirections: {redirect_count}");

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
        std::fs::write(&tmp, toml::to_string_pretty(&cfg).unwrap()).unwrap();

        let result = execute(&tmp);
        assert!(result.is_ok(), "validate should succeed: {:?}", result);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_validate_bad_config() {
        let tmp = std::env::temp_dir().join("clink_test_validate_bad.toml");
        std::fs::write(&tmp, "this is not valid toml for clink config [[[").unwrap();

        let result = execute(&tmp);
        assert!(result.is_err(), "validate should fail for bad TOML");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_validate_missing_config() {
        let tmp = std::env::temp_dir().join("clink_test_validate_missing.toml");
        let _ = std::fs::remove_file(&tmp);

        let result = execute(&tmp);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}
