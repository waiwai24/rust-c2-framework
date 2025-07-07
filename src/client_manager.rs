use common::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 客户端管理器
pub struct ClientManager {
    clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
    commands: Arc<RwLock<HashMap<String, Vec<CommandRequest>>>>,
    command_results: Arc<RwLock<HashMap<String, Vec<CommandResponse>>>>,
}

impl Default for ClientManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            commands: Arc::new(RwLock::new(HashMap::new())),
            command_results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册客户端
    pub async fn register_client(&self, client: ClientInfo) {
        let mut clients = self.clients.write().await;
        clients.insert(client.id.clone(), client);
    }

    /// 更新客户端心跳
    pub async fn update_heartbeat(&self, client_id: &str) {
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.get_mut(client_id) {
            client.last_seen = chrono::Utc::now();
        }
    }

    /// 获取所有客户端
    pub async fn get_clients(&self) -> Vec<ClientInfo> {
        let clients = self.clients.read().await;
        clients.values().cloned().collect()
    }

    /// 获取指定客户端
    pub async fn get_client(&self, client_id: &str) -> Option<ClientInfo> {
        let clients = self.clients.read().await;
        clients.get(client_id).cloned()
    }

    /// 添加命令到队列
    pub async fn add_command(&self, client_id: &str, command: CommandRequest) {
        let mut commands = self.commands.write().await;
        commands
            .entry(client_id.to_string())
            .or_insert_with(Vec::new)
            .push(command);
    }

    /// 获取客户端命令队列
    pub async fn get_commands(&self, client_id: &str) -> Vec<CommandRequest> {
        let mut commands = self.commands.write().await;
        commands.remove(client_id).unwrap_or_default()
    }

    /// 添加命令结果
    pub async fn add_command_result(&self, result: CommandResponse) {
        let mut results = self.command_results.write().await;
        results
            .entry(result.client_id.clone())
            .or_insert_with(Vec::new)
            .push(result);
    }

    /// 获取命令结果
    pub async fn get_command_results(&self, client_id: &str) -> Vec<CommandResponse> {
        let results = self.command_results.read().await;
        results.get(client_id).cloned().unwrap_or_default()
    }

    /// 清理离线客户端
    pub async fn cleanup_offline_clients(&self, timeout_seconds: i64) {
        let mut clients = self.clients.write().await;
        let now = chrono::Utc::now();

        clients
            .retain(|_, client| (now.timestamp() - client.last_seen.timestamp()) < timeout_seconds);
    }

    /// 获取在线客户端数量
    pub async fn get_online_count(&self) -> usize {
        let clients = self.clients.read().await;
        let now = chrono::Utc::now();

        clients
            .values()
            .filter(|client| (now.timestamp() - client.last_seen.timestamp()) < 60)
            .count()
    }
}
