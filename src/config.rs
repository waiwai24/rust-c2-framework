use serde::{Deserialize, Serialize};
/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// 服务器监听地址
    pub host: String,
    /// 服务器监听端口
    pub port: u16,
    /// 加密密钥
    pub encryption_key: String,
    /// 客户端超时时间（秒）
    pub client_timeout: u64,
    /// 最大客户端数量
    pub max_clients: usize,
    /// 日志文件路径
    pub log_file: String,
    /// 启用审计日志
    pub enable_audit: bool,
    /// Web界面配置
    pub web: WebConfig,
}

/// Web界面配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// 启用Web界面
    pub enabled: bool,
    /// 静态文件目录
    pub static_dir: String,
    /// 模板目录
    pub template_dir: String,
    /// 启用CORS
    pub enable_cors: bool,
}

/// 客户端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// 服务器URL
    pub server_url: String,
    /// 客户端标识
    pub client_id: Option<String>,
    /// 加密密钥
    pub encryption_key: String,
    /// 心跳间隔（秒）
    pub heartbeat_interval: u64,
    /// 命令检查间隔（秒）
    pub command_check_interval: u64,
    /// 连接重试次数
    pub retry_count: u32,
    /// 连接重试间隔（秒）
    pub retry_interval: u64,
    /// 持久化配置
    pub persistence: PersistenceConfig,
}

/// 持久化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// 启用持久化
    pub enabled: bool,
    /// 自启动方式
    pub method: PersistenceMethod,
    /// 安装路径
    pub install_path: String,
    /// 服务名称
    pub service_name: String,
}

/// 持久化方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PersistenceMethod {
    /// 系统服务
    SystemService,
    /// 启动项
    StartupItem,
    /// 定时任务
    CronJob,
    /// 无持久化
    None,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            encryption_key: "default_key_change_me".to_string(),
            client_timeout: 300,
            max_clients: 1000,
            log_file: "c2_server.log".to_string(),
            enable_audit: true,
            web: WebConfig::default(),
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
        }
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_url: "http://127.0.0.1:8080".to_string(),
            client_id: None,
            encryption_key: "default_key_change_me".to_string(),
            heartbeat_interval: 30,
            command_check_interval: 5,
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

/// 配置管理器
pub struct ConfigManager;

impl ConfigManager {
    /// 加载服务器配置
    pub fn load_server_config(path: &str) -> Result<ServerConfig, Box<dyn std::error::Error>> {
        if std::path::Path::new(path).exists() {
            let content = std::fs::read_to_string(path)?;
            let config: ServerConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = ServerConfig::default();
            Self::save_server_config(path, &config)?;
            Ok(config)
        }
    }

    /// 保存服务器配置
    pub fn save_server_config(path: &str, config: &ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(config)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 加载客户端配置
    pub fn load_client_config(path: &str) -> Result<ClientConfig, Box<dyn std::error::Error>> {
        if std::path::Path::new(path).exists() {
            let content = std::fs::read_to_string(path)?;
            let config: ClientConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = ClientConfig::default();
            Self::save_client_config(path, &config)?;
            Ok(config)
        }
    }

    /// 保存客户端配置
    pub fn save_client_config(path: &str, config: &ClientConfig) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(config)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
