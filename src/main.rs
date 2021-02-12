extern crate clipboard;

mod lib;

use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;
use lib::{find_and_replace, Mode};
use std::env;
use std::thread;
use std::time::Duration;

fn main() {
    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    let mut buff = "".to_string();
    let mode: Mode;
    match env::var("CLINK_MODE")
        .unwrap_or("Remove".to_string())
        .as_str()
    {
        "Remove" => mode = Mode::Remove,
        "YourMom" => mode = Mode::YourMom,
        _ => mode = Mode::Remove,
    }

    loop {
        match ctx.get_contents() {
            Ok(v) => {
                if buff != v {
                    ctx.set_contents(find_and_replace(&v, &mode)).unwrap();
                    buff = v;
                }
            }
            Err(_e) => {}
        }
        thread::sleep(Duration::from_millis(100))
    }
}
