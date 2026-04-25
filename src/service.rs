use std::path::Path;

#[cfg(target_os = "macos")]
mod platform {
    use crate::runtime;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn plist_path() -> PathBuf {
        dirs_next::home_dir()
            .expect("Could not determine home directory")
            .join("Library/LaunchAgents/com.clink.agent.plist")
    }

    pub fn generate_plist(binary_path: &Path, config_path: &Path) -> String {
        let log_path = runtime::log_file_path();
        let config_arg = config_path.display();
        let binary = binary_path.display();
        let log = log_path.display();

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.clink.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>{binary}</string>
        <string>--config</string>
        <string>{config_arg}</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{log}</string>
    <key>StandardErrorPath</key>
    <string>{log}</string>
</dict>
</plist>
"#
        )
    }

    pub fn install(binary_path: &Path, config_path: &Path) -> Result<(), String> {
        let plist = plist_path();
        if let Some(parent) = plist.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create LaunchAgents directory: {e}"))?;
        }

        let content = generate_plist(binary_path, config_path);
        fs::write(&plist, &content)
            .map_err(|e| format!("Failed to write plist at {}: {e}", plist.display()))?;

        let output = Command::new("launchctl")
            .args(["load", "-w"])
            .arg(&plist)
            .output()
            .map_err(|e| format!("Failed to run launchctl: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("launchctl load failed: {stderr}"));
        }

        println!("Installed launchd service at {}", plist.display());
        println!("clink will start automatically on login.");
        Ok(())
    }

    pub fn uninstall() -> Result<(), String> {
        let plist = plist_path();
        if !plist.exists() {
            return Err("clink is not installed as a launchd service.".to_string());
        }

        let output = Command::new("launchctl")
            .args(["unload"])
            .arg(&plist)
            .output()
            .map_err(|e| format!("Failed to run launchctl: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("launchctl unload failed: {stderr}"));
        }

        fs::remove_file(&plist).map_err(|e| format!("Failed to remove plist: {e}"))?;

        println!("Uninstalled launchd service. Removed {}", plist.display());
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn unit_path() -> PathBuf {
        dirs_next::home_dir()
            .expect("Could not determine home directory")
            .join(".config/systemd/user/clink.service")
    }

    pub fn generate_unit(binary_path: &Path, config_path: &Path) -> String {
        let binary = binary_path.display();
        let config_arg = config_path.display();

        format!(
            r"[Unit]
Description=Clean links copied to clipboard
Documentation=https://github.com/Lurk/clink?tab=readme-ov-file#readme

[Service]
ExecStart={binary} --config {config_arg} run
ExecReload=/bin/kill -HUP $MAINPID
# Sandboxing and other hardening
NoNewPrivileges=yes
ProtectProc=noaccess
SystemCallFilter=@system-service
SystemCallArchitectures=native
ProtectSystem=strict
PrivateTmp=yes
PrivateDevices=yes
ProtectHostname=yes
ProtectClock=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectKernelLogs=yes
ProtectControlGroups=yes
RestrictAddressFamilies=AF_UNIX
RestrictFileSystems=~@privileged-api
LockPersonality=yes
MemoryDenyWriteExecute=yes
RestrictRealtime=yes

[Install]
WantedBy=default.target
"
        )
    }

    pub fn install(binary_path: &Path, config_path: &Path) -> Result<(), String> {
        let unit = unit_path();
        if let Some(parent) = unit.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create systemd user directory: {e}"))?;
        }

        let content = generate_unit(binary_path, config_path);
        fs::write(&unit, &content)
            .map_err(|e| format!("Failed to write unit file at {}: {e}", unit.display()))?;

        let output = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output()
            .map_err(|e| format!("Failed to run systemctl: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("systemctl daemon-reload failed: {stderr}"));
        }

        let output = Command::new("systemctl")
            .args(["--user", "enable", "--now", "clink"])
            .output()
            .map_err(|e| format!("Failed to run systemctl: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("systemctl enable failed: {stderr}"));
        }

        println!("Installed systemd user service at {}", unit.display());
        println!("clink will start automatically on login.");
        Ok(())
    }

    pub fn uninstall() -> Result<(), String> {
        let unit = unit_path();
        if !unit.exists() {
            return Err("clink is not installed as a systemd service.".to_string());
        }

        let output = Command::new("systemctl")
            .args(["--user", "disable", "--now", "clink"])
            .output()
            .map_err(|e| format!("Failed to run systemctl: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("systemctl disable failed: {stderr}"));
        }

        fs::remove_file(&unit).map_err(|e| format!("Failed to remove unit file: {e}"))?;

        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output();

        println!("Uninstalled systemd service. Removed {}", unit.display());
        Ok(())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod platform {
    use std::path::Path;

    pub fn install(_binary_path: &Path, _config_path: &Path) -> Result<(), String> {
        Err("Service install is not supported on this platform.".to_string())
    }

    pub fn uninstall() -> Result<(), String> {
        Err("Service uninstall is not supported on this platform.".to_string())
    }
}

pub fn install(binary_path: &Path, config_path: &Path) -> Result<(), String> {
    platform::install(binary_path, config_path)
}

pub fn uninstall() -> Result<(), String> {
    platform::uninstall()
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "macos")]
    #[test]
    fn test_generate_launchd_plist() {
        use super::platform::generate_plist;
        use std::path::Path;

        let plist = generate_plist(
            Path::new("/usr/local/bin/clink"),
            Path::new("/Users/test/.config/clink/config.toml"),
        );

        assert!(plist.contains("<string>com.clink.agent</string>"));
        assert!(plist.contains("<string>/usr/local/bin/clink</string>"));
        assert!(plist.contains("<string>--config</string>"));
        assert!(plist.contains("/Users/test/.config/clink/config.toml"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<key>KeepAlive</key>"));
        assert!(plist.contains("<key>StandardOutPath</key>"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_generate_systemd_unit() {
        use super::platform::generate_unit;
        use std::path::Path;

        let unit = generate_unit(
            Path::new("/usr/bin/clink"),
            Path::new("/home/test/.config/clink/config.toml"),
        );

        assert!(unit.contains(
            "ExecStart=/usr/bin/clink --config /home/test/.config/clink/config.toml run"
        ));
        assert!(unit.contains("ExecReload=/bin/kill -HUP $MAINPID"));
        assert!(unit.contains("WantedBy=default.target"));
        assert!(unit.contains("NoNewPrivileges=yes"));
    }
}
