use std::{error::Error, fmt};

#[derive(Debug, Default)]
pub struct WyagError {
    _message: String,
}

impl WyagError {
    pub fn new(message: &str) -> WyagError {
        WyagError {
            _message: String::from(message),
        }
    }
}

impl Error for WyagError {
    fn description(&self) -> &str {
        self._message.as_ref()
    }
}

impl fmt::Display for WyagError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Failed to do task")
    }
}
