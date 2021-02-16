mod mode;
mod utils;

use self::mode::Mode;
use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;
use rustop::opts;
use std::thread;
use std::time::Duration;
use utils::find_and_replace;

fn main() {
    let (args, _rest) = opts! {
        command_name "clink";
        synopsis "Clink automatically cleans url in your clipboard";    
        version env!("CARGO_PKG_VERSION");     
        opt verbose:bool, desc:"Be verbose.";           
        opt mode: Mode = Mode::Remove, desc:"Mode of clink. Available \"remove\" and \"your_mom\" modes"; 
    }.parse_or_exit();

    if args.verbose {
        println!("Clink is running with {} mode.", args.mode);
    }

    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    let mut previous_clipboard = "".to_string();

    loop {
        match ctx.get_contents() {
            Ok(current_clipboard) => {
                if previous_clipboard != current_clipboard {
                    let cleaned = find_and_replace(&current_clipboard, &args.mode);
                    if cleaned != current_clipboard {
                        ctx.set_contents(cleaned.clone()).unwrap();
                    }
                    previous_clipboard = cleaned;
                }
            }
            Err(_e) => {}
        }
        thread::sleep(Duration::from_millis(100))
    }
}
