//! PCP (Port Control Protocol) - RFC 6887
//!
//! This module implements the Port Control Protocol (PCP) as defined in RFC 6887.
//! PCP is a modern NAT traversal protocol that provides efficient port mapping
//! with support for both IPv4 and IPv6.
//!
//! # Features
//!
//! - MAP operation for creating port mappings
//! - IPv4-mapped IPv6 address handling
//! - Comprehensive error codes with descriptive messages
//! - Automatic gateway discovery
//!
//! # Protocol Details
//!
//! PCP uses UDP port 5351 (IANA assigned) for communication with the gateway.
//! Requests are 60 bytes for MAP operations, responses can be up to 1100 bytes.
//!
//! # Example
//!
//! ```no_run
//! use pure2p::connectivity::try_pcp_mapping;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let result = try_pcp_mapping(8080, 3600).await?;
//! println!("External address: {}:{}", result.external_ip, result.external_port);
//! # Ok(())
//! # }
//! ```

use super::gateway::find_default_gateway;
use super::types::{IpProtocol, MappingError, MappingProtocol, PortMappingResult};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// PCP protocol version
pub(crate) const PCP_VERSION: u8 = 2;

/// PCP server port (IANA assigned)
const PCP_SERVER_PORT: u16 = 5351;

/// Default timeout for PCP requests
const PCP_TIMEOUT: Duration = Duration::from_secs(3);

/// PCP opcode values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(dead_code)] // Peer and Announce reserved for future use
pub(crate) enum PcpOpcode {
    Map = 1,
    Peer = 2,
    Announce = 0,
}

/// PCP result codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum PcpResultCode {
    Success = 0,
    UnsuppVersion = 1,
    NotAuthorized = 2,
    MalformedRequest = 3,
    UnsuppOpcode = 4,
    UnsuppOption = 5,
    MalformedOption = 6,
    NetworkFailure = 7,
    NoResources = 8,
    UnsuppProtocol = 9,
    UserExQuota = 10,
    CannotProvideExternal = 11,
    AddressMismatch = 12,
    ExcessiveRemotePeers = 13,
}

impl PcpResultCode {
    pub(crate) fn from_u8(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Success),
            1 => Some(Self::UnsuppVersion),
            2 => Some(Self::NotAuthorized),
            3 => Some(Self::MalformedRequest),
            4 => Some(Self::UnsuppOpcode),
            5 => Some(Self::UnsuppOption),
            6 => Some(Self::MalformedOption),
            7 => Some(Self::NetworkFailure),
            8 => Some(Self::NoResources),
            9 => Some(Self::UnsuppProtocol),
            10 => Some(Self::UserExQuota),
            11 => Some(Self::CannotProvideExternal),
            12 => Some(Self::AddressMismatch),
            13 => Some(Self::ExcessiveRemotePeers),
            _ => None,
        }
    }

    pub(crate) fn to_error_message(&self) -> &'static str {
        match self {
            Self::Success => "Success",
            Self::UnsuppVersion => "Unsupported PCP version",
            Self::NotAuthorized => "Not authorized",
            Self::MalformedRequest => "Malformed request",
            Self::UnsuppOpcode => "Unsupported opcode",
            Self::UnsuppOption => "Unsupported option",
            Self::MalformedOption => "Malformed option",
            Self::NetworkFailure => "Network failure",
            Self::NoResources => "No resources available",
            Self::UnsuppProtocol => "Unsupported protocol",
            Self::UserExQuota => "User exceeded quota",
            Self::CannotProvideExternal => "Cannot provide external port",
            Self::AddressMismatch => "Address mismatch",
            Self::ExcessiveRemotePeers => "Excessive remote peers",
        }
    }
}

