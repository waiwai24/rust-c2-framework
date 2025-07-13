use crate::error::{C2Error, C2Result};
use serde::{Deserialize, Serialize};

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server listening address
    pub host: String,
    /// Server listening port
    pub port: u16,
    /// Port for reverse shell connections
    pub reverse_shell_port: u16,
    /// Encryption key
    pub encryption_key: String,
    /// Client timeout (seconds)
    pub client_timeout: u64,
    /// Maximum number of clients
    pub max_clients: usize,
    /// Log file path
    pub log_file: String,
    /// Enable audit log
    pub enable_audit: bool,
    /// Web configuration
    pub web: WebConfig,
    /// Authentication configuration
    pub auth: AuthConfig,
}

/// Web configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// Enable web interface
    pub enabled: bool,
    /// Static files directory
    pub static_dir: String,
    /// Template directory
    pub template_dir: String,
    /// Enable CORS(Cross-Origin Resource Sharing)
    pub enable_cors: bool,
    /// Auto refresh interval for web UI (seconds)
    pub refresh_interval: u64,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Username for authentication
    pub username: String,
    /// Password for authentication
    pub password: String,
}

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Server URL
    pub server_url: String,
    /// Client ID
    pub client_id: Option<String>,
    /// Encryption key
    pub encryption_key: String,
    /// Heartbeat interval (seconds)
    pub heartbeat_interval: u64,
    /// Command check interval (seconds)
    pub command_check_interval: u64,
    /// Connection retry count
    pub retry_count: u32,
    /// Connection retry interval (seconds)
    pub retry_interval: u64,
    /// Persistence configuration
    pub persistence: PersistenceConfig,
}

/// Persistence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Enable persistence
    pub enabled: bool,
    /// Startup method
    pub method: PersistenceMethod,
    /// Installation path
    pub install_path: String,
    /// Service name
    pub service_name: String,
}

/// Persistence method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PersistenceMethod {
    /// System service
    SystemService,
    /// Startup item
    StartupItem,
    /// Cron job
    CronJob,
    /// No persistence
    None,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            reverse_shell_port: 31229,
            encryption_key: "your-32-byte-secret-key-here!!!!".to_string(),
            client_timeout: 300,
            max_clients: 1000,
            log_file: "c2_server.log".to_string(),
            enable_audit: true,
            web: WebConfig::default(),
            auth: AuthConfig::default(),
        }
    }
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            static_dir: "web/static".to_string(),
            template_dir: "templates".to_string(),
            enable_cors: true,
            refresh_interval: 30,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            username: "".to_string(),
            password: "".to_string(),
        }
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_url: "http://127.0.0.1:8080".to_string(),
            client_id: None,
            encryption_key: "your-32-byte-secret-key-here!!!!".to_string(),
            heartbeat_interval: 30,
            command_check_interval: 2,
            retry_count: 3,
            retry_interval: 10,
            persistence: PersistenceConfig::default(),
        }
    }
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            method: PersistenceMethod::None,
            install_path: "".to_string(),
            service_name: "system_service".to_string(),
        }
    }
}

/// Configuration manager for loading and saving configurations
pub struct ConfigManager;

impl ConfigManager {
    /// Loads the server configuration from a file.
    pub fn load_server_config(path: &str) -> C2Result<ServerConfig> {
        let content = std::fs::read_to_string(path).map_err(C2Error::Io)?;
        let config: ServerConfig =
            toml::from_str(&content).map_err(|e| C2Error::Serialization(e.to_string()))?;
        Ok(config)
    }

    /// Saves the server configuration to a file.
    pub fn save_server_config(path: &str, config: &ServerConfig) -> C2Result<()> {
        let content =
            toml::to_string_pretty(config).map_err(|e| C2Error::Serialization(e.to_string()))?;
        std::fs::write(path, content).map_err(C2Error::Io)?;
        Ok(())
    }

    /// Loads the client configuration from a file.
    pub fn load_client_config(path: &str) -> C2Result<ClientConfig> {
        let content = std::fs::read_to_string(path).map_err(C2Error::Io)?;
        let config: ClientConfig =
            toml::from_str(&content).map_err(|e| C2Error::Serialization(e.to_string()))?;
        Ok(config)
    }

    /// Saves the client configuration to a file.
    pub fn save_client_config(path: &str, config: &ClientConfig) -> C2Result<()> {
        let content =
            toml::to_string_pretty(config).map_err(|e| C2Error::Serialization(e.to_string()))?;
        std::fs::write(path, content).map_err(C2Error::Io)?;
        Ok(())
    }
}
