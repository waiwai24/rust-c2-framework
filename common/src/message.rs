use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// C2 framework message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub message_type: MessageType,
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
    /// File transfer
    FileTransfer,
    /// System information
    SystemInfo,
    /// Error message
    Error,
}

/// Client information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub id: String,
    pub hostname: String,
    pub username: String,
    pub os: String,
    pub arch: String,
    pub ip: String,
    pub country_info: Option<String>,
    pub cpu_brand: String,
    pub cpu_frequency: u64,
    pub cpu_cores: usize,
    pub memory: u64,
    pub total_disk_space: u64,
    pub available_disk_space: u64,
    pub connected_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

/// Command request and response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    pub client_id: String,
    pub command: String,
    pub args: Vec<String>,
}

/// Command response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    pub client_id: String,
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub executed_at: DateTime<Utc>,
}

/// Reverse shell session structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSession {
    pub client_id: String,
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

/// Shell data structure for reverse shell communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellData {
    pub session_id: String,
    pub data: Vec<u8>,
    pub timestamp: DateTime<Utc>,
}

/// Implementation of methods for ShellData
impl ShellData {
    pub fn new(session_id: String, data: Vec<u8>) -> Self {
        Self {
            session_id,
            data,
            timestamp: Utc::now(),
        }
    }
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
