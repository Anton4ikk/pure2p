//! Gateway discovery for different platforms

use crate::connectivity::types::MappingError;
use std::net::{IpAddr, Ipv4Addr};

/// Find the default gateway IP address
///
/// This is a simple implementation that works on most platforms.
/// On Linux/macOS, it reads from routing table. On Windows, uses ipconfig.
pub fn find_default_gateway() -> Result<IpAddr, MappingError> {
    #[cfg(target_os = "linux")]
    {
        find_gateway_linux()
    }

    #[cfg(target_os = "macos")]
    {
        find_gateway_macos()
    }

    #[cfg(target_os = "windows")]
    {
        find_gateway_windows()
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(MappingError::NotSupported)
    }
}

#[cfg(target_os = "linux")]
fn find_gateway_linux() -> Result<IpAddr, MappingError> {
    use std::fs;

    // Read /proc/net/route
    let route_table = fs::read_to_string("/proc/net/route")
        .map_err(|e| MappingError::Internal(format!("Failed to read route table: {}", e)))?;

    for line in route_table.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 3 {
            continue;
        }

        // Check if this is the default route (destination = 00000000)
        if fields[1] == "00000000" {
            // Gateway is in field 2, in hex little-endian format
            let gateway_hex = fields[2];
            if let Ok(gateway_u32) = u32::from_str_radix(gateway_hex, 16) {
                // Convert from little-endian
                let ip = Ipv4Addr::from(gateway_u32.to_be());
                return Ok(IpAddr::V4(ip));
            }
        }
    }

    Err(MappingError::NoGateway)
}

#[cfg(target_os = "macos")]
fn find_gateway_macos() -> Result<IpAddr, MappingError> {
    use std::process::Command;

    // Use netstat command
    let output = Command::new("netstat")
        .args(&["-rn", "-f", "inet"])
        .output()
        .map_err(|e| MappingError::Internal(format!("Failed to run netstat: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if line.starts_with("default") {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 2 {
                if let Ok(ip) = fields[1].parse::<Ipv4Addr>() {
                    return Ok(IpAddr::V4(ip));
                }
            }
        }
    }

    Err(MappingError::NoGateway)
}

#[cfg(target_os = "windows")]
fn find_gateway_windows() -> Result<IpAddr, MappingError> {
    use std::process::Command;

    // Use route print command
    let output = Command::new("route")
        .args(&["print", "0.0.0.0"])
        .output()
        .map_err(|e| MappingError::Internal(format!("Failed to run route: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("0.0.0.0") {
            let fields: Vec<&str> = trimmed.split_whitespace().collect();
            if fields.len() >= 3 {
                if let Ok(ip) = fields[2].parse::<Ipv4Addr>() {
                    return Ok(IpAddr::V4(ip));
                }
            }
        }
    }

    Err(MappingError::NoGateway)
}
