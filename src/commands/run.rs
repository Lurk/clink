use crate::clink::Clink;
use crate::config::{ClinkConfig, load_config};
use crate::runtime;
use copypasta::{ClipboardContext, ClipboardProvider};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::{thread, time::Duration};

fn log(verbose: bool, msg: &str) {
    let stamped = format!(
        "[{}] {msg}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    if verbose {
        println!("{stamped}");
    }
    let _ = runtime::append_log(&stamped);
}

fn log_err(msg: &str) {
    let stamped = format!(
        "[{}] {msg}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    eprintln!("{stamped}");
    let _ = runtime::append_log(&stamped);
}

pub fn execute(config_path: &Path, verbose: bool) -> Result<(), String> {
    runtime::write_pid_file()?;

    #[cfg(unix)]
    let signals = crate::signal::install_signal_handlers();

    log(
        verbose,
        &format!(
            "clink {} started (PID {}, config: {})",
            env!("CARGO_PKG_VERSION"),
            std::process::id(),
            config_path.display()
        ),
    );

    let mut cfg: ClinkConfig = load_config(config_path)?;
    cfg.verbose = verbose;

    if verbose {
        println!("Config ({}):\n {cfg:#?}", config_path.display());
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
                log(verbose, "clink shutting down (SIGTERM)");
                runtime::remove_pid_file();
                return Ok(());
            }

            if signals.reload_requested.load(Ordering::SeqCst) {
                signals.reload_requested.store(false, Ordering::SeqCst);
                log(
                    verbose,
                    &format!("Reloading config from {}", config_path.display()),
                );

                match load_config(config_path) {
                    Ok(mut new_cfg) => {
                        new_cfg.verbose = verbose;
                        clink = Clink::new(new_cfg);
                        log(verbose, "Config reloaded successfully");
                    }
                    Err(e) => {
                        log_err(&format!("Failed to reload config: {e}"));
                    }
                }
            }
        }

        if let Ok(current_clipboard) = ctx.get_contents() {
            if previous_clipboard != current_clipboard {
                let cleaned = clink.find_and_replace(&current_clipboard);
                if cleaned != current_clipboard {
                    if let Err(e) = ctx.set_contents(cleaned.clone()) {
                        log_err(&format!("Failed to set clipboard: {e}"));
                    } else {
                        log(verbose, "Cleaned URL in clipboard");
                    }
                }
                previous_clipboard = cleaned;
            }
        }
        thread::sleep(sleep_duration);
    }
}
