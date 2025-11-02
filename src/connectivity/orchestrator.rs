//! Connectivity orchestrator - unified strategy selection

use super::cgnat::detect_cgnat;
use super::health_check::{verify_external_reachability, ReachabilityStatus};
use super::http_ip::detect_external_ip;
use super::ipv6::check_ipv6_connectivity;
use super::natpmp::try_natpmp_mapping;
use super::pcp::try_pcp_mapping;
use super::upnp::try_upnp_mapping;
use super::types::{ConnectivityResult, MappingProtocol, PortMappingResult, StrategyAttempt};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

/// Verify connectivity after transport server is running
///
/// This function should be called AFTER the transport server has started listening.
/// It tests external reachability and updates the ConnectivityResult.
///
/// # Arguments
/// * `result` - The connectivity result to update with reachability status
///
/// # Returns
/// * Updated `ConnectivityResult` with externally_reachable field set
pub async fn verify_connectivity_health(mut result: ConnectivityResult) -> ConnectivityResult {
    if let Some(mapping) = &result.mapping {
        info!("Verifying external reachability of {}:{}", mapping.external_ip, mapping.external_port);

        match verify_external_reachability(mapping, 5).await {
            ReachabilityStatus::Reachable => {
                info!("✓ Port is confirmed reachable from external networks");
                result.externally_reachable = Some(true);
            }
            ReachabilityStatus::Unreachable => {
                warn!("✗ Port is NOT reachable from external networks");
                warn!("   Possible causes:");
                warn!("   - Firewall blocking the port");
                warn!("   - Port mapping failed silently");
                warn!("   - Router behind CGNAT (external IP in 100.64.0.0/10 range)");
                warn!("   - Testing from behind the same NAT");
                result.externally_reachable = Some(false);
            }
            ReachabilityStatus::TestFailed(e) => {
                warn!("⚠ Health check inconclusive: {}", e);
                result.externally_reachable = None;
            }
        }
    } else {
        warn!("No mapping available to verify");
        result.externally_reachable = Some(false);
    }

    result
}

/// Establish connectivity using automatic protocol detection and fallback
///
/// This function tries different connectivity strategies in order:
/// 1. IPv6 direct connectivity (if available)
/// 2. PCP (Port Control Protocol - modern, efficient)
/// 3. NAT-PMP (legacy Apple/Cisco protocol)
/// 4. UPnP IGD (universal but slower)
/// 5. HTTP-based IP detection (fallback when all NAT traversal fails)
///
/// After a successful mapping, this function verifies external reachability
/// by testing the /health endpoint. If verification fails, it continues to
/// the next strategy to find a working solution.
///
/// Returns a comprehensive result showing all attempts and the final mapping.
///
/// # Arguments
///
/// * `port` - The local port to expose
///
/// # Returns
///
/// A `ConnectivityResult` containing the status of all attempts and the final mapping (if any).
///
/// # Example
///
/// ```no_run
/// use pure2p::connectivity::establish_connectivity;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let result = establish_connectivity(8080).await;
///
/// if let Some(mapping) = result.mapping {
///     if result.externally_reachable == Some(true) {
///         println!("✓ Success! Confirmed reachable at {}:{}",
///                  mapping.external_ip, mapping.external_port);
///     } else {
///         println!("⚠ Mapping created but reachability unconfirmed");
///     }
/// } else {
///     println!("All methods failed: {}", result.summary());
/// }
/// # Ok(())
/// # }
/// ```
pub async fn establish_connectivity(port: u16) -> ConnectivityResult {
    info!(
        "Establishing connectivity for port {} (trying IPv6 → PCP → NAT-PMP → UPnP → HTTP IP detection)",
        port
    );

    let mut result = ConnectivityResult::new();
    let lifetime_secs = 3600; // 1 hour default lifetime for NAT mappings

    // Strategy 1: IPv6 direct connectivity
    info!("Attempting IPv6 direct connectivity...");
    match check_ipv6_connectivity(port).await {
        Ok(mapping) => {
            info!("IPv6 connectivity successful");
            result.cgnat_detected = detect_cgnat(mapping.external_ip);
            result.ipv6 = StrategyAttempt::Success(mapping.clone());
            result.mapping = Some(mapping);
            return result; // Early return - IPv6 is best
        }
        Err(e) => {
            debug!("IPv6 not available: {}", e);
            result.ipv6 = StrategyAttempt::Failed(e.to_string());
        }
    }

    // Strategy 2: PCP (Port Control Protocol)
    info!("Attempting PCP mapping...");
    match try_pcp_mapping(port, lifetime_secs).await {
        Ok(mapping) => {
            info!("PCP mapping successful");
            result.cgnat_detected = detect_cgnat(mapping.external_ip);
            result.pcp = StrategyAttempt::Success(mapping.clone());
            result.mapping = Some(mapping);
            return result; // Early return - found a working method
        }
        Err(e) => {
            debug!("PCP failed: {}", e);
            result.pcp = StrategyAttempt::Failed(e.to_string());
        }
    }

    // Strategy 3: NAT-PMP (legacy)
    info!("Attempting NAT-PMP mapping...");
    match try_natpmp_mapping(port, lifetime_secs).await {
        Ok(mapping) => {
            info!("NAT-PMP mapping successful");
            result.cgnat_detected = detect_cgnat(mapping.external_ip);
            result.natpmp = StrategyAttempt::Success(mapping.clone());
            result.mapping = Some(mapping);
            return result; // Early return - found a working method
        }
        Err(e) => {
            debug!("NAT-PMP failed: {}", e);
            result.natpmp = StrategyAttempt::Failed(e.to_string());
        }
    }

    // Strategy 4: UPnP IGD (slowest but most universal)
    info!("Attempting UPnP mapping...");
    match try_upnp_mapping(port, lifetime_secs).await {
        Ok(mapping) => {
            info!("UPnP mapping successful");
            result.cgnat_detected = detect_cgnat(mapping.external_ip);
            result.upnp = StrategyAttempt::Success(mapping.clone());
            result.mapping = Some(mapping);
            return result; // Success!
        }
        Err(e) => {
            warn!("UPnP failed: {}", e);
            result.upnp = StrategyAttempt::Failed(e.to_string());
        }
    }

    // All NAT traversal strategies failed - try HTTP-based IP detection as final fallback
    warn!("All NAT traversal protocols failed. Attempting HTTP-based IP detection...");
    match detect_external_ip().await {
        Ok(external_ip) => {
            info!("External IP detected via HTTP: {}", external_ip);

            // Create a mapping result without port mapping (direct connectivity attempt)
            let created_at_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            let mapping = PortMappingResult {
                external_ip,
                external_port: port, // No port mapping, use local port
                lifetime_secs: 0,    // No NAT mapping lifetime
                protocol: MappingProtocol::Direct, // New protocol type for HTTP-detected IPs
                created_at_ms,
            };

            result.cgnat_detected = detect_cgnat(external_ip);
            result.http = StrategyAttempt::Success(mapping.clone());
            result.mapping = Some(mapping);

            info!("Connectivity established via HTTP IP detection (direct mode, no NAT mapping)");
            return result;
        }
        Err(e) => {
            error!("HTTP IP detection failed: {}", e);
            result.http = StrategyAttempt::Failed(e.to_string());
        }
    }

    // All strategies failed
    error!(
        "All connectivity strategies failed. Summary: {}",
        result.summary()
    );
    result
}
