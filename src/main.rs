mod mode;
mod utils;
mod params;

use params::get_default_params;
use mode::Mode;
use utils::find_and_replace;

use clipboard::{ClipboardContext, ClipboardProvider};
use rustop::opts;
use std::thread;
use std::time::Duration;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct ClinkConfig {
    mode: Mode,
    params: Vec<String>,
}

/// `MyConfig` implements `Default`
impl ::std::default::Default for ClinkConfig {
    fn default() -> Self { 
        Self { 
            mode: Mode::Remove, 
            params: get_default_params()
        } 
    }
}

fn main() -> Result<(), confy::ConfyError> {
    let (args, _rest) = opts! {
        command_name "clink";
        synopsis "Clink automatically cleans url in your clipboard";    
        version env!("CARGO_PKG_VERSION");     
        opt verbose:bool, desc:"Be verbose.";           
    }.parse_or_exit();

    let cfg: ClinkConfig = confy::load_path("./clink.toml")?;
    confy::store_path("./clink.toml", &cfg)?;

    if args.verbose {
        println!("Clink is running with {} mode.", cfg.mode);
        println!("Params that will be updated: {:?}", cfg.params);
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
