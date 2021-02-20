mod mode;
mod params;
mod utils;

use mode::Mode;
use params::get_default_params;
use utils::find_and_replace;

use clipboard::{ClipboardContext, ClipboardProvider};
use rustop::opts;
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug)]
pub struct ClinkConfig {
    mode: Mode,
    params: Vec<String>,
}

impl ::std::default::Default for ClinkConfig {
    fn default() -> Self {
        Self {
            mode: Mode::Remove,
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
    }
    .parse_or_exit();

    let config_path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("clink.toml");
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

    loop {
        match ctx.get_contents() {
            Ok(current_clipboard) => {
                if previous_clipboard != current_clipboard {
                    let cleaned = find_and_replace(&current_clipboard, &cfg);
                    if cleaned != current_clipboard {
                        ctx.set_contents(cleaned.clone()).unwrap();
                    }
                    previous_clipboard = cleaned;
                }
            }
            Err(_e) => {}
        }
        thread::sleep(Duration::from_millis(150))
    }
}
