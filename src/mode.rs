use rustop::DefaultName;
use std::error::Error;
use std::fmt;
use std::str::FromStr;

pub enum Mode {
    Remove,
    YourMom,
    Evil,
}

#[derive(Debug)]
pub struct ModeError {
    details: String,
}

impl ModeError {
    fn new(msg: &str) -> ModeError {
        ModeError {
            details: msg.to_string(),
        }
    }
}

impl Error for ModeError {
    fn description(&self) -> &str {
        &self.details
    }
}

impl fmt::Display for ModeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl FromStr for Mode {
    type Err = ModeError;
    fn from_str(day: &str) -> Result<Self, Self::Err> {
        match day {
            "remove" => Ok(Mode::Remove),
            "your_mom" => Ok(Mode::YourMom),
            "evil" => Ok(Mode::Evil),
            _ => Err(ModeError::new(
                "Mode can be \"remove\", \"your_mom\" or \"evil\"",
            )),
        }
    }
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

impl DefaultName for Mode {
    fn default_name() -> Option<&'static str> {
        Some("<Mode>")
    }
}
