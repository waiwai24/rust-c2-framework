use std::fmt;

/// C2 framework error type
#[derive(Debug)]
pub enum C2Error {
    Network(String),
    Crypto(String),
    Serialization(String),
    Io(std::io::Error),
    Other(String),
}

impl fmt::Display for C2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            C2Error::Network(msg) => write!(f, "Network error: {msg}"),
            C2Error::Crypto(msg) => write!(f, "Crypto error: {msg}"),
            C2Error::Serialization(msg) => write!(f, "Serialization error: {msg}"),
            C2Error::Io(err) => write!(f, "IO error: {err}"),
            C2Error::Other(msg) => write!(f, "Other error: {msg}"),
        }
    }
}

impl std::error::Error for C2Error {}

impl From<std::io::Error> for C2Error {
    fn from(err: std::io::Error) -> Self {
        C2Error::Io(err)
    }
}

impl From<serde_json::Error> for C2Error {
    fn from(err: serde_json::Error) -> Self {
        C2Error::Serialization(err.to_string())
    }
}

impl From<reqwest::Error> for C2Error {
    fn from(err: reqwest::Error) -> Self {
        C2Error::Network(err.to_string())
    }
}

/// Result type for C2 framework operations
pub type C2Result<T> = Result<T, C2Error>;
