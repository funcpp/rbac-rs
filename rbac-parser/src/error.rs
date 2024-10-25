use serde_json::{self};

#[derive(Debug)]
pub enum ParserError {
    SyntaxError(String),
    IoError(std::io::Error),
    SerdeError(serde_json::Error),
}

impl From<std::io::Error> for ParserError {
    fn from(err: std::io::Error) -> Self {
        ParserError::IoError(err)
    }
}

impl From<serde_json::Error> for ParserError {
    fn from(err: serde_json::Error) -> Self {
        ParserError::SerdeError(err)
    }
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParserError::SyntaxError(err) => write!(f, "Syntax error: {}", err),
            ParserError::IoError(err) => write!(f, "IO error: {}", err),
            ParserError::SerdeError(err) => write!(f, "Serde error: {}", err),
        }
    }
}
