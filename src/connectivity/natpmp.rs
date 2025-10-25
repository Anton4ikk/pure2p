//! NAT-PMP (NAT Port Mapping Protocol) implementation - RFC 6886
//!
//! NAT-PMP is a legacy protocol supported by older routers, particularly Apple AirPort
//! devices and some Cisco routers. It provides a simple mechanism for creating port
//! mappings on NAT gateways.
//!
//! # Protocol Overview
//!
//! NAT-PMP uses UDP on port 5351 to communicate with the gateway. It supports:
//! - External IP address requests
//! - UDP port mappings
//! - TCP port mappings
//!
//! # Example
//!
//! ```no_run
//! use pure2p::connectivity::try_natpmp_mapping;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let result = try_natpmp_mapping(8080, 3600).await?;
//! println!("External address: {}:{}", result.external_ip, result.external_port);
//! # Ok(())
//! # }
//! ```

use super::gateway::find_default_gateway;
use super::types::{IpProtocol, MappingError, MappingProtocol, PortMappingResult};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

/// NAT-PMP server port (IANA assigned)
const NATPMP_SERVER_PORT: u16 = 5351;

/// NAT-PMP protocol version
pub(crate) const NATPMP_VERSION: u8 = 0;

/// Default timeout for NAT-PMP requests
const NATPMP_TIMEOUT: Duration = Duration::from_secs(2);

/// NAT-PMP opcodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(dead_code)] // ExternalAddress used internally
pub(crate) enum NatPmpOpcode {
    /// External address request
    ExternalAddress = 0,
    /// UDP port mapping
    MapUdp = 1,
    /// TCP port mapping
    MapTcp = 2,
}

/// NAT-PMP result codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub(crate) enum NatPmpResultCode {
    Success = 0,
    UnsupportedVersion = 1,
    NotAuthorized = 2,
    NetworkFailure = 3,
    OutOfResources = 4,
    UnsupportedOpcode = 5,
}

impl NatPmpResultCode {
    pub(crate) fn from_u16(code: u16) -> Option<Self> {
        match code {
            0 => Some(Self::Success),
            1 => Some(Self::UnsupportedVersion),
            2 => Some(Self::NotAuthorized),
            3 => Some(Self::NetworkFailure),
            4 => Some(Self::OutOfResources),
            5 => Some(Self::UnsupportedOpcode),
            _ => None,
        }
    }

    pub(crate) fn to_error_message(&self) -> &'static str {
        match self {
            Self::Success => "Success",
            Self::UnsupportedVersion => "Unsupported NAT-PMP version",
            Self::NotAuthorized => "Not authorized/refused",
            Self::NetworkFailure => "Network failure",
            Self::OutOfResources => "Out of resources",
            Self::UnsupportedOpcode => "Unsupported opcode",
        }
    }
}

/// Attempt to create a port mapping using NAT-PMP (NAT Port Mapping Protocol)
///
/// NAT-PMP is a legacy protocol (RFC 6886) supported by older routers,
/// particularly Apple AirPort devices and some Cisco routers.
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
/// use pure2p::connectivity::try_natpmp_mapping;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let result = try_natpmp_mapping(8080, 3600).await?;
/// println!("External address: {}:{}", result.external_ip, result.external_port);
/// # Ok(())
/// # }
/// ```
pub async fn try_natpmp_mapping(
    local_port: u16,
    lifetime_secs: u32,
) -> Result<PortMappingResult, MappingError> {
    try_natpmp_mapping_with_protocol(local_port, lifetime_secs, IpProtocol::TCP).await
}

/// Attempt to create a port mapping using NAT-PMP with specific IP protocol
///
/// # Arguments
///
/// * `local_port` - The local port to map
/// * `lifetime_secs` - Requested lifetime in seconds (0 = delete mapping)
/// * `protocol` - IP protocol (TCP or UDP)
pub async fn try_natpmp_mapping_with_protocol(
    local_port: u16,
    lifetime_secs: u32,
    protocol: IpProtocol,
) -> Result<PortMappingResult, MappingError> {
    info!(
        "Attempting NAT-PMP mapping for port {} (lifetime: {}s, protocol: {:?})",
        local_port, lifetime_secs, protocol
    );

    // Find default gateway
    let gateway = find_default_gateway()?;
    debug!("Found default gateway: {}", gateway);

    // Create UDP socket for NAT-PMP communication
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(NATPMP_TIMEOUT))?;
    socket.set_write_timeout(Some(NATPMP_TIMEOUT))?;

    // Build NAT-PMP MAP request
    let request = build_natpmp_map_request(local_port, local_port, lifetime_secs, protocol);

    // Send request to gateway
    let server_addr = SocketAddr::new(gateway, NATPMP_SERVER_PORT);
    socket.send_to(&request, server_addr)?;
    debug!("Sent NAT-PMP MAP request to {}", server_addr);

    // Receive response
    let mut response_buf = [0u8; 16]; // NAT-PMP response is 16 bytes
    let (bytes_received, _) = socket
        .recv_from(&mut response_buf)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                MappingError::Timeout
            } else {
                MappingError::Io(e)
            }
        })?;

    debug!("Received {} bytes from NAT-PMP server", bytes_received);

    // Parse response
    parse_natpmp_map_response(&response_buf[..bytes_received], gateway)
}

