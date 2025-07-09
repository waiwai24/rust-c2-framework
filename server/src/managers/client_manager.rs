use common::message::{ClientInfo, CommandRequest, CommandResponse, Message}; // Import Message
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// ClientManager handles client registration, command management, and command results.
pub struct ClientManager {
    clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
    commands: Arc<RwLock<HashMap<String, Vec<CommandRequest>>>>,
    command_results: Arc<RwLock<HashMap<String, Vec<CommandResponse>>>>,
    // Store file operation responses, keyed by client_id and then message_id
    file_operation_responses: Arc<RwLock<HashMap<String, HashMap<String, Message>>>>,
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
            file_operation_responses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new client
    pub async fn register_client(&self, client: ClientInfo) {
        let mut clients = self.clients.write().await;
        clients.insert(client.id.clone(), client);
    }

    /// Update the heartbeat for a specific client
    pub async fn update_heartbeat(&self, client_id: &str) {
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.get_mut(client_id) {
            client.last_seen = chrono::Utc::now();
        }
    }

    /// Get all clients
    pub async fn get_clients(&self) -> Vec<ClientInfo> {
        let clients = self.clients.read().await;
        clients.values().cloned().collect()
    }

    /// Get a specific client by ID
    pub async fn get_client(&self, client_id: &str) -> Option<ClientInfo> {
        let clients = self.clients.read().await;
        clients.get(client_id).cloned()
    }

    /// Add a command to a client's command queue
    pub async fn add_command(&self, client_id: &str, command: CommandRequest) {
        let mut commands = self.commands.write().await;
        commands
            .entry(client_id.to_string())
            .or_insert_with(Vec::new)
            .push(command);
    }

    /// Get all commands for a specific client
    pub async fn get_commands(&self, client_id: &str) -> Vec<CommandRequest> {
        let mut commands = self.commands.write().await;
        commands.remove(client_id).unwrap_or_default()
    }

    /// Add a command result
    pub async fn add_command_result(&self, result: CommandResponse) {
        let mut results = self.command_results.write().await;
        results
            .entry(result.client_id.clone())
            .or_insert_with(Vec::new)
            .push(result);
    }

    /// Get all command results for a specific client
    pub async fn get_command_results(&self, client_id: &str) -> Vec<CommandResponse> {
        let results = self.command_results.read().await;
        results.get(client_id).cloned().unwrap_or_default()
    }

    /// Add a file operation response
    pub async fn add_file_operation_response(&self, client_id: &str, message: Message) {
        let mut responses = self.file_operation_responses.write().await;
        responses
            .entry(client_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(message.id.clone(), message);
    }

    /// Get a specific file operation response by client ID and message ID
    pub async fn get_file_operation_response(
        &self,
        client_id: &str,
        message_id: &str,
    ) -> Option<Message> {
        let mut responses = self.file_operation_responses.write().await; // Use write lock to remove after retrieval
        if let Some(client_responses) = responses.get_mut(client_id) {
            client_responses.remove(message_id)
        } else {
            None
        }
    }

    /// Clean up offline clients based on a timeout
    pub async fn cleanup_offline_clients(&self, timeout_seconds: i64) {
        let mut clients = self.clients.write().await;
        let now = chrono::Utc::now();

        clients
            .retain(|_, client| (now.timestamp() - client.last_seen.timestamp()) < timeout_seconds);
    }

    /// Get the number of online clients
    pub async fn get_online_count(&self) -> usize {
        let clients = self.clients.read().await;
        let now = chrono::Utc::now();

        clients
            .values()
            .filter(|client| (now.timestamp() - client.last_seen.timestamp()) < 60)
            .count()
    }
}
