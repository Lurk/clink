use crate::config::ClinkConfig;
use std::path::PathBuf;

pub fn execute(config_path: PathBuf) -> Result<(), String> {
    if config_path.is_file() {
        return Err(format!(
            "Config already exists at {config_path:?}. Remove it first if you want to reinitialize."
        ));
    }

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;
    }

    let cfg = ClinkConfig::default();
    confy::store_path(&config_path, &cfg).map_err(|e| format!("Failed to write config: {e}"))?;

    println!("Config initialized at {config_path:?}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_creates_config() {
        let tmp = std::env::temp_dir().join("clink_test_init_config.toml");
        let _ = std::fs::remove_file(&tmp);

        let result = execute(tmp.clone());
        assert!(result.is_ok(), "init should succeed: {:?}", result);
        assert!(tmp.is_file(), "config file should exist");

        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("mode"));
        assert!(content.contains("sleep_duration"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_init_does_not_overwrite() {
        let tmp = std::env::temp_dir().join("clink_test_init_no_overwrite.toml");
        std::fs::write(&tmp, "existing content").unwrap();

        let result = execute(tmp.clone());
        assert!(result.is_err(), "init should fail when file exists");
        assert!(result.unwrap_err().contains("already exists"));

        let content = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(content, "existing content");

        let _ = std::fs::remove_file(&tmp);
    }
}