/// Build a NAT-PMP MAP request packet
pub(crate) fn build_natpmp_map_request(
    internal_port: u16,
    suggested_external_port: u16,
    lifetime_secs: u32,
    protocol: IpProtocol,
) -> Vec<u8> {
    let mut request = Vec::with_capacity(12); // NAT-PMP MAP request is 12 bytes

    // Version (1 byte)
    request.push(NATPMP_VERSION);

    // Opcode (1 byte) - 1 for UDP, 2 for TCP
    let opcode = match protocol {
        IpProtocol::UDP => NatPmpOpcode::MapUdp,
        IpProtocol::TCP => NatPmpOpcode::MapTcp,
    };
    request.push(opcode as u8);

    // Reserved (2 bytes, must be zero)
    request.extend_from_slice(&[0u8; 2]);

    // Internal port (2 bytes, big-endian)
    request.extend_from_slice(&internal_port.to_be_bytes());

    // Suggested external port (2 bytes, big-endian)
    request.extend_from_slice(&suggested_external_port.to_be_bytes());

    // Requested lifetime (4 bytes, big-endian)
    request.extend_from_slice(&lifetime_secs.to_be_bytes());

    request
}

/// Parse a NAT-PMP MAP response packet
pub(crate) fn parse_natpmp_map_response(
    response: &[u8],
    gateway_ip: IpAddr,
) -> Result<PortMappingResult, MappingError> {
    if response.len() < 16 {
        return Err(MappingError::InvalidResponse(format!(
            "Response too short: {} bytes (expected 16)",
            response.len()
        )));
    }

    // Parse version
    let version = response[0];
    if version != NATPMP_VERSION {
        return Err(MappingError::InvalidResponse(format!(
            "Invalid version: {} (expected {})",
            version, NATPMP_VERSION
        )));
    }

    // Parse opcode (should be 128 + request opcode for response)
    let opcode = response[1];
    if opcode < 128 {
        return Err(MappingError::InvalidResponse(
            "Received request instead of response".to_string(),
        ));
    }

    // Parse result code (bytes 2-3, big-endian)
    let result_code = u16::from_be_bytes([response[2], response[3]]);
    let result = NatPmpResultCode::from_u16(result_code).ok_or_else(|| {
        MappingError::InvalidResponse(format!("Unknown result code: {}", result_code))
    })?;

    if result != NatPmpResultCode::Success {
        return Err(MappingError::GatewayError(
            result.to_error_message().to_string(),
        ));
    }

    // Parse seconds since epoch (bytes 4-7, big-endian)
    let _epoch_secs = u32::from_be_bytes([response[4], response[5], response[6], response[7]]);

    // Parse internal port (bytes 8-9, big-endian)
    let _internal_port = u16::from_be_bytes([response[8], response[9]]);

    // Parse external port (bytes 10-11, big-endian)
    let external_port = u16::from_be_bytes([response[10], response[11]]);

    // Parse lifetime (bytes 12-15, big-endian)
    let lifetime_secs = u32::from_be_bytes([response[12], response[13], response[14], response[15]]);

    // NAT-PMP doesn't return external IP in the MAP response
    // We need to make a separate request for external address
    let external_ip = get_external_ip_natpmp(gateway_ip)?;

    let created_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let result = PortMappingResult {
        external_ip,
        external_port,
        lifetime_secs,
        protocol: MappingProtocol::NATPMP,
        created_at_ms,
    };

    info!(
        "NAT-PMP mapping successful: {}:{} (lifetime: {}s)",
        result.external_ip, result.external_port, result.lifetime_secs
    );

    Ok(result)
}

/// Get external IP address using NAT-PMP
fn get_external_ip_natpmp(gateway: IpAddr) -> Result<IpAddr, MappingError> {
    // Create UDP socket
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(NATPMP_TIMEOUT))?;
    socket.set_write_timeout(Some(NATPMP_TIMEOUT))?;

    // Build external address request (2 bytes)
    let request = vec![NATPMP_VERSION, NatPmpOpcode::ExternalAddress as u8];

    // Send request
    let server_addr = SocketAddr::new(gateway, NATPMP_SERVER_PORT);
    socket.send_to(&request, server_addr)?;

    // Receive response
    let mut response_buf = [0u8; 12]; // External address response is 12 bytes
    let (bytes_received, _) = socket.recv_from(&mut response_buf).map_err(|e| {
        if e.kind() == std::io::ErrorKind::WouldBlock {
            MappingError::Timeout
        } else {
            MappingError::Io(e)
        }
    })?;

    if bytes_received < 12 {
        return Err(MappingError::InvalidResponse(format!(
            "External IP response too short: {} bytes",
            bytes_received
        )));
    }

    // Parse response
    let version = response_buf[0];
    let opcode = response_buf[1];
    let result_code = u16::from_be_bytes([response_buf[2], response_buf[3]]);

    if version != NATPMP_VERSION {
        return Err(MappingError::InvalidResponse(format!(
            "Invalid version in external IP response: {}",
            version
        )));
    }

    if opcode != (128 + NatPmpOpcode::ExternalAddress as u8) {
        return Err(MappingError::InvalidResponse(format!(
            "Invalid opcode in external IP response: {}",
            opcode
        )));
    }

    let result = NatPmpResultCode::from_u16(result_code).ok_or_else(|| {
        MappingError::InvalidResponse(format!("Unknown result code: {}", result_code))
    })?;

    if result != NatPmpResultCode::Success {
        return Err(MappingError::GatewayError(
            result.to_error_message().to_string(),
        ));
    }

    // Parse external IP (bytes 8-11)
    let external_ip = Ipv4Addr::new(
        response_buf[8],
        response_buf[9],
        response_buf[10],
        response_buf[11],
    );

    Ok(IpAddr::V4(external_ip))
}
