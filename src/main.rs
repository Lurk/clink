extern crate clipboard;

mod lib;

use std::time::Duration;
use std::thread;
use clipboard::ClipboardProvider;
use clipboard::ClipboardContext;
use lib::{find_and_replace, ProcessTypes};

fn main() {
    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    let mut buff = "".to_string();
    loop {
        match ctx.get_contents() {
            Ok(v) => {
                if buff != v {
                    ctx.set_contents(find_and_replace(&v, &ProcessTypes::REMOVE)).unwrap();
                    buff = v;
                }
            },
            Err(_e) => {}
        }
        thread::sleep(Duration::from_millis(100))
    }
}
