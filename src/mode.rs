use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
pub enum Mode {
    #[serde(rename = "remove")]
    Remove,
    #[serde(rename = "replace")]
    Replace,
    #[serde(rename = "your_mom")]
    YourMom,
    #[serde(rename = "evil")]
    Evil,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Mode::Remove => write!(f, "Remove"),
            Mode::Replace => write!(f, "Replace"),
            Mode::YourMom => write!(f, "YourMom"),
            Mode::Evil => write!(f, "Evil"),
        }
    }
}
