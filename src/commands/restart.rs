use std::path::Path;
use std::process::{Command, Stdio};

use crate::runtime;

fn build_run_command(config_path: &Path, verbose: bool) -> Command {
    let exe = std::env::current_exe().expect("Failed to determine current executable path");
    let mut cmd = Command::new(exe);
    cmd.arg("--config").arg(config_path).arg("run");
    if verbose {
        cmd.arg("--verbose");
    }
    cmd
}

pub fn execute(config_path: &Path, verbose: bool) -> Result<(), String> {
    match runtime::read_pid() {
        None => {
            println!("clink is not running. Starting it.");
        }
        Some(pid) if !runtime::is_running(pid) => {
            runtime::remove_pid_file();
            println!("clink is not running (stale PID file for PID {pid}). Starting it.");
        }
        Some(pid) => {
            stop(pid)?;
        }
    }

    let child = build_run_command(config_path, verbose)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start clink: {e}"))?;

    println!("clink started (PID {}).", child.id());
    Ok(())
}

#[cfg(unix)]
fn stop(pid: u32) -> Result<(), String> {
    crate::signal::send_signal(pid, nix::sys::signal::Signal::SIGTERM)?;
    println!("Sent stop signal to clink (PID {pid}).");

    for _ in 0..50 {
        if !runtime::is_running(pid) {
            println!("clink stopped. Restarting.");
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Err(format!(
        "clink (PID {pid}) did not stop within 5 seconds. You may need to kill it manually: kill -9 {pid}"
    ))
}

#[cfg(not(unix))]
fn stop(_pid: u32) -> Result<(), String> {
    Err("Restart is not supported on this platform.".to_string())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn build_run_command_includes_config_path() {
        let cmd = super::build_run_command(Path::new("/tmp/config.toml"), false);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(args.contains(&std::ffi::OsStr::new("--config")));
        assert!(args.contains(&std::ffi::OsStr::new("/tmp/config.toml")));
        assert!(args.contains(&std::ffi::OsStr::new("run")));
        assert!(!args.contains(&std::ffi::OsStr::new("--verbose")));
    }

    #[test]
    fn build_run_command_includes_verbose_when_set() {
        let cmd = super::build_run_command(Path::new("/tmp/config.toml"), true);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(args.contains(&std::ffi::OsStr::new("--verbose")));
    }
}
