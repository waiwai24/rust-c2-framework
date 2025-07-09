use chrono::Utc;
use common::message::{ClientInfo, CommandRequest, CommandResponse, ShellSession};
use log::{error, info, warn};
use std::fs::OpenOptions;
use std::io::Write;

/// Audit logger for recording client actions and system events
pub struct AuditLogger {
    log_file: String,
}

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

        info!("{log_entry}");
        self.write_to_file(&log_entry);
    }

    /// Log client disconnection
    pub fn log_client_disconnect(&self, client_id: &str) {
        let log_entry = format!(
            "[{}] CLIENT_DISCONNECT - ID: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            client_id
        );

        info!("{log_entry}");
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

        info!("{log_entry}");
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

        info!("{log_entry}");
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

        info!("{log_entry}");
        self.write_to_file(&log_entry);
    }

    /// Log error messages
    pub fn log_error(&self, error_msg: &str) {
        let log_entry = format!(
            "[{}] ERROR - {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            error_msg
        );

        error!("{log_entry}");
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
