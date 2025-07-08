use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// 客户端与服务端之间的通信协议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub message_type: MessageType,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    /// 客户端注册
    ClientRegister,
    /// 客户端心跳
    Heartbeat,
    /// 执行命令
    ExecuteCommand,
    /// 命令结果
    CommandResult,
    /// 反弹Shell请求
    ReverseShell,
    /// Shell数据
    ShellData,
    /// 文件传输
    FileTransfer,
    /// 系统信息
    SystemInfo,
    /// 错误消息
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub id: String,
    pub hostname: String,
    pub username: String,
    pub os: String,
    pub arch: String,
    pub ip: String,
    pub connected_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    pub client_id: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    pub client_id: String,
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSession {
    pub client_id: String,
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellData {
    pub session_id: String,
    pub data: Vec<u8>,
    pub timestamp: DateTime<Utc>,
}

impl ShellData {
    pub fn new(session_id: String, data: Vec<u8>) -> Self {
        Self {
            session_id,
            data,
            timestamp: Utc::now(),
        }
    }
}

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
