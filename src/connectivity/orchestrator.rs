//! Connectivity orchestrator - unified strategy selection

use super::ipv6::check_ipv6_connectivity;
use super::natpmp::try_natpmp_mapping;
use super::pcp::try_pcp_mapping;
use super::upnp::try_upnp_mapping;
use super::types::{ConnectivityResult, StrategyAttempt};
use tracing::{debug, error, info, warn};

/// Establish connectivity using automatic protocol detection and fallback
///
/// This function tries different connectivity strategies in order:
/// 1. IPv6 direct connectivity (if available)
/// 2. PCP (Port Control Protocol - modern, efficient)
/// 3. NAT-PMP (legacy Apple/Cisco protocol)
/// 4. UPnP IGD (universal but slower)
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
///     println!("Success! External address: {}:{}",
///              mapping.external_ip, mapping.external_port);
/// } else {
///     println!("All methods failed: {}", result.summary());
/// }
/// # Ok(())
/// # }
/// ```
pub async fn establish_connectivity(port: u16) -> ConnectivityResult {
    info!(
        "Establishing connectivity for port {} (trying IPv6 → PCP → NAT-PMP → UPnP)",
        port
    );

    let mut result = ConnectivityResult::new();
    let lifetime_secs = 3600; // 1 hour default lifetime for NAT mappings

    // Strategy 1: IPv6 direct connectivity
    info!("Attempting IPv6 direct connectivity...");
    match check_ipv6_connectivity(port).await {
        Ok(mapping) => {
            info!("IPv6 connectivity successful");
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
            result.upnp = StrategyAttempt::Success(mapping.clone());
            result.mapping = Some(mapping);
            return result; // Success!
        }
        Err(e) => {
            warn!("UPnP failed: {}", e);
            result.upnp = StrategyAttempt::Failed(e.to_string());
        }
    }

    // All strategies failed
    error!(
        "All connectivity strategies failed. Summary: {}",
        result.summary()
    );
    result
}
