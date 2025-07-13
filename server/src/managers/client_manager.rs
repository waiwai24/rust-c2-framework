use common::message::{ClientInfo, CommandRequest, CommandResponse, Message}; // Import Message
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

/// ClientManager handles client registration, command management, and command results.
pub struct ClientManager {
    clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
    commands: Arc<RwLock<HashMap<String, Vec<CommandRequest>>>>,
    command_results: Arc<RwLock<HashMap<String, Vec<CommandResponse>>>>,
    // Store file operation responses, keyed by client_id and then message_id
    file_operation_responses: Arc<RwLock<HashMap<String, HashMap<String, Message>>>>,
}

/// Implement Default for ClientManager to allow easy instantiation
impl Default for ClientManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of ClientManager methods
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
    #[instrument(skip(self), fields(client_id = %client_id))]
    pub async fn update_heartbeat(&self, client_id: &str) {
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.get_mut(client_id) {
            let old_last_seen = client.last_seen;
            client.last_seen = chrono::Utc::now();
            debug!(
                client_id = %client_id,
                old_time = %old_last_seen,
                new_time = %client.last_seen,
                "Heartbeat updated successfully"
            );
        } else {
            warn!(
                client_id = %client_id,
                "Attempted to update heartbeat for non-existent client"
            );
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
    #[instrument(skip(self), fields(client_id = %client_id, command = %command.command))]
    pub async fn add_command(&self, client_id: &str, command: CommandRequest) {
        debug!(
            client_id = %client_id,
            command = %command.command,
            message_id = ?command.message_id,
            "Adding command to client queue"
        );

        let mut commands = self.commands.write().await;
        let queue_length_before = commands.get(client_id).map(|v| v.len()).unwrap_or(0);

        commands
            .entry(client_id.to_string())
            .or_insert_with(Vec::new)
            .push(command);

        info!(
            client_id = %client_id,
            queue_length = %(queue_length_before + 1),
            "Command queued successfully"
        );
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
        info!(
            "Storing file operation response: client_id={}, message_id={}",
            client_id, message.id
        );
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
        let mut responses = self.file_operation_responses.write().await;
        if let Some(client_responses) = responses.get_mut(client_id) {
            let result = client_responses.remove(message_id);
            info!("result: {:?}", result);
            result
        } else {
            None
        }
    }

    /// Clean up offline clients based on a timeout
    #[instrument(skip(self), fields(timeout_seconds = %timeout_seconds))]
    pub async fn cleanup_offline_clients(&self, timeout_seconds: i64) {
        let mut clients = self.clients.write().await;
        let now = chrono::Utc::now();
        let initial_count = clients.len();

        let mut removed_clients = Vec::new();
        clients.retain(|client_id, client| {
            let should_retain = (now.timestamp() - client.last_seen.timestamp()) < timeout_seconds;
            if !should_retain {
                removed_clients.push(client_id.clone());
            }
            should_retain
        });

        if !removed_clients.is_empty() {
            info!(
                count = %removed_clients.len(),
                clients = ?removed_clients,
                timeout_seconds = %timeout_seconds,
                "Cleaned up offline clients"
            );
        }

        debug!(
            initial_count = %initial_count,
            remaining_count = %clients.len(),
            removed_count = %removed_clients.len(),
            "Client cleanup completed"
        );
    }

    /// Delete a specific client by ID
    #[instrument(skip(self), fields(client_id = %client_id))]
    pub async fn delete_client(&self, client_id: &str) -> bool {
        info!(
            client_id = %client_id,
            "Attempting to delete client"
        );

        let mut clients = self.clients.write().await;
        let mut commands = self.commands.write().await;
        let mut command_results = self.command_results.write().await;
        let mut file_responses = self.file_operation_responses.write().await;

        // Record state before deletion
        let command_count = commands.get(client_id).map(|v| v.len()).unwrap_or(0);
        let result_count = command_results.get(client_id).map(|v| v.len()).unwrap_or(0);
        let file_response_count = file_responses.get(client_id).map(|v| v.len()).unwrap_or(0);

        // Remove client from all data structures
        let client_removed = clients.remove(client_id).is_some();
        commands.remove(client_id);
        command_results.remove(client_id);
        file_responses.remove(client_id);

        if client_removed {
            info!(
                client_id = %client_id,
                commands_cleared = %command_count,
                results_cleared = %result_count,
                file_responses_cleared = %file_response_count,
                "Client deleted successfully"
            );
        } else {
            warn!(
                client_id = %client_id,
                "Attempted to delete non-existent client"
            );
        }

        client_removed
    }
}
