//! CGNAT (Carrier-Grade NAT) detection
//!
//! This module provides detection for CGNAT and symmetric NAT scenarios,
//! which prevent direct peer-to-peer connectivity.

use std::net::IpAddr;
use tracing::{debug, warn};

/// CGNAT (RFC 6598) IP range: 100.64.0.0/10
const CGNAT_RANGE_START: u32 = 0x64400000; // 100.64.0.0
const CGNAT_RANGE_END: u32 = 0x647FFFFF; // 100.127.255.255

/// Detect if an external IP address is within CGNAT range
///
/// CGNAT (Carrier-Grade NAT) uses the shared address space 100.64.0.0/10
/// as defined in RFC 6598. If your external IP is in this range, you are
/// behind CGNAT and will need relay/TURN servers for P2P connectivity.
///
/// # Arguments
///
/// * `external_ip` - The external IP address to check
///
/// # Returns
///
/// `true` if the IP is in CGNAT range, `false` otherwise
///
/// # Example
///
/// ```
/// use std::net::IpAddr;
/// use pure2p::connectivity::detect_cgnat;
///
/// let cgnat_ip: IpAddr = "100.64.0.1".parse().unwrap();
/// assert!(detect_cgnat(cgnat_ip));
///
/// let public_ip: IpAddr = "203.0.113.5".parse().unwrap();
/// assert!(!detect_cgnat(public_ip));
/// ```
pub fn detect_cgnat(external_ip: IpAddr) -> bool {
    match external_ip {
        IpAddr::V4(ipv4) => {
            let ip_u32 = u32::from(ipv4);
            let is_cgnat = ip_u32 >= CGNAT_RANGE_START && ip_u32 <= CGNAT_RANGE_END;

            if is_cgnat {
                warn!(
                    "CGNAT detected: External IP {} is in range 100.64.0.0/10. \
                     Direct P2P connectivity not possible - relay required.",
                    ipv4
                );
            } else {
                debug!("External IP {} is not in CGNAT range", ipv4);
            }

            is_cgnat
        }
        IpAddr::V6(_) => {
            // IPv6 addresses are never in CGNAT range
            debug!("IPv6 address - CGNAT not applicable");
            false
        }
    }
}

/// Check if an IP is a private/local address
///
/// This helps distinguish between local IPs and CGNAT IPs.
///
/// # Arguments
///
/// * `ip` - The IP address to check
///
/// # Returns
///
/// `true` if the IP is private (RFC 1918, loopback, link-local), `false` otherwise
pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_private() || ipv4.is_loopback() || ipv4.is_link_local() || ipv4.is_unspecified()
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback() || ipv6.is_unspecified() || is_ipv6_private(&ipv6)
        }
    }
}

/// Check if an IPv6 address is private
fn is_ipv6_private(ipv6: &std::net::Ipv6Addr) -> bool {
    let segments = ipv6.segments();
    // ULA (Unique Local Address): fc00::/7
    (segments[0] & 0xfe00) == 0xfc00 ||
    // Link-local: fe80::/10
    (segments[0] & 0xffc0) == 0xfe80
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_cgnat_in_range() {
        // Start of range
        let ip: IpAddr = "100.64.0.0".parse().unwrap();
        assert!(detect_cgnat(ip));

        // Middle of range
        let ip: IpAddr = "100.100.50.25".parse().unwrap();
        assert!(detect_cgnat(ip));

        // End of range
        let ip: IpAddr = "100.127.255.255".parse().unwrap();
        assert!(detect_cgnat(ip));
    }

    #[test]
    fn test_detect_cgnat_outside_range() {
        // Just before range
        let ip: IpAddr = "100.63.255.255".parse().unwrap();
        assert!(!detect_cgnat(ip));

        // Just after range
        let ip: IpAddr = "100.128.0.0".parse().unwrap();
        assert!(!detect_cgnat(ip));

        // Public IP
        let ip: IpAddr = "203.0.113.5".parse().unwrap();
        assert!(!detect_cgnat(ip));

        // Private IP (not CGNAT)
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        assert!(!detect_cgnat(ip));
    }

    #[test]
    fn test_detect_cgnat_ipv6() {
        // IPv6 should never be CGNAT
        let ip: IpAddr = "2001:4860:4860::8888".parse().unwrap();
        assert!(!detect_cgnat(ip));

        let ip: IpAddr = "::1".parse().unwrap();
        assert!(!detect_cgnat(ip));
    }

    #[test]
    fn test_is_private_ip_v4() {
        // RFC 1918 addresses
        assert!(is_private_ip("10.0.0.1".parse().unwrap()));
        assert!(is_private_ip("172.16.0.1".parse().unwrap()));
        assert!(is_private_ip("192.168.1.1".parse().unwrap()));

        // Loopback
        assert!(is_private_ip("127.0.0.1".parse().unwrap()));

        // Link-local
        assert!(is_private_ip("169.254.0.1".parse().unwrap()));

        // Public IP
        assert!(!is_private_ip("8.8.8.8".parse().unwrap()));

        // CGNAT is not private
        assert!(!is_private_ip("100.64.0.1".parse().unwrap()));
    }

    #[test]
    fn test_is_private_ip_v6() {
        // Loopback
        assert!(is_private_ip("::1".parse().unwrap()));

        // Link-local
        assert!(is_private_ip("fe80::1".parse().unwrap()));

        // ULA
        assert!(is_private_ip("fc00::1".parse().unwrap()));
        assert!(is_private_ip("fd00::1".parse().unwrap()));

        // Public IPv6
        assert!(!is_private_ip("2001:4860:4860::8888".parse().unwrap()));
    }
}
