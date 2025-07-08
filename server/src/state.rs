use crate::audit::AuditLogger;
use crate::managers::client_manager::ClientManager;
use crate::managers::shell_manager::ShellManager;
use common::config::ServerConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application state
#[derive(Clone)]
pub struct AppState {
    pub client_manager: Arc<ClientManager>,
    pub shell_manager: Arc<ShellManager>,
    pub audit_logger: Arc<AuditLogger>,
    pub config: Arc<ServerConfig>,
    pub session_tokens: Arc<RwLock<HashMap<String, chrono::DateTime<chrono::Utc>>>>,
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        let audit_logger = if config.enable_audit {
            AuditLogger::new(&config.log_file)
        } else {
            // This is a simple way to disable logging. A more robust solution
            // might involve a logger that implements a trait and has a `NoOp` variant.
            AuditLogger::new("/dev/null")
        };

        Self {
            client_manager: Arc::new(ClientManager::new()),
            shell_manager: Arc::new(ShellManager::new()),
            audit_logger: Arc::new(audit_logger),
            config: Arc::new(config),
            session_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
