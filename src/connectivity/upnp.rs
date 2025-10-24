//! UPnP IGD (Internet Gateway Device) port mapping implementation
//!
//! This module provides port mapping through UPnP (Universal Plug and Play),
//! which uses SSDP (Simple Service Discovery Protocol) to discover IGD devices
//! on the local network, then uses SOAP to communicate with the gateway.
//!
//! UPnP is widely supported across different router manufacturers and is often
//! the fallback option when modern protocols (PCP, NAT-PMP) are not available.

use super::types::{IpProtocol, MappingError, MappingProtocol, PortMappingResult};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Default timeout for UPnP operations
const UPNP_TIMEOUT: Duration = Duration::from_secs(5);

/// Get local IP address for UPnP gateway communication
fn get_local_ip_for_gateway() -> Result<Ipv4Addr, MappingError> {
    // Try to connect to a public IP to determine our local address
    // We don't actually send data, just use the socket to get the local address
    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| MappingError::Internal(format!("Failed to create socket: {}", e)))?;

    socket
        .connect("8.8.8.8:80")
        .map_err(|e| MappingError::Internal(format!("Failed to connect: {}", e)))?;

    let local_addr = socket
        .local_addr()
        .map_err(|e| MappingError::Internal(format!("Failed to get local address: {}", e)))?;

    match local_addr.ip() {
        IpAddr::V4(ipv4) => Ok(ipv4),
        IpAddr::V6(_) => Err(MappingError::Internal(
            "UPnP requires IPv4 address".to_string(),
        )),
    }
}

/// Attempt to create a port mapping using UPnP IGD (Internet Gateway Device)
///
/// UPnP uses SSDP (Simple Service Discovery Protocol) to discover IGD devices
/// on the local network, then uses SOAP to communicate with the gateway.
///
/// # Arguments
///
/// * `local_port` - The local port to map
/// * `lifetime_secs` - Requested lifetime in seconds (0 = permanent until reboot)
///
/// # Returns
///
/// Returns `Ok(PortMappingResult)` on success, or `Err(MappingError)` on failure.
///
/// # Example
///
/// ```no_run
/// use pure2p::connectivity::try_upnp_mapping;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let result = try_upnp_mapping(8080, 3600).await?;
/// println!("External address: {}:{}", result.external_ip, result.external_port);
/// # Ok(())
/// # }
/// ```
pub async fn try_upnp_mapping(
    local_port: u16,
    lifetime_secs: u32,
) -> Result<PortMappingResult, MappingError> {
    try_upnp_mapping_with_protocol(local_port, lifetime_secs, IpProtocol::TCP).await
}

/// Attempt to create a port mapping using UPnP with specific IP protocol
///
/// # Arguments
///
/// * `local_port` - The local port to map
/// * `lifetime_secs` - Requested lifetime in seconds (0 = permanent)
/// * `protocol` - IP protocol (TCP or UDP)
pub async fn try_upnp_mapping_with_protocol(
    local_port: u16,
    lifetime_secs: u32,
    protocol: IpProtocol,
) -> Result<PortMappingResult, MappingError> {
    info!(
        "Attempting UPnP mapping for port {} (lifetime: {}s, protocol: {:?})",
        local_port, lifetime_secs, protocol
    );

    // Spawn blocking task for UPnP operations (uses blocking I/O)
    let result = tokio::task::spawn_blocking(move || {
        upnp_mapping_blocking(local_port, lifetime_secs, protocol)
    })
    .await
    .map_err(|e| MappingError::Internal(format!("Task join error: {}", e)))??;

    Ok(result)
}

/// Blocking UPnP mapping implementation
pub(crate) fn upnp_mapping_blocking(
    local_port: u16,
    lifetime_secs: u32,
    protocol: IpProtocol,
) -> Result<PortMappingResult, MappingError> {
    // Search for IGD gateway with timeout
    debug!("Searching for UPnP IGD gateway...");
    let gateway = igd_next::search_gateway(igd_next::SearchOptions {
        timeout: Some(UPNP_TIMEOUT),
        ..Default::default()
    })
    .map_err(|e| {
        debug!("UPnP gateway search failed: {}", e);
        MappingError::NoGateway
    })?;

    debug!("Found UPnP gateway");

    // Convert protocol
    let upnp_protocol = match protocol {
        IpProtocol::TCP => igd_next::PortMappingProtocol::TCP,
        IpProtocol::UDP => igd_next::PortMappingProtocol::UDP,
    };

    // Add port mapping
    // Description format: "Pure2P-{protocol}-{port}"
    let description = format!("Pure2P-{:?}-{}", protocol, local_port);

    // Get local IP - we need to determine our local address
    // The igd-next library will use the socket's local address
    let local_ipv4 = get_local_ip_for_gateway()?;
    let local_addr = SocketAddr::new(IpAddr::V4(local_ipv4), local_port);

    debug!(
        "Adding port mapping: {} -> {} ({}s)",
        local_addr, local_port, lifetime_secs
    );

    // add_port signature: (protocol, external_port, local_addr, lease_duration, description)
    gateway
        .add_port(
            upnp_protocol,
            local_port,          // external port (requested)
            local_addr,          // local socket address (IP + port)
            lifetime_secs,       // lease duration in seconds
            &description,        // description
        )
        .map_err(|e| {
            warn!("UPnP AddPortMapping failed: {}", e);
            MappingError::GatewayError(format!("AddPortMapping failed: {}", e))
        })?;

    // Get external IP address
    let external_ip = gateway
        .get_external_ip()
        .map_err(|e| {
            // Clean up the mapping if we can't get external IP
            let _ = gateway.remove_port(upnp_protocol, local_port);
            MappingError::GatewayError(format!("GetExternalIPAddress failed: {}", e))
        })?;

    let created_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let result = PortMappingResult {
        external_ip,
        external_port: local_port, // UPnP typically returns same port
        lifetime_secs,
        protocol: MappingProtocol::UPnP,
        created_at_ms,
    };

    info!(
        "UPnP mapping successful: {}:{} (lifetime: {}s)",
        result.external_ip, result.external_port, result.lifetime_secs
    );

    Ok(result)
}

/// Delete a UPnP port mapping
///
/// This should be called when the application shuts down to clean up port mappings.
pub async fn delete_upnp_mapping(
    local_port: u16,
    protocol: IpProtocol,
) -> Result<(), MappingError> {
    info!(
        "Deleting UPnP mapping for port {} (protocol: {:?})",
        local_port, protocol
    );

    tokio::task::spawn_blocking(move || {
        // Search for gateway
        let gateway = igd_next::search_gateway(igd_next::SearchOptions {
            timeout: Some(UPNP_TIMEOUT),
            ..Default::default()
        })
        .map_err(|_| MappingError::NoGateway)?;

        // Convert protocol
        let upnp_protocol = match protocol {
            IpProtocol::TCP => igd_next::PortMappingProtocol::TCP,
            IpProtocol::UDP => igd_next::PortMappingProtocol::UDP,
        };

        // Remove port mapping
        gateway
            .remove_port(upnp_protocol, local_port)
            .map_err(|e| {
                MappingError::GatewayError(format!("DeletePortMapping failed: {}", e))
            })?;

        info!("UPnP mapping deleted successfully");
        Ok(())
    })
    .await
    .map_err(|e| MappingError::Internal(format!("Task join error: {}", e)))?
}
