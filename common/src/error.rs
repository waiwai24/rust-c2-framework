use std::fmt;

/// C2框架错误类型
#[derive(Debug)]
pub enum C2Error {
    /// 网络错误
    Network(String),
    /// 加密错误
    Crypto(String),
    /// 序列化错误
    Serialization(String),
    /// IO错误
    Io(std::io::Error),
    /// 其他错误
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

/// C2框架结果类型
pub type C2Result<T> = Result<T, C2Error>;
