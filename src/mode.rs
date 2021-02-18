use std::fmt;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub enum Mode {
    #[serde(rename="remove")]
    Remove,
    #[serde(rename="your_mom")]
    YourMom,
    #[serde(rename="evil")]
    Evil,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Mode::Remove => write!(f, "Remove"),
            Mode::YourMom => write!(f, "YourMom"),
            Mode::Evil => write!(f, "Evil"),
        }
    }
}
