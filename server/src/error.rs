use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::{error::Error as StdError, fmt, io};

/// Represents a file operation error
#[derive(Debug, Serialize, Deserialize)]
pub enum FileOperationError {
    NotFound,
    PermissionDenied,
    IoError(String),
    SerializationError(String),
    InvalidPath,
    Other(String),
}

/// Converts various error types into `FileOperationError`
impl From<io::Error> for FileOperationError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => FileOperationError::NotFound,
            io::ErrorKind::PermissionDenied => FileOperationError::PermissionDenied,
            _ => FileOperationError::IoError(err.to_string()),
        }
    }
}

/// Converts serde_json errors into `FileOperationError`
impl From<serde_json::Error> for FileOperationError {
    fn from(err: serde_json::Error) -> Self {
        FileOperationError::SerializationError(err.to_string())
    }
}

/// Converts string errors into `FileOperationError`
impl fmt::Display for FileOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileOperationError::NotFound => write!(f, "File or directory not found."),
            FileOperationError::PermissionDenied => write!(f, "Permission denied."),
            FileOperationError::IoError(msg) => write!(f, "IO Error: {}", msg),
            FileOperationError::SerializationError(msg) => {
                write!(f, "Serialization Error: {}", msg)
            }
            FileOperationError::InvalidPath => write!(f, "Invalid path provided."),
            FileOperationError::Other(msg) => write!(f, "An unexpected error occurred: {}", msg),
        }
    }
}

/// Implementing the StdError trait for FileOperationError
impl StdError for FileOperationError {}

/// Response structure for file operations
#[derive(Debug, Serialize)]
pub struct ServerFileOperationResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Converts `FileOperationError` into an Axum response
impl IntoResponse for FileOperationError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            FileOperationError::NotFound => (
                StatusCode::NOT_FOUND,
                "File or directory not found.".to_string(),
            ),
            FileOperationError::PermissionDenied => {
                (StatusCode::FORBIDDEN, "Permission denied.".to_string())
            }
            FileOperationError::IoError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("IO Error: {}", msg),
            ),
            FileOperationError::SerializationError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Serialization Error: {}", msg),
            ),
            FileOperationError::InvalidPath => (
                StatusCode::BAD_REQUEST,
                "Invalid path provided.".to_string(),
            ),
            FileOperationError::Other(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("An unexpected error occurred: {}", msg),
            ),
        };

        let body = Json(ServerFileOperationResponse {
            success: false,
            message: error_message,
            data: None,
        });

        (status, body).into_response()
    }
}
