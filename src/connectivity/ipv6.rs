//! IPv6 connectivity detection and helpers

use super::types::{MappingError, MappingProtocol, PortMappingResult};
use std::net::{IpAddr, Ipv6Addr, SocketAddrV6};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

/// Check if IPv6 connectivity is available
///
/// This function checks if the system has a global IPv6 address
/// and can potentially receive direct connections without NAT.
pub(crate) async fn check_ipv6_connectivity(port: u16) -> Result<PortMappingResult, MappingError> {
    info!("Checking IPv6 connectivity on port {}", port);

    // Try to bind to IPv6 address to verify we can listen
    let ipv6_any = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port, 0, 0);

    tokio::net::TcpListener::bind(ipv6_any)
        .await
        .map_err(|e| {
            debug!("IPv6 bind failed: {}", e);
            MappingError::NotSupported
        })?;

    // Get the actual IPv6 address (not just ::)
    // For now, we'll use a simple heuristic: check if we have any global IPv6 address
    let global_ipv6 = get_global_ipv6_address()?;

    let created_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    info!("IPv6 connectivity available: {}", global_ipv6);

    Ok(PortMappingResult {
        external_ip: IpAddr::V6(global_ipv6),
        external_port: port,
        lifetime_secs: 0, // IPv6 doesn't need lifetime (no NAT)
        protocol: MappingProtocol::IPv6,
        created_at_ms,
    })
}

/// Get a global IPv6 address for this machine
///
/// Returns the first global (non-link-local, non-loopback) IPv6 address found.
fn get_global_ipv6_address() -> Result<Ipv6Addr, MappingError> {
    // Try to get local addresses by connecting to a public IPv6 address
    let socket = std::net::UdpSocket::bind("[::]:0")
        .map_err(|_| MappingError::NotSupported)?;

    // Try to connect to Google's IPv6 DNS (doesn't actually send data)
    socket
        .connect("[2001:4860:4860::8888]:80")
        .map_err(|_| MappingError::NotSupported)?;

    let local_addr = socket
        .local_addr()
        .map_err(|_| MappingError::NotSupported)?;

    match local_addr.ip() {
        IpAddr::V6(ipv6) => {
            // Check if it's a global address (not link-local, not loopback)
            if ipv6.is_loopback() || is_ipv6_link_local(&ipv6) {
                return Err(MappingError::NotSupported);
            }
            Ok(ipv6)
        }
        IpAddr::V4(_) => Err(MappingError::NotSupported),
    }
}

/// Check if an IPv6 address is link-local (fe80::/10)
pub(crate) fn is_ipv6_link_local(addr: &Ipv6Addr) -> bool {
    let segments = addr.segments();
    (segments[0] & 0xffc0) == 0xfe80
}
