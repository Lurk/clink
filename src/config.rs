use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process,
    rc::Rc,
};

use serde::{Deserialize, Serialize};

use crate::mode::Mode;

fn get_default_params() -> HashSet<Rc<str>> {
    HashSet::from([
        "fbclid".into(),
        "gclid".into(),
        "gclsrc".into(),
        "dclid".into(),
        "zanpid".into(),
        "utm_source".into(),
        "utm_campaign".into(),
        "utm_medium".into(),
        "utm_term".into(),
        "utm_content".into(),
    ])
}

fn get_default_exit() -> Vec<Vec<Rc<str>>> {
    vec![
    vec!["vk.com/away.php".into(), "to".into()],
    vec!["exit.sc/".into(), "url".into()],
    vec!["facebook.com/(l|confirmemail|login).php".into(), "u".into(), "next".into()],
    vec!["(www.|)(encrypted.|)google.(at|be|ca|ch|co.(bw|il|uk)|com(|.(ar|au|br|eg|tr|tw))|cl|de|dk|es|fr|nl|pl|se)/url".into(), "url".into()],
    vec!["l.instagram.com/".into(), "u".into()],
    vec!["youtube.com/redirect".into(), "q".into()],
    vec!["linkedin.com/authwall".into(), "sessionRedirect".into()],
    vec!["mora.jp/cart".into(), "returnUrl".into()],
    ]
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClinkConfig {
    pub mode: Mode,
    pub replace_to: String,
    pub sleep_duration: u64,
    pub params: HashSet<Rc<str>>,
    pub exit: Vec<Vec<Rc<str>>>,
}

impl ClinkConfig {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            replace_to: "clink".into(),
            sleep_duration: 150,
            params: get_default_params(),
            exit: get_default_exit(),
        }
    }
}

impl Default for ClinkConfig {
    fn default() -> Self {
        Self::new(Mode::Remove)
    }
}

pub fn load_config(config_path: &Path) -> ClinkConfig {
    match confy::load_path(config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            println!("Clink {}\nConfig error\n", env!("CARGO_PKG_VERSION"));
            println!("looks like you have bad config or config for an old version");
            println!("Look at: {:?}\n", config_path);
            println!(
                "config should look like this:\n\n{}",
                toml::to_string_pretty(&ClinkConfig::default()).unwrap()
            );

            eprintln!("original error:\n {e:#?}");
            process::exit(1);
        }
    }
}

pub fn fallback_config_path(path: Option<PathBuf>) -> PathBuf {
    let p = match path {
        Some(p) => p.join("clink"),
        None => std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf(),
    };

    p.join("config.toml")
}
