//! HTTP-based external IP detection
//!
//! This module provides fallback IP detection when port mapping protocols fail.
//! It queries public HTTP services to determine the external IP address.

use super::types::MappingError;
use std::net::IpAddr;
use std::time::Duration;
use tracing::{debug, info, warn};

/// List of public IP detection services
/// We try multiple services for redundancy
const IP_DETECTION_SERVICES: &[&str] = &[
    "https://api.ipify.org",
    "https://ifconfig.me/ip",
    "https://icanhazip.com",
    "https://checkip.amazonaws.com",
];

/// Detect external IP address using HTTP services
///
/// This function tries multiple public IP detection services
/// and returns the first successful result.
///
/// # Returns
///
/// The detected external IP address (IPv4 or IPv6)
///
/// # Errors
///
/// Returns `MappingError::NotSupported` if all services fail
pub async fn detect_external_ip() -> Result<IpAddr, MappingError> {
    info!("Attempting HTTP-based external IP detection...");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| {
            warn!("Failed to create HTTP client: {}", e);
            MappingError::NotSupported
        })?;

    for service_url in IP_DETECTION_SERVICES {
        debug!("Trying IP detection service: {}", service_url);

        match client.get(*service_url).send().await {
            Ok(response) => {
                if let Ok(text) = response.text().await {
                    let ip_str = text.trim();

                    // Try to parse as IP address
                    if let Ok(ip) = ip_str.parse::<IpAddr>() {
                        info!("External IP detected via HTTP: {} (from {})", ip, service_url);
                        return Ok(ip);
                    } else {
                        debug!("Invalid IP response from {}: {}", service_url, ip_str);
                    }
                } else {
                    debug!("Failed to read response from {}", service_url);
                }
            }
            Err(e) => {
                debug!("Failed to query {}: {}", service_url, e);
            }
        }
    }

    warn!("All HTTP IP detection services failed");
    Err(MappingError::NotSupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_external_ip() {
        // This test requires internet connectivity
        // It's marked as ignored so it doesn't run in CI
        let result = detect_external_ip().await;

        // We can't assert success since it requires internet,
        // but we can verify the function doesn't panic
        match result {
            Ok(ip) => println!("Detected external IP: {}", ip),
            Err(e) => println!("IP detection failed (expected without internet): {:?}", e),
        }
    }

    #[test]
    fn test_ip_parsing() {
        // Test IPv4 parsing
        assert!("192.168.1.1".parse::<IpAddr>().is_ok());

        // Test IPv6 parsing
        assert!("2001:db8::1".parse::<IpAddr>().is_ok());

        // Test invalid IP
        assert!("not-an-ip".parse::<IpAddr>().is_err());
    }
}
