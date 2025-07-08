use std::net::{IpAddr, Ipv4Addr};
use std::process::Command;

pub fn get_local_ip() -> Result<IpAddr, Box<dyn std::error::Error>> {
    let output = Command::new("hostname")
        .arg("-I")
        .output()?;
    
    let ip_str = String::from_utf8(output.stdout)?;
    let ip = ip_str.trim().split_whitespace().next()
        .unwrap_or("127.0.0.1")
        .parse::<Ipv4Addr>()?;
    
    Ok(IpAddr::V4(ip))
}

pub fn get_hostname() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("hostname")
        .output()?;
    
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}
