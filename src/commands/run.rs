use crate::clink::Clink;
use crate::config::{ClinkConfig, load_config};
use crate::runtime;
use copypasta::{ClipboardContext, ClipboardProvider};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::{thread, time::Duration};

pub fn execute(config_path: PathBuf, verbose: bool) -> Result<(), String> {
    runtime::write_pid_file()?;

    #[cfg(unix)]
    let signals = crate::signal::install_signal_handlers();

    let log_msg = format!(
        "[{}] clink {} started (PID {}, config: {:?})",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        env!("CARGO_PKG_VERSION"),
        std::process::id(),
        config_path
    );
    if verbose {
        println!("{log_msg}");
    }
    let _ = runtime::append_log(&log_msg);

    let mut cfg: ClinkConfig = load_config(&config_path)?;
    cfg.verbose = verbose;

    if verbose {
        println!("Config ({config_path:?}):\n {cfg:#?}");
    }

    let sleep_duration = Duration::from_millis(cfg.sleep_duration);
    let mut clink = Clink::new(cfg);
    let mut ctx: ClipboardContext =
        ClipboardContext::new().map_err(|e| format!("Failed to access clipboard: {e}"))?;
    let mut previous_clipboard = String::new();

    loop {
        #[cfg(unix)]
        {
            if signals.shutdown_requested.load(Ordering::SeqCst) {
                let msg = format!(
                    "[{}] clink shutting down (SIGTERM)",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                );
                if verbose {
                    println!("{msg}");
                }
                let _ = runtime::append_log(&msg);
                runtime::remove_pid_file();
                return Ok(());
            }

            if signals.reload_requested.load(Ordering::SeqCst) {
                signals.reload_requested.store(false, Ordering::SeqCst);
                let msg = format!(
                    "[{}] Reloading config from {:?}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    config_path
                );
                if verbose {
                    println!("{msg}");
                }
                let _ = runtime::append_log(&msg);

                match load_config(&config_path) {
                    Ok(mut new_cfg) => {
                        new_cfg.verbose = verbose;
                        clink = Clink::new(new_cfg);
                        let msg = format!(
                            "[{}] Config reloaded successfully",
                            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                        );
                        if verbose {
                            println!("{msg}");
                        }
                        let _ = runtime::append_log(&msg);
                    }
                    Err(e) => {
                        let msg = format!(
                            "[{}] Failed to reload config: {e}",
                            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                        );
                        eprintln!("{msg}");
                        let _ = runtime::append_log(&msg);
                    }
                }
            }
        }

        if let Ok(current_clipboard) = ctx.get_contents() {
            if previous_clipboard != current_clipboard {
                let cleaned = clink.find_and_replace(&current_clipboard);
                if cleaned != current_clipboard {
                    if let Err(e) = ctx.set_contents(cleaned.clone()) {
                        let msg = format!(
                            "[{}] Failed to set clipboard: {e}",
                            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                        );
                        eprintln!("{msg}");
                        let _ = runtime::append_log(&msg);
                    } else {
                        let msg = format!(
                            "[{}] Cleaned URL in clipboard",
                            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                        );
                        if verbose {
                            println!("{msg}");
                        }
                        let _ = runtime::append_log(&msg);
                    }
                }
                previous_clipboard = cleaned;
            }
        }
        thread::sleep(sleep_duration);
    }
}
