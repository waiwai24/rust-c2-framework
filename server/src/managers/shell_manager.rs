use common::message::ShellSession;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// ShellManager manages shell sessions and their associated data.
use tokio::sync::{mpsc, broadcast}; // Import mpsc and broadcast

/// ShellManager manages shell sessions and their associated data.
pub struct ShellManager {
    sessions: Arc<RwLock<HashMap<String, ShellSession>>>,
    session_data: Arc<RwLock<HashMap<String, Vec<String>>>>,
    // New fields for WebSocket communication
    tx_to_shell: Arc<RwLock<HashMap<String, mpsc::Sender<Vec<u8>>>>>,
    tx_shell_output: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
}

impl Default for ShellManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_data: Arc::new(RwLock::new(HashMap::new())),
            tx_to_shell: Arc::new(RwLock::new(HashMap::new())),
            tx_shell_output: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new shell session and return its ID and communication channels
    pub async fn create_session(
        &self,
        client_id: &str,
    ) -> (
        String,
        mpsc::Receiver<Vec<u8>>,
        broadcast::Receiver<String>,
    ) {
        let session_id = Uuid::new_v4().to_string();
        let session = ShellSession {
            client_id: client_id.to_string(),
            session_id: session_id.clone(),
            created_at: chrono::Utc::now(),
            is_active: true,
        };

        let (tx_shell_input, rx_shell_input) = mpsc::channel::<Vec<u8>>(100);
        let (tx_shell_output, rx_shell_output) = broadcast::channel::<String>(100);

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }
        {
            let mut tx_to_shell_map = self.tx_to_shell.write().await;
            tx_to_shell_map.insert(session_id.clone(), tx_shell_input);
        }
        {
            let mut tx_shell_output_map = self.tx_shell_output.write().await;
            tx_shell_output_map.insert(session_id.clone(), tx_shell_output.clone());
        }

        (session_id, rx_shell_input, rx_shell_output)
    }

    /// Get a specific shell session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<ShellSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Get the mpsc sender for a specific shell session
    pub async fn get_tx_to_shell(&self, session_id: &str) -> Option<mpsc::Sender<Vec<u8>>> {
        let tx_map = self.tx_to_shell.read().await;
        tx_map.get(session_id).cloned()
    }

    /// Connect an existing session to a reverse shell connection
    /// Returns the receiver for this session if it exists
    pub async fn connect_reverse_shell(&self, session_id: &str) -> Option<mpsc::Receiver<Vec<u8>>> {
        let tx_map = self.tx_to_shell.read().await;
        if tx_map.contains_key(session_id) {
            // Create a new receiver that will get data from WebSocket
            let (tx, rx) = mpsc::channel::<Vec<u8>>(100);
            drop(tx_map);
            
            // Replace the sender in the map
            let mut tx_map = self.tx_to_shell.write().await;
            tx_map.insert(session_id.to_string(), tx);
            Some(rx)
        } else {
            None
        }
    }

    /// Subscribe to the broadcast receiver for shell output
    pub async fn subscribe_to_shell_output(&self, session_id: &str) -> broadcast::Receiver<String> {
        let tx_map = self.tx_shell_output.read().await;
        tx_map
            .get(session_id)
            .expect("Shell session not found for subscription")
            .subscribe()
    }

    /// Add data to a shell session and broadcast it
    pub async fn add_shell_data(&self, session_id: &str, data: String) {
        let mut session_data = self.session_data.write().await;
        session_data
            .entry(session_id.to_string())
            .or_insert_with(Vec::new)
            .push(data.clone()); // Store a clone for historical data

        // Broadcast the data to all subscribers
        if let Some(tx) = self.tx_shell_output.read().await.get(session_id) {
            let _ = tx.send(data); // Ignore error if no receivers
        }
    }

    /// Get all data for a specific shell session
    pub async fn get_shell_data(&self, session_id: &str) -> Vec<String> {
        let session_data = self.session_data.read().await;
        session_data.get(session_id).cloned().unwrap_or_default()
    }

    /// Close a shell session and clean up its channels
    pub async fn close_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.is_active = false;
        }
        // Remove senders to drop channels and close connections
        self.tx_to_shell.write().await.remove(session_id);
        self.tx_shell_output.write().await.remove(session_id);
        // Also remove historical data
        self.session_data.write().await.remove(session_id);
    }

    /// Clean up expired shell sessions based on a timeout
    pub async fn cleanup_expired_sessions(&self, timeout_seconds: i64) {
        let mut sessions = self.sessions.write().await;
        let mut session_data = self.session_data.write().await;
        let now = chrono::Utc::now();

        sessions.retain(|session_id, session| {
            let should_retain =
                (now.timestamp() - session.created_at.timestamp()) < timeout_seconds;
            if !should_retain {
                session_data.remove(session_id);
            }
            should_retain
        });
    }

    /// Get all shell sessions for a specific client
    pub async fn get_client_sessions(&self, client_id: &str) -> Vec<ShellSession> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|session| session.client_id == client_id)
            .cloned()
            .collect()
    }
}
