use crate::clink::Clink;
use crate::config::{ClinkConfig, load_config};
use crate::runtime;
use crate::stats;
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

    crate::remote::resolve_patterns(&mut cfg, &runtime::data_dir());

    if let Err(e) = runtime::write_loaded_config(&cfg) {
        log_err(&format!("Failed to write loaded config: {e}"));
    }

    if verbose {
        println!("Config ({}):\n {cfg:#?}", config_path.display());
    }

    let sleep_duration = Duration::from_millis(cfg.sleep_duration);
    let mut clink = Clink::new(cfg);
    let mut ctx: ClipboardContext =
        ClipboardContext::new().map_err(|e| format!("Failed to access clipboard: {e}"))?;
    let mut previous_clipboard = String::new();
    let stats_path = runtime::stats_file_path();
    let mut statistics = stats::load(&stats_path);
    statistics.reset_session();

    loop {
        #[cfg(unix)]
        {
            if signals.shutdown_requested.load(Ordering::SeqCst) {
                log(verbose, "clink shutting down (SIGTERM)");
                let _ = stats::save(&statistics, &stats_path);
                runtime::remove_pid_file();
                runtime::remove_loaded_config();
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
                        crate::remote::resolve_patterns(&mut new_cfg, &runtime::data_dir());
                        if let Err(e) = runtime::write_loaded_config(&new_cfg) {
                            log_err(&format!("Failed to write loaded config: {e}"));
                        }
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
                statistics.check_rollovers();
                statistics.increment(0, 0, 0, 1);
                let result = clink.find_and_replace(&current_clipboard);
                if result.text != current_clipboard {
                    statistics.increment(
                        result.urls_cleaned,
                        result.params_removed,
                        result.exits_unwrapped,
                        0,
                    );
                    if let Err(e) = ctx.set_contents(result.text.clone()) {
                        log_err(&format!("Failed to set clipboard: {e}"));
                    }

                    if let Err(e) = stats::save(&statistics, &stats_path) {
                        log_err(&format!("Failed to save stats: {e}"));
                    }
                }
                previous_clipboard = result.text;
            }
        }
        thread::sleep(sleep_duration);
    }
}
