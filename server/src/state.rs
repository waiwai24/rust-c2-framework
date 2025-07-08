use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use common::*;

/// 服务器状态
#[derive(Clone)]
pub struct ServerState {
    pub clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
    pub commands: Arc<RwLock<HashMap<String, Vec<CommandRequest>>>>,
    pub command_results: Arc<RwLock<HashMap<String, Vec<CommandResponse>>>>,
    pub shell_sessions: Arc<RwLock<HashMap<String, ShellSession>>>,
    pub session_tokens: Arc<RwLock<HashMap<String, chrono::DateTime<chrono::Utc>>>>,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            commands: Arc::new(RwLock::new(HashMap::new())),
            command_results: Arc::new(RwLock::new(HashMap::new())),
            shell_sessions: Arc::new(RwLock::new(HashMap::new())),
            session_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
