mod clink;
mod config;
mod expand_string;
mod mode;

use clink::Clink;
use config::{ClinkConfig, fallback_config_path, load_config};
use copypasta::{ClipboardContext, ClipboardProvider};
use dirs_next::config_dir;
use rustop::opts;
use std::{path::PathBuf, thread, time::Duration};

fn main() -> Result<(), confy::ConfyError> {
    let (args, _rest) = opts! {
        command_name "clink";
        synopsis "Clink automatically cleans url in your clipboard";
        version env!("CARGO_PKG_VERSION");
        opt verbose:bool, desc:"Be verbose.";
        opt config:String = fallback_config_path(config_dir()).into_os_string().into_string().unwrap(), desc: "config path."; 
    }
    .parse_or_exit();

    let config_path = PathBuf::from(args.config);

    let mut cfg: ClinkConfig = load_config(&config_path);
    cfg.verbose = args.verbose;

    if !config_path.is_file() {
        confy::store_path(&config_path, &cfg)?;
    }

    if args.verbose {
        println!("Clink {}", env!("CARGO_PKG_VERSION"));
        println!("\nConfig ({config_path:?}):\n {cfg:#?}");
    }
    let sleep_duration = Duration::from_millis(cfg.sleep_duration);
    let clink = Clink::new(cfg);
    let mut ctx: ClipboardContext = ClipboardContext::new().unwrap();
    let mut previous_clipboard = "".to_string();

    loop {
        match ctx.get_contents() {
            Ok(current_clipboard) => {
                if previous_clipboard != current_clipboard {
                    let cleaned = clink.find_and_replace(&current_clipboard);
                    if cleaned != current_clipboard {
                        ctx.set_contents(cleaned.clone()).unwrap();
                    }
                    previous_clipboard = cleaned;
                }
            }
            Err(_e) => {}
        }
        thread::sleep(sleep_duration);
    }
}
