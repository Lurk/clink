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
        crate::signal::send_signal(pid, nix::sys::signal::Signal::SIGTERM)?;
        println!("Sent stop signal to clink (PID {pid}).");

        // Wait for process to exit
        for _ in 0..50 {
            if !runtime::is_running(pid) {
                println!("clink stopped.");
                println!(
                    "Start it again with `clink` or use `clink install` for automatic restarts."
                );
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        println!("clink (PID {pid}) did not stop within 5 seconds.");
        println!("You may need to kill it manually: kill -9 {pid}");
    }

    #[cfg(not(unix))]
    {
        return Err("Restart is not supported on this platform.".to_string());
    }

    Ok(())
}
