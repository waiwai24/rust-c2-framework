use common::message::ShellSession;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// ShellManager manages shell sessions and their associated data.
pub struct ShellManager {
    sessions: Arc<RwLock<HashMap<String, ShellSession>>>,
    session_data: Arc<RwLock<HashMap<String, Vec<String>>>>,
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
        }
    }

    /// Create a new shell session
    pub async fn create_session(&self, client_id: &str) -> String {
        let session_id = Uuid::new_v4().to_string();
        let session = ShellSession {
            client_id: client_id.to_string(),
            session_id: session_id.clone(),
            created_at: chrono::Utc::now(),
            is_active: true,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);

        session_id
    }

    /// Get a specific shell session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<ShellSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Add data to a shell session
    pub async fn add_shell_data(&self, session_id: &str, data: String) {
        let mut session_data = self.session_data.write().await;
        session_data
            .entry(session_id.to_string())
            .or_insert_with(Vec::new)
            .push(data);
    }

    /// Get all data for a specific shell session
    pub async fn get_shell_data(&self, session_id: &str) -> Vec<String> {
        let session_data = self.session_data.read().await;
        session_data.get(session_id).cloned().unwrap_or_default()
    }

    /// Close a shell session
    pub async fn close_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.is_active = false;
        }
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
