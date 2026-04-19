use std::path::Path;

const DEFAULT_CONFIG_TEMPLATE: &str = include_str!("../default_config.toml");

pub fn execute(config_path: &Path) -> Result<(), String> {
    if config_path.is_file() {
        return Err(format!(
            "Config already exists at {}. Remove it first if you want to reinitialize.",
            config_path.display()
        ));
    }

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;
    }

    std::fs::write(config_path, DEFAULT_CONFIG_TEMPLATE)
        .map_err(|e| format!("Failed to write config: {e}"))?;

    println!("Config initialized at {}", config_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_creates_config() {
        let tmp = std::env::temp_dir().join("clink_test_init_config.toml");
        let _ = std::fs::remove_file(&tmp);

        let result = execute(&tmp);
        assert!(result.is_ok(), "init should succeed: {:?}", result);
        assert!(tmp.is_file(), "config file should exist");

        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("mode"));
        assert!(content.contains("sleep_duration"));
        assert!(
            content.contains("[providers."),
            "template should contain at least one provider"
        );
        assert!(
            !content.contains("[providers.global]"),
            "template should not bake in global provider rules — those come from the builtin snapshot"
        );
        assert!(
            content.contains("[providers.exitsc]"),
            "template should ship the exit.sc redirector default"
        );
        assert!(
            content.contains("[remote]"),
            "template should contain remote section"
        );

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_init_does_not_overwrite() {
        let tmp = std::env::temp_dir().join("clink_test_init_no_overwrite.toml");
        std::fs::write(&tmp, "existing content").unwrap();

        let result = execute(&tmp);
        assert!(result.is_err(), "init should fail when file exists");
        assert!(result.unwrap_err().contains("already exists"));

        let content = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(content, "existing content");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_template_is_valid_config() {
        let tmp = std::env::temp_dir().join("clink_test_init_template_valid.toml");
        let _ = std::fs::remove_file(&tmp);
        std::fs::write(&tmp, DEFAULT_CONFIG_TEMPLATE).unwrap();

        let result = crate::config::load_config(&tmp);
        assert!(
            result.is_ok(),
            "template should be a valid config: {:?}",
            result
        );

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_template_with_no_cache_has_effective_builtin_providers() {
        let dir = std::env::temp_dir().join("clink_test_init_builtin_effective");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let config_path = dir.join("config.toml");
        execute(&config_path).unwrap();

        let mut cfg = crate::config::load_config(&config_path).unwrap();
        // Templated config now ships clink-curated providers (exit.sc, amazon, ...).
        // The builtin fallback still supplies tracking rules like fbclid.
        assert!(
            cfg.providers.contains_key("exitsc"),
            "templated config should include the exit.sc redirector default"
        );
        crate::remote::resolve_patterns(&mut cfg, &dir);

        let has_fbclid = cfg
            .providers
            .values()
            .any(|p| p.rules.iter().any(|r| r.contains("fbclid")));
        assert!(
            has_fbclid,
            "fresh install with no cache must clean fbclid via builtin fallback"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