/// Attempt to create a port mapping using PCP (Port Control Protocol)
///
/// This function sends a PCP MAP request to the default gateway and waits
/// for a response. If successful, it returns the external IP, port, and
/// lifetime of the mapping.
///
/// # Arguments
///
/// * `local_port` - The local port to map
/// * `lifetime_secs` - Requested lifetime in seconds (0 = delete mapping)
///
/// # Returns
///
/// Returns `Ok(PortMappingResult)` on success, or `Err(MappingError)` on failure.
///
/// # Example
///
/// ```no_run
/// use pure2p::connectivity::try_pcp_mapping;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let result = try_pcp_mapping(8080, 3600).await?;
/// println!("External address: {}:{}", result.external_ip, result.external_port);
/// # Ok(())
/// # }
/// ```
pub async fn try_pcp_mapping(
    local_port: u16,
    lifetime_secs: u32,
) -> Result<PortMappingResult, MappingError> {
    try_pcp_mapping_with_protocol(local_port, lifetime_secs, IpProtocol::TCP).await
}

/// Attempt to create a port mapping using PCP with specific IP protocol
///
/// # Arguments
///
/// * `local_port` - The local port to map
/// * `lifetime_secs` - Requested lifetime in seconds (0 = delete mapping)
/// * `protocol` - IP protocol (TCP or UDP)
pub async fn try_pcp_mapping_with_protocol(
    local_port: u16,
    lifetime_secs: u32,
    protocol: IpProtocol,
) -> Result<PortMappingResult, MappingError> {
    info!(
        "Attempting PCP mapping for port {} (lifetime: {}s, protocol: {:?})",
        local_port, lifetime_secs, protocol
    );

    // Find default gateway
    let gateway = find_default_gateway()?;
    debug!("Found default gateway: {}", gateway);

    // Create UDP socket for PCP communication
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(PCP_TIMEOUT))?;
    socket.set_write_timeout(Some(PCP_TIMEOUT))?;

    // Get local IP address from the socket
    let local_ip = socket.local_addr()?.ip();

    // Build PCP MAP request
    let request = build_pcp_map_request(local_ip, local_port, lifetime_secs, protocol);

    // Send request to gateway
    let server_addr = SocketAddr::new(gateway, PCP_SERVER_PORT);
    socket.send_to(&request, server_addr)?;
    debug!("Sent PCP MAP request to {}", server_addr);

    // Receive response
    let mut response_buf = [0u8; 1100]; // PCP response can be up to 1100 bytes
    let (bytes_received, _) = socket
        .recv_from(&mut response_buf)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                MappingError::Timeout
            } else {
                MappingError::Io(e)
            }
        })?;

    debug!("Received {} bytes from PCP server", bytes_received);

    // Parse response
    parse_pcp_map_response(&response_buf[..bytes_received], local_port)
}

/// Build a PCP MAP request packet
pub(crate) fn build_pcp_map_request(
    local_ip: IpAddr,
    internal_port: u16,
    lifetime_secs: u32,
    protocol: IpProtocol,
) -> Vec<u8> {
    let mut request = Vec::with_capacity(60); // PCP MAP request is 60 bytes

    // PCP common header (24 bytes)
    request.push(PCP_VERSION); // Version
    request.push(PcpOpcode::Map as u8); // Opcode (R=0 for request)
    request.extend_from_slice(&[0u8; 2]); // Reserved
    request.extend_from_slice(&lifetime_secs.to_be_bytes()); // Requested lifetime

    // Client IP address (16 bytes, IPv4-mapped if needed)
    match local_ip {
        IpAddr::V4(ipv4) => {
            // IPv4-mapped IPv6 format: ::ffff:a.b.c.d
            request.extend_from_slice(&[0u8; 10]); // 10 zeros
            request.extend_from_slice(&[0xff, 0xff]); // 0xffff
            request.extend_from_slice(&ipv4.octets()); // IPv4 address
        }
        IpAddr::V6(ipv6) => {
            request.extend_from_slice(&ipv6.octets()); // IPv6 address
        }
    }

    // MAP opcode-specific data (36 bytes)
    request.extend_from_slice(&[0u8; 12]); // Mapping nonce (12 bytes) - can be random for better tracking
    request.push(protocol as u8); // Protocol
    request.extend_from_slice(&[0u8; 3]); // Reserved
    request.extend_from_slice(&internal_port.to_be_bytes()); // Internal port
    request.extend_from_slice(&internal_port.to_be_bytes()); // Suggested external port (same as internal)

    // Suggested external IP (16 bytes, all zeros = no suggestion)
    request.extend_from_slice(&[0u8; 16]);

    request
}

