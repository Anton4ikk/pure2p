//! External connectivity health check
//!
//! This module provides functionality to verify that a mapped port is actually
//! reachable from the external internet, not just that the port mapping was
//! created successfully.

use crate::connectivity::types::PortMappingResult;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Result of external reachability test
#[derive(Debug, Clone, PartialEq)]
pub enum ReachabilityStatus {
    /// Port is reachable from external networks
    Reachable,
    /// Port is not reachable (firewall, mapping failed, etc.)
    Unreachable,
    /// Test could not be completed (network error, service unavailable)
    TestFailed(String),
}

/// Verify that an external address is actually reachable
///
/// This function tests whether the mapped port is accessible from the public
/// internet by making an HTTP GET request to the /health endpoint.
///
/// # Arguments
/// * `mapping` - The port mapping result to verify
/// * `timeout_secs` - Timeout in seconds for the health check
///
/// # Returns
/// * `ReachabilityStatus` - Whether the port is actually reachable
///
/// # Example
/// ```rust,no_run
/// use pure2p::connectivity::{verify_external_reachability, PortMappingResult, MappingProtocol};
/// use chrono::Utc;
///
/// # async fn example() {
/// let mapping = PortMappingResult {
///     external_ip: "203.0.113.1".parse().unwrap(),
///     external_port: 8080,
///     lifetime_secs: 3600,
///     protocol: MappingProtocol::UPnP,
///     created_at_ms: Utc::now().timestamp_millis(),
/// };
///
/// match verify_external_reachability(&mapping, 5).await {
///     pure2p::connectivity::ReachabilityStatus::Reachable => {
///         println!("✓ Port is reachable from internet");
///     }
///     pure2p::connectivity::ReachabilityStatus::Unreachable => {
///         println!("✗ Port is NOT reachable (check firewall/router)");
///     }
///     pure2p::connectivity::ReachabilityStatus::TestFailed(e) => {
///         println!("⚠ Health check failed: {}", e);
///     }
/// }
/// # }
/// ```
pub async fn verify_external_reachability(
    mapping: &PortMappingResult,
    timeout_secs: u64,
) -> ReachabilityStatus {
    let url = format!("http://{}:{}/health", mapping.external_ip, mapping.external_port);

    info!("Testing external reachability: {}", url);

    // Create HTTP client with timeout
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create HTTP client for health check: {}", e);
            return ReachabilityStatus::TestFailed(format!("Client creation failed: {}", e));
        }
    };

    // Try to fetch the health endpoint
    match client.get(&url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                // Check if response body is "ok"
                match response.text().await {
                    Ok(body) => {
                        if body.trim() == "ok" {
                            info!("✓ External reachability confirmed: {}", url);
                            ReachabilityStatus::Reachable
                        } else {
                            warn!(
                                "Health endpoint returned unexpected body: {}",
                                body
                            );
                            ReachabilityStatus::Unreachable
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read health response body: {}", e);
                        ReachabilityStatus::TestFailed(format!("Body read failed: {}", e))
                    }
                }
            } else {
                warn!(
                    "Health endpoint returned non-success status: {}",
                    response.status()
                );
                ReachabilityStatus::Unreachable
            }
        }
        Err(e) => {
            // Check error type to distinguish between unreachable vs test failure
            if e.is_timeout() {
                debug!("Health check timed out (port likely unreachable): {}", e);
                ReachabilityStatus::Unreachable
            } else if e.is_connect() {
                debug!("Connection failed (port unreachable or firewalled): {}", e);
                ReachabilityStatus::Unreachable
            } else {
                error!("Health check request failed: {}", e);
                ReachabilityStatus::TestFailed(format!("Request failed: {}", e))
            }
        }
    }
}

/// Verify external reachability using a third-party port checker service
///
/// This is a fallback method when we can't directly test our own endpoint
/// (e.g., when testing from behind the same NAT).
///
/// # Arguments
/// * `mapping` - The port mapping result to verify
///
/// # Returns
/// * `ReachabilityStatus` - Whether the port is reachable according to external service
///
/// Note: This function uses public port checking services and may be rate-limited.
/// Use `verify_external_reachability` as the primary method.
pub async fn verify_via_port_checker(
    _mapping: &PortMappingResult,
) -> ReachabilityStatus {
    // Use a public port checker API (example: tcp-ping.com, portquiz.net, etc.)
    // This is a fallback for when direct health check doesn't work

    warn!("Third-party port checker not yet implemented");
    ReachabilityStatus::TestFailed("Not implemented".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectivity::types::MappingProtocol;
    use chrono::Utc;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_verify_unreachable_address() {
        // Test with an unreachable address
        let mapping = PortMappingResult {
            external_ip: Ipv4Addr::new(198, 51, 100, 1).into(), // TEST-NET-2
            external_port: 9999,
            lifetime_secs: 3600,
            protocol: MappingProtocol::UPnP,
            created_at_ms: Utc::now().timestamp_millis(),
        };

        let result = verify_external_reachability(&mapping, 2).await;

        // Should be unreachable or test failed (depending on network)
        assert!(
            matches!(result, ReachabilityStatus::Unreachable | ReachabilityStatus::TestFailed(_)),
            "Expected unreachable or test failed, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_verify_invalid_port() {
        // Test with localhost and invalid port
        let mapping = PortMappingResult {
            external_ip: Ipv4Addr::new(127, 0, 0, 1).into(),
            external_port: 1, // Port 1 unlikely to have our health endpoint
            lifetime_secs: 3600,
            protocol: MappingProtocol::Direct,
            created_at_ms: Utc::now().timestamp_millis(),
        };

        let result = verify_external_reachability(&mapping, 2).await;

        // Should be unreachable or test failed
        assert!(
            matches!(result, ReachabilityStatus::Unreachable | ReachabilityStatus::TestFailed(_)),
            "Expected unreachable or test failed, got: {:?}",
            result
        );
    }
}
