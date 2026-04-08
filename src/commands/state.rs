use crate::runtime;

#[allow(clippy::unnecessary_wraps)]
pub fn execute() -> Result<(), String> {
    let pid = runtime::read_pid();

    match pid {
        Some(pid) if runtime::is_running(pid) => {
            println!("clink is running (PID {pid})");
        }
        Some(pid) => {
            println!("clink is not running (stale PID file for PID {pid})");
            runtime::remove_pid_file();
        }
        None => {
            println!("clink is not running");
        }
    }

    let log_path = runtime::log_file_path();
    println!("\nLog file: {}", log_path.display());

    let lines = runtime::read_last_log_lines(20);
    if lines.is_empty() {
        println!("(no log entries)");
    } else {
        println!("\nLast log entries:");
        for line in &lines {
            println!("  {line}");
        }
    }

    Ok(())
}
