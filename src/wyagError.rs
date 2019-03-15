use std::{error::Error, fmt};


#[derive(Debug, Default)]
pub struct WyagError<'a> {
    _message: &'a str,
}

impl<'a> WyagError<'a> {
    pub fn new(message: &'a str) -> WyagError {
        WyagError {
            _message: message,
        }
    }
}

impl<'a> Error for WyagError<'a> {
    fn description(&self) -> &str {
        self._message
    }
}

impl<'a> fmt::Display for WyagError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Failed to do task")
    }
}