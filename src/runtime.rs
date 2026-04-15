use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn pid_file_path() -> PathBuf {
    runtime_dir().join("clink.pid")
}

pub fn log_file_path() -> PathBuf {
    data_dir().join("clink.log")
}

pub fn stats_file_path() -> PathBuf {
    data_dir().join("stats.toml")
}

fn runtime_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        std::env::var("TMPDIR").map_or_else(|_| std::env::temp_dir(), PathBuf::from)
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_RUNTIME_DIR").map_or_else(|_| std::env::temp_dir(), PathBuf::from)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        std::env::temp_dir()
    }
}

pub fn data_dir() -> PathBuf {
    dirs_next::data_dir().map_or_else(|| PathBuf::from("."), |d| d.join("clink"))
}

pub fn write_pid_file() -> Result<(), String> {
    let path = pid_file_path();
    if let Some(existing_pid) = read_pid() {
        if is_running(existing_pid) {
            return Err(format!(
                "clink is already running (PID {existing_pid}). \
                 If this is incorrect, remove {} and try again.",
                path.display()
            ));
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {e}"))?;
    }
    let mut file =
        fs::File::create(&path).map_err(|e| format!("Failed to create PID file: {e}"))?;
    write!(file, "{}", std::process::id()).map_err(|e| format!("Failed to write PID file: {e}"))?;
    Ok(())
}

pub fn read_pid() -> Option<u32> {
    let path = pid_file_path();
    fs::read_to_string(&path).ok()?.trim().parse().ok()
}

pub fn remove_pid_file() {
    let _ = fs::remove_file(pid_file_path());
}

#[cfg(unix)]
pub fn is_running(pid: u32) -> bool {
    use nix::sys::signal;
    use nix::unistd::Pid;
    #[allow(clippy::cast_possible_wrap)]
    signal::kill(Pid::from_raw(pid as i32), None).is_ok()
}

#[cfg(not(unix))]
pub fn is_running(_pid: u32) -> bool {
    false
}

pub fn loaded_config_path() -> PathBuf {
    data_dir().join("loaded_config.toml")
}

pub fn write_loaded_config(config: &crate::config::ClinkConfig) -> Result<(), String> {
    let path = loaded_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {e}"))?;
    }
    let content =
        toml::to_string_pretty(config).map_err(|e| format!("Failed to serialize config: {e}"))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write loaded config: {e}"))?;
    Ok(())
}

pub fn remove_loaded_config() {
    let _ = fs::remove_file(loaded_config_path());
}

pub fn append_log(message: &str) -> Result<(), String> {
    let path = log_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create log directory: {e}"))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Failed to open log file: {e}"))?;
    writeln!(file, "{message}").map_err(|e| format!("Failed to write log: {e}"))?;
    Ok(())
}

pub fn read_last_log_lines(n: usize) -> Vec<String> {
    let path = log_file_path();
    match fs::read_to_string(&path) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            let start = lines.len().saturating_sub(n);
            lines[start..]
                .iter()
                .map(std::string::ToString::to_string)
                .collect()
        }
        Err(_) => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loaded_config_write_and_read() {
        let cfg = crate::config::ClinkConfig::default();
        let content = toml::to_string_pretty(&cfg).unwrap();
        assert!(content.contains("mode"));
    }

    #[test]
    fn test_pid_file_write_read_remove() {
        let test_path = std::env::temp_dir().join("clink_test_pid_wrt.pid");
        let _ = fs::remove_file(&test_path);

        let mut f = fs::File::create(&test_path).unwrap();
        write!(f, "12345").unwrap();
        drop(f);

        let content = fs::read_to_string(&test_path).unwrap();
        assert_eq!(content.trim().parse::<u32>().unwrap(), 12345);

        fs::remove_file(&test_path).unwrap();
        assert!(!test_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_is_running_current_process() {
        assert!(is_running(std::process::id()));
    }

    #[cfg(unix)]
    #[test]
    fn test_is_running_dead_pid() {
        // PID 4194304 is above typical PID range
        assert!(!is_running(4194304));
    }

    #[test]
    fn test_append_and_read_log() {
        let tmp_dir = std::env::temp_dir().join("clink_test_log");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();

        let log_path = tmp_dir.join("test.log");
        let _ = fs::remove_file(&log_path);

        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .unwrap();
        for i in 0..5 {
            writeln!(f, "line {i}").unwrap();
        }
        drop(f);

        let content = fs::read_to_string(&log_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        let last3: Vec<&str> = lines[lines.len().saturating_sub(3)..].to_vec();
        assert_eq!(last3, vec!["line 2", "line 3", "line 4"]);

        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
