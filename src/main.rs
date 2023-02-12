mod mode;
mod params;
mod utils;

use mode::Mode;
use params::{create_index, get_default_params};
use utils::{fallback_config_path, find_and_replace};

use copypasta::{ClipboardContext, ClipboardProvider};
use dirs_next::config_dir;
use rustop::opts;
use serde::{Deserialize, Serialize};

use std::{
    path::{Path, PathBuf},
    process, thread,
    time::Duration,
};

use linkify::{LinkFinder, LinkKind};

#[derive(Serialize, Deserialize, Debug)]
pub struct ClinkConfig {
    mode: Mode,
    replace_to: String,
    sleep_duration: u64,
    params: Vec<String>,
}

impl ::std::default::Default for ClinkConfig {
    fn default() -> Self {
        Self {
            mode: Mode::Remove,
            replace_to: "aHR0cHM6Ly95b3V0dS5iZS9kUXc0dzlXZ1hjUQ==".to_string(),
            sleep_duration: 150,
            params: get_default_params(),
        }
    }
}

fn load_config(config_path: &Path) -> ClinkConfig {
    match confy::load_path(config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            println!("Clink {}\nConfig error\n", env!("CARGO_PKG_VERSION"));
            println!("looks like you have bad config or config for an old version");
            println!("Look at: {config_path:?}\n");
            println!(
                "config should look like this:\n\n{}",
                toml::to_string(&ClinkConfig::default()).unwrap()
            );

            eprintln!("original error:\n {e:#?}");
            process::exit(1);
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

    let cfg: ClinkConfig = load_config(&config_path);

    if !config_path.is_file() {
        confy::store_path(&config_path, &cfg)?;
    }

    if args.verbose {
        println!("Clink {}", env!("CARGO_PKG_VERSION"));
        println!("\nConfig ({config_path:?}):\n {cfg:#?}");
    }

    let mut ctx: ClipboardContext = ClipboardContext::new().unwrap();
    let mut previous_clipboard = "".to_string();
    let index = create_index(&cfg.params);
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Url]);
    let sleep_duration = Duration::from_millis(cfg.sleep_duration);
    loop {
        match ctx.get_contents() {
            Ok(current_clipboard) => {
                if previous_clipboard != current_clipboard {
                    let cleaned = find_and_replace(&current_clipboard, &cfg, &index, &finder);
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
