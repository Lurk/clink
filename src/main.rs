mod mode;
mod params;
mod utils;

use mode::Mode;
use params::{create_index, get_default_params};
use utils::{fallback_config_path, find_and_replace};

use clipboard::{ClipboardContext, ClipboardProvider};
use dirs::config_dir;
use rustop::opts;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{path::PathBuf, thread};

#[derive(Serialize, Deserialize, Debug)]
pub struct ClinkConfig {
    mode: Mode,
    your_mom: String,
    except_mothers_day: bool,
    sleep_duration: u64,
    params: Vec<String>,
}

impl ::std::default::Default for ClinkConfig {
    fn default() -> Self {
        Self {
            mode: Mode::Remove,
            your_mom: "your_mom".to_string(),
            except_mothers_day: true,
            sleep_duration: 150,
            params: get_default_params(),
        }
    }
}

fn main() -> Result<(), confy::ConfyError> {
    let (args, _rest) = opts! {
        command_name "clink";
        synopsis "Clink automatically cleans url in your clipboard";
        version env!("CARGO_PKG_VERSION");
        opt verbose:bool, desc:"Be verbose.";
        opt config:String = fallback_config_path(config_dir()).into_os_string().into_string().unwrap(), desc: "config path";
    }
    .parse_or_exit();

    let config_path = PathBuf::from(args.config);
    let cfg: ClinkConfig = confy::load_path(&config_path)?;

    if !config_path.is_file() {
        confy::store_path(&config_path, &cfg)?;
    }

    if args.verbose {
        println!("Clink {}", env!("CARGO_PKG_VERSION"));
        println!("\nConfig ({:?}):\n {:#?}", config_path, cfg);
    }

    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    let mut previous_clipboard = "".to_string();
    let index = create_index(&cfg.params);
    loop {
        match ctx.get_contents() {
            Ok(current_clipboard) => {
                if previous_clipboard != current_clipboard {
                    let cleaned = find_and_replace(&current_clipboard, &cfg, &index);
                    if cleaned != current_clipboard {
                        ctx.set_contents(cleaned.clone()).unwrap();
                    }
                    previous_clipboard = cleaned;
                }
            }
            Err(_e) => {}
        }
        thread::sleep(Duration::from_millis(cfg.sleep_duration))
    }
}