/// Parse a PCP MAP response packet
fn parse_pcp_map_response(
    response: &[u8],
    expected_internal_port: u16,
) -> Result<PortMappingResult, MappingError> {
    if response.len() < 60 {
        return Err(MappingError::InvalidResponse(format!(
            "Response too short: {} bytes (expected at least 60)",
            response.len()
        )));
    }

    // Parse common header
    let version = response[0];
    if version != PCP_VERSION {
        warn!("PCP version mismatch: got {}, expected {}", version, PCP_VERSION);
    }

    let opcode_byte = response[1];
    let is_response = (opcode_byte & 0x80) != 0;
    let opcode = opcode_byte & 0x7f;

    if !is_response {
        return Err(MappingError::InvalidResponse(
            "Received request instead of response".to_string(),
        ));
    }

    if opcode != PcpOpcode::Map as u8 {
        return Err(MappingError::InvalidResponse(format!(
            "Wrong opcode: got {}, expected {}",
            opcode,
            PcpOpcode::Map as u8
        )));
    }

    // Parse result code
    let result_code = response[3];
    let result = PcpResultCode::from_u8(result_code)
        .ok_or_else(|| MappingError::InvalidResponse(format!("Unknown result code: {}", result_code)))?;

    if result != PcpResultCode::Success {
        return Err(MappingError::GatewayError(result.to_error_message().to_string()));
    }

    // Parse lifetime
    let lifetime_secs = u32::from_be_bytes([response[4], response[5], response[6], response[7]]);

    // Parse epoch time (not used currently, but available for time sync)
    let _epoch_time = u32::from_be_bytes([response[8], response[9], response[10], response[11]]);

    // Skip reserved bytes (12 bytes)

    // Parse MAP-specific data (starts at byte 24)
    // Mapping nonce (12 bytes) - skip for now
    // Protocol at byte 36
    let _protocol = response[36];

    // Internal port at bytes 40-41
    let internal_port = u16::from_be_bytes([response[40], response[41]]);
    if internal_port != expected_internal_port {
        warn!(
            "Internal port mismatch: expected {}, got {}",
            expected_internal_port, internal_port
        );
    }

    // Assigned external port at bytes 42-43
    let external_port = u16::from_be_bytes([response[42], response[43]]);

    // Assigned external IP address (bytes 44-59, 16 bytes)
    let external_ip = parse_pcp_ip_address(&response[44..60])?;

    let created_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let result = PortMappingResult {
        external_ip,
        external_port,
        lifetime_secs,
        protocol: MappingProtocol::PCP,
        created_at_ms,
    };

    info!(
        "PCP mapping successful: {}:{} (lifetime: {}s)",
        result.external_ip, result.external_port, result.lifetime_secs
    );

    Ok(result)
}

/// Parse a 16-byte PCP IP address (IPv6 or IPv4-mapped IPv6)
pub(crate) fn parse_pcp_ip_address(bytes: &[u8]) -> Result<IpAddr, MappingError> {
    if bytes.len() != 16 {
        return Err(MappingError::InvalidResponse(format!(
            "Invalid IP address length: {} (expected 16)",
            bytes.len()
        )));
    }

    // Check for IPv4-mapped IPv6 (::ffff:a.b.c.d)
    if bytes[0..10] == [0u8; 10] && bytes[10..12] == [0xff, 0xff] {
        // IPv4-mapped
        let ipv4 = Ipv4Addr::new(bytes[12], bytes[13], bytes[14], bytes[15]);
        Ok(IpAddr::V4(ipv4))
    } else {
        // Pure IPv6
        let ipv6 = std::net::Ipv6Addr::from([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]);
        Ok(IpAddr::V6(ipv6))
    }
}
