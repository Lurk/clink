use crate::config::DEFAULT_CONFIG_TEMPLATE;
use crate::service;
use std::path::Path;

pub fn execute(config_path: &Path) -> Result<(), String> {
    let binary_path =
        std::env::current_exe().map_err(|e| format!("Failed to determine binary path: {e}"))?;

    if !config_path.is_file() {
        println!(
            "Config not found at {}, creating default...",
            config_path.display()
        );
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }
        std::fs::write(config_path, DEFAULT_CONFIG_TEMPLATE)
            .map_err(|e| format!("Failed to write default config: {e}"))?;
        println!("Default config created at {}", config_path.display());
    }

    service::install(&binary_path, config_path)
}
