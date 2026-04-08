use crate::runtime;

pub fn execute() -> Result<(), String> {
    let pid = runtime::read_pid().ok_or("clink is not running (no PID file found).")?;

    if !runtime::is_running(pid) {
        runtime::remove_pid_file();
        return Err(format!(
            "clink is not running (stale PID file for PID {pid})."
        ));
    }

    #[cfg(unix)]
    {
        crate::signal::send_signal(pid, nix::sys::signal::Signal::SIGHUP)?;
        println!("Sent reload signal to clink (PID {pid}).");
        Ok(())
    }

    #[cfg(not(unix))]
    {
        Err("Reload is not supported on this platform.".to_string())
    }
}
