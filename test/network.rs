//! ```cargo
//! [dependencies]
//! hostname = "0.3"
//! local-ip-address = "0.5"
//! ```

pub mod network {
    use std::net::IpAddr;
    
    #[derive(Debug, Clone)]
    pub struct SystemInfo {
        pub hostname: String,
        pub local_ip: IpAddr,
        pub username: String,
        pub os: String,
        pub arch: String,
    }
    
    pub fn get_system_info() -> Result<SystemInfo, Box<dyn std::error::Error>> {
        Ok(SystemInfo {
            hostname: get_hostname()?,
            local_ip: get_local_ip()?,
            username: get_username()?,
            os: get_os_info(),
            arch: get_arch_info(),
        })
    }
    
    pub fn get_hostname() -> Result<String, Box<dyn std::error::Error>> {
        hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .map_err(|e| e.into())
    }
    
    pub fn get_local_ip() -> Result<IpAddr, Box<dyn std::error::Error>> {
        local_ip_address::local_ip().map_err(|e| e.into())
    }
    
    pub fn get_username() -> Result<String, Box<dyn std::error::Error>> {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .map_err(|e| e.into())
    }
    
    pub fn get_os_info() -> String {
        std::env::consts::OS.to_string()
    }
    
    pub fn get_arch_info() -> String {
        std::env::consts::ARCH.to_string()
    }
    
    // 安全的后备方案
    pub fn get_hostname_fallback() -> String {
        hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    }
    
    pub fn get_local_ip_fallback() -> IpAddr {
        local_ip_address::local_ip()
            .unwrap_or_else(|_| IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)))
    }
}

fn main() {
    println!("Testing network module...");
    
    // 测试基本功能
    match network::get_system_info() {
        Ok(info) => {
            println!("✓ System Info: {:?}", info);
        }
        Err(e) => {
            eprintln!("✗ Error retrieving system info: {}", e);
        }
    }
    
    // 测试单独的功能
    println!("\nTesting individual functions:");
    
    match network::get_hostname() {
        Ok(hostname) => println!("✓ Hostname: {}", hostname),
        Err(e) => eprintln!("✗ Hostname error: {}", e),
    }
    
    match network::get_local_ip() {
        Ok(ip) => println!("✓ Local IP: {}", ip),
        Err(e) => eprintln!("✗ Local IP error: {}", e),
    }
    
    match network::get_username() {
        Ok(username) => println!("✓ Username: {}", username),
        Err(e) => eprintln!("✗ Username error: {}", e),
    }
    
    println!("✓ OS: {}", network::get_os_info());
    println!("✓ Arch: {}", network::get_arch_info());
    
    // 测试后备方案
    println!("\nTesting fallback functions:");
    println!("✓ Hostname (fallback): {}", network::get_hostname_fallback());
    println!("✓ Local IP (fallback): {}", network::get_local_ip_fallback());
}