use hostname::get;
use reqwest::blocking::Client;
use serde_json::json;
use serde_json::Value;
use std::net::{IpAddr, UdpSocket};
use sysinfo::{Disks, System};

pub fn get_local_ip() -> Result<IpAddr, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    // pretend to connect to google address
    socket.connect("8.8.8.8:80")?;
    Ok(socket.local_addr()?.ip())
}

pub fn get_hostname() -> Result<String, Box<dyn std::error::Error>> {
    Ok(get()?.into_string().unwrap_or("localhost".to_string()))
}

pub fn get_country(ip: String) -> Result<String, Box<dyn std::error::Error>> {
    let mut result = {
        let url = format!("https://ipapi.co/{ip}/json/");
        let client = Client::new();
        let response = client
            .get(&url)
            .header("Referer", "https://ipapi.co/")
            .send()?;

        let json: Value = response.json()?;
        Ok(format!(
            "国家: {}\n地区: {}\n城市: {}\nISP: {}",
            json["country_name"].as_str().unwrap_or("未知"),
            json["region"].as_str().unwrap_or("未知"),
            json["city"].as_str().unwrap_or("未知"),
            json["org"].as_str().unwrap_or("未知")
        ))
    };
    if result.is_err() {
        result = Ok("无法获取国家信息".to_string());
    }
    result
}

pub fn get_hardware_info() -> Result<String, Box<dyn std::error::Error>> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_brand = sys
        .cpus()
        .first()
        .map(|cpu| cpu.brand().to_string())
        .unwrap_or_default();
    let cpu_frequency = sys.cpus().first().map(|cpu| cpu.frequency()).unwrap_or(0);
    let cpu_cores = sys.cpus().len();

    let cpu_info = json!({
        "brand": cpu_brand,
        "frequency": cpu_frequency,
        "cores": cpu_cores,
    });

    let memory = sys.total_memory() / 1024 / 1024 / 1024;

    let disks = Disks::new_with_refreshed_list();
    let mut total_disk_space = 0u64;
    let mut total_available_space = 0u64;
    for disk in disks.iter() {
        total_disk_space += disk.total_space() / 1024 / 1024 / 1024;
        total_available_space += disk.available_space() / 1024 / 1024 / 1024;
    }

    // 构建完整JSON
    let info = json!({
        "cpu_brand": cpu_info["brand"],
        "cpu_frequency_MHz": cpu_info["frequency"],
        "cpu_cores": cpu_info["cores"],
        "memory_GB": memory,
        "total_disk_space_GB": total_disk_space,
        "available_disk_space_GB": total_available_space,
    });

    Ok(serde_json::to_string_pretty(&info)?)
}
