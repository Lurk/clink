use rustop::DefaultName;
use std::error::Error;
use std::fmt;
use std::str::FromStr;

pub enum Mode {
    Remove,
    YourMom,
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
            _ => Err(ModeError::new("Mode can be \"remove\" or \"your_mom\"")),
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Mode::Remove => write!(f, "Remove"),
            Mode::YourMom => write!(f, "YourMom"),
        }
    }
}

impl DefaultName for Mode {
    fn default_name() -> Option<&'static str> {
        Some("<Mode>")
    }
}
