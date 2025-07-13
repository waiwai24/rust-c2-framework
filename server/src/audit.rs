use chrono::Utc;
use common::message::{ClientInfo, CommandRequest, CommandResponse, ShellSession};
use std::fs::OpenOptions;
use std::io::Write;
use tracing::warn;

/// Audit logger for recording client actions and system events
pub struct AuditLogger {
    log_file: String,
}

/// Implementation of the AuditLogger
impl AuditLogger {
    pub fn new(log_file: &str) -> Self {
        Self {
            log_file: log_file.to_string(),
        }
    }

    /// Log client connection
    pub fn log_client_connect(&self, client_info: &ClientInfo) {
        let log_entry = format!(
            "[{}] CLIENT_CONNECT - ID: {}, Hostname: {}, User: {}, IP: {}, OS: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            client_info.id,
            client_info.hostname,
            client_info.username,
            client_info.ip,
            client_info.os
        );

        self.write_to_file(&log_entry);
    }

    /// Log client disconnection
    pub fn log_client_disconnect(&self, client_id: &str) {
        let log_entry = format!(
            "[{}] CLIENT_DISCONNECT - ID: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            client_id
        );

        self.write_to_file(&log_entry);
    }

    /// Log command execution
    pub fn log_command_execution(&self, cmd: &CommandRequest) {
        let log_entry = format!(
            "[{}] COMMAND_EXECUTE - Client: {}, Command: {} {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            cmd.client_id,
            cmd.command,
            cmd.args.join(" ")
        );

        self.write_to_file(&log_entry);
    }

    /// Log command result
    pub fn log_command_result(&self, result: &CommandResponse) {
        let log_entry = format!(
            "[{}] COMMAND_RESULT - Client: {}, Command: {}, ExitCode: {}, StdoutLines: {}, StderrLines: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            result.client_id,
            result.command,
            result.exit_code,
            result.stdout.lines().count(),
            result.stderr.lines().count()
        );

        self.write_to_file(&log_entry);
    }

    /// Log shell session
    pub fn log_shell_session(&self, session: &ShellSession) {
        let log_entry = format!(
            "[{}] SHELL_SESSION - Client: {}, Session: {}, Active: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            session.client_id,
            session.session_id,
            session.is_active
        );

        self.write_to_file(&log_entry);
    }

    /// Log error messages
    pub fn log_error(&self, error_msg: &str) {
        let log_entry = format!(
            "[{}] ERROR - {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            error_msg
        );

        self.write_to_file(&log_entry);
    }

    /// Log authentication failure
    pub fn log_authentication_failure(&self, username: &str, reason: &str, ip: Option<&str>) {
        let log_entry = format!(
            "[{}] AUTH_FAILURE - Username: {}, Reason: {}, IP: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            username,
            reason,
            ip.unwrap_or("unknown")
        );

        self.write_to_file(&log_entry);
    }

    /// Log session management events
    pub fn log_session_event(&self, username: &str, event_type: &str, session_token: &str) {
        let log_entry = format!(
            "[{}] SESSION_{} - Username: {}, Token: {}...",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            event_type,
            username,
            &session_token[..8.min(session_token.len())]
        );

        self.write_to_file(&log_entry);
    }

    /// Log file operation events
    pub fn log_file_operation(
        &self,
        client_id: &str,
        operation: &str,
        file_path: &str,
        file_size: Option<u64>,
        result: &str,
    ) {
        let size_info = file_size
            .map(|s| format!(", Size: {s}"))
            .unwrap_or_default();
        let log_entry = format!(
            "[{}] FILE_{} - Client: {}, Path: {}{}, Result: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            operation.to_uppercase(),
            client_id,
            file_path,
            size_info,
            result
        );

        self.write_to_file(&log_entry);
    }

    /// Log client lifecycle events
    pub fn log_client_lifecycle(&self, client_id: &str, event_type: &str, details: &str) {
        let log_entry = format!(
            "[{}] CLIENT_{} - ID: {}, Details: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            event_type.to_uppercase(),
            client_id,
            details
        );

        self.write_to_file(&log_entry);
    }

    /// Log WebSocket connection events
    pub fn log_websocket_event(&self, connection_id: &str, event_type: &str, details: &str) {
        let log_entry = format!(
            "[{}] WEBSOCKET_{} - ConnectionID: {}, Details: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            event_type.to_uppercase(),
            connection_id,
            details
        );

        self.write_to_file(&log_entry);
    }

    /// Write a log entry to the file
    fn write_to_file(&self, log_entry: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
        {
            if let Err(e) = writeln!(file, "{log_entry}") {
                warn!("Failed to write to log file: {e}");
            }
        } else {
            warn!("Failed to open log file: {}", self.log_file);
        }
    }
}
