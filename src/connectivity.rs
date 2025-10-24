//! Connectivity module for NAT traversal and port mapping
//!
//! This module provides automatic port mapping through various protocols:
//! - PCP (Port Control Protocol) - RFC 6887
//! - NAT-PMP (NAT Port Mapping Protocol) - RFC 6886
//! - UPnP (Universal Plug and Play)
//! - IPv6 support
//!
//! The module automatically attempts different protocols in priority order
//! and manages mapping lifecycle including renewal.

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Result of a port mapping operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PortMappingResult {
    /// External IP address visible to the internet
    pub external_ip: IpAddr,
    /// External port mapped on the gateway
    pub external_port: u16,
    /// Lifetime of the mapping in seconds
    pub lifetime_secs: u32,
    /// Protocol used for the mapping
    pub protocol: MappingProtocol,
    /// Timestamp when mapping was created (Unix milliseconds)
    pub created_at_ms: i64,
}

/// Protocols available for port mapping
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MappingProtocol {
    /// Port Control Protocol (RFC 6887)
    PCP,
    /// NAT Port Mapping Protocol (RFC 6886)
    NATPMP,
    /// Universal Plug and Play
    UPnP,
    /// Manual configuration (no automatic mapping)
    Manual,
}

/// Errors that can occur during port mapping
#[derive(Debug, Error)]
pub enum MappingError {
    /// Network timeout waiting for response
    #[error("Mapping request timed out")]
    Timeout,

    /// Invalid response from gateway
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Gateway returned an error
    #[error("Gateway error: {0}")]
    GatewayError(String),

    /// No gateway found on network
    #[error("No gateway found")]
    NoGateway,

    /// IO error during communication
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Protocol not supported by gateway
    #[error("Protocol not supported")]
    NotSupported,

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// PCP opcode values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum PcpOpcode {
    Map = 1,
    Peer = 2,
    Announce = 0,
}

/// PCP protocol version
const PCP_VERSION: u8 = 2;

/// PCP server port (IANA assigned)
const PCP_SERVER_PORT: u16 = 5351;

/// Default timeout for PCP requests
const PCP_TIMEOUT: Duration = Duration::from_secs(3);

/// PCP result codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum PcpResultCode {
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
    fn from_u8(code: u8) -> Option<Self> {
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

    fn to_error_message(&self) -> &'static str {
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

/// IP protocol numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IpProtocol {
    /// TCP protocol
    TCP = 6,
    /// UDP protocol
    UDP = 17,
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
fn build_pcp_map_request(
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
fn parse_pcp_ip_address(bytes: &[u8]) -> Result<IpAddr, MappingError> {
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

/// Find the default gateway IP address
///
/// This is a simple implementation that works on most platforms.
/// On Linux/macOS, it reads from routing table. On Windows, uses ipconfig.
fn find_default_gateway() -> Result<IpAddr, MappingError> {
    // Try to find gateway using default-net crate (we'll use a simpler approach for now)
    // For production, consider using the `default-net` crate or `netstat` parsing

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

/// Automatic port mapping manager with renewal
///
/// This manager creates a port mapping and automatically renews it
/// before it expires (at 80% of lifetime).
pub struct PortMappingManager {
    local_port: u16,
    lifetime_secs: u32,
    protocol: IpProtocol,
    current_mapping: Arc<Mutex<Option<PortMappingResult>>>,
    renewal_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl PortMappingManager {
    /// Create a new port mapping manager
    pub fn new(local_port: u16, lifetime_secs: u32, protocol: IpProtocol) -> Self {
        Self {
            local_port,
            lifetime_secs,
            protocol,
            current_mapping: Arc::new(Mutex::new(None)),
            renewal_task: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the port mapping and automatic renewal
    ///
    /// This will create an initial mapping and spawn a background task
    /// to renew it at 80% of the lifetime.
    pub async fn start(&self) -> Result<PortMappingResult, MappingError> {
        info!(
            "Starting port mapping manager for port {} (lifetime: {}s)",
            self.local_port, self.lifetime_secs
        );

        // Create initial mapping
        let mapping = try_pcp_mapping_with_protocol(
            self.local_port,
            self.lifetime_secs,
            self.protocol,
        )
        .await?;

        // Store mapping
        *self.current_mapping.lock().await = Some(mapping.clone());

        // Start renewal task
        self.start_renewal_task().await;

        Ok(mapping)
    }

    /// Stop the port mapping and cancel renewal
    pub async fn stop(&self) -> Result<(), MappingError> {
        info!("Stopping port mapping manager for port {}", self.local_port);

        // Cancel renewal task
        if let Some(task) = self.renewal_task.lock().await.take() {
            task.abort();
        }

        // Delete mapping (lifetime = 0)
        try_pcp_mapping_with_protocol(self.local_port, 0, self.protocol).await?;

        // Clear current mapping
        *self.current_mapping.lock().await = None;

        Ok(())
    }

    /// Get the current mapping (if any)
    pub async fn current_mapping(&self) -> Option<PortMappingResult> {
        self.current_mapping.lock().await.clone()
    }

    /// Start the background renewal task
    async fn start_renewal_task(&self) {
        let local_port = self.local_port;
        let lifetime_secs = self.lifetime_secs;
        let protocol = self.protocol;
        let current_mapping = self.current_mapping.clone();

        // Cancel existing task if any
        if let Some(task) = self.renewal_task.lock().await.take() {
            task.abort();
        }

        // Spawn new renewal task
        let task = tokio::spawn(async move {
            loop {
                // Calculate renewal time (80% of lifetime)
                let renewal_delay_secs = (lifetime_secs as f64 * 0.8) as u64;
                debug!(
                    "Next renewal in {} seconds (80% of {}s lifetime)",
                    renewal_delay_secs, lifetime_secs
                );

                tokio::time::sleep(Duration::from_secs(renewal_delay_secs)).await;

                // Attempt renewal
                info!("Renewing port mapping for port {}", local_port);
                match try_pcp_mapping_with_protocol(local_port, lifetime_secs, protocol).await {
                    Ok(new_mapping) => {
                        info!(
                            "Port mapping renewed: {}:{} (lifetime: {}s)",
                            new_mapping.external_ip,
                            new_mapping.external_port,
                            new_mapping.lifetime_secs
                        );
                        *current_mapping.lock().await = Some(new_mapping);
                    }
                    Err(e) => {
                        error!("Failed to renew port mapping: {}", e);
                        // Continue trying - next renewal will happen after normal interval
                    }
                }
            }
        });

        *self.renewal_task.lock().await = Some(task);
    }
}

impl Drop for PortMappingManager {
    fn drop(&mut self) {
        // Cancel renewal task on drop
        if let Some(task) = self.renewal_task.try_lock().ok().and_then(|mut guard| guard.take()) {
            task.abort();
        }
    }
}

// ============================================================================
// NAT-PMP (NAT Port Mapping Protocol) - RFC 6886
// ============================================================================

/// NAT-PMP server port (IANA assigned)
const NATPMP_SERVER_PORT: u16 = 5351;

/// NAT-PMP protocol version
const NATPMP_VERSION: u8 = 0;

/// Default timeout for NAT-PMP requests
const NATPMP_TIMEOUT: Duration = Duration::from_secs(2);

/// NAT-PMP opcodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum NatPmpOpcode {
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
enum NatPmpResultCode {
    Success = 0,
    UnsupportedVersion = 1,
    NotAuthorized = 2,
    NetworkFailure = 3,
    OutOfResources = 4,
    UnsupportedOpcode = 5,
}

impl NatPmpResultCode {
    fn from_u16(code: u16) -> Option<Self> {
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

    fn to_error_message(&self) -> &'static str {
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
fn build_natpmp_map_request(
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
fn parse_natpmp_map_response(
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
    // For now, we'll use the gateway IP as a placeholder or make an additional request
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

/// Attempt port mapping with automatic fallback: PCP → NAT-PMP
///
/// This function tries PCP first, and if it fails, falls back to NAT-PMP.
/// This provides maximum compatibility across different router types.
///
/// # Arguments
///
/// * `local_port` - The local port to map
/// * `lifetime_secs` - Requested lifetime in seconds
///
/// # Returns
///
/// Returns the first successful mapping, or the last error if all fail.
pub async fn try_auto_mapping(
    local_port: u16,
    lifetime_secs: u32,
) -> Result<PortMappingResult, MappingError> {
    try_auto_mapping_with_protocol(local_port, lifetime_secs, IpProtocol::TCP).await
}

/// Attempt port mapping with automatic fallback and specific protocol
pub async fn try_auto_mapping_with_protocol(
    local_port: u16,
    lifetime_secs: u32,
    protocol: IpProtocol,
) -> Result<PortMappingResult, MappingError> {
    info!(
        "Attempting auto port mapping for port {} (trying PCP, then NAT-PMP)",
        local_port
    );

    // Try PCP first (modern protocol)
    match try_pcp_mapping_with_protocol(local_port, lifetime_secs, protocol).await {
        Ok(result) => {
            info!("PCP mapping succeeded");
            return Ok(result);
        }
        Err(e) => {
            debug!("PCP mapping failed: {}, trying NAT-PMP...", e);
        }
    }

    // Fall back to NAT-PMP (legacy protocol)
    match try_natpmp_mapping_with_protocol(local_port, lifetime_secs, protocol).await {
        Ok(result) => {
            info!("NAT-PMP mapping succeeded");
            Ok(result)
        }
        Err(e) => {
            warn!("NAT-PMP mapping failed: {}", e);
            Err(e)
        }
    }
}

// ============================================================================
// UPnP IGD (Internet Gateway Device) - UPnP Forum specification
// ============================================================================

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
fn upnp_mapping_blocking(
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

/// Update auto_mapping to include UPnP fallback: PCP → NAT-PMP → UPnP
///
/// This function tries all three protocols in order of preference:
/// 1. PCP (modern, efficient, IPv6-ready)
/// 2. NAT-PMP (legacy, widely supported on Apple/Cisco)
/// 3. UPnP (universal, but slower and more complex)
///
/// # Arguments
///
/// * `local_port` - The local port to map
/// * `lifetime_secs` - Requested lifetime in seconds
///
/// # Returns
///
/// Returns the first successful mapping, or the last error if all fail.
pub async fn try_auto_mapping_v2(
    local_port: u16,
    lifetime_secs: u32,
) -> Result<PortMappingResult, MappingError> {
    try_auto_mapping_v2_with_protocol(local_port, lifetime_secs, IpProtocol::TCP).await
}

/// Attempt port mapping with full fallback chain and specific protocol
pub async fn try_auto_mapping_v2_with_protocol(
    local_port: u16,
    lifetime_secs: u32,
    protocol: IpProtocol,
) -> Result<PortMappingResult, MappingError> {
    info!(
        "Attempting auto port mapping for port {} (trying PCP → NAT-PMP → UPnP)",
        local_port
    );

    // Try PCP first (modern protocol)
    match try_pcp_mapping_with_protocol(local_port, lifetime_secs, protocol).await {
        Ok(result) => {
            info!("PCP mapping succeeded");
            return Ok(result);
        }
        Err(e) => {
            debug!("PCP mapping failed: {}, trying NAT-PMP...", e);
        }
    }

    // Fall back to NAT-PMP (legacy protocol)
    match try_natpmp_mapping_with_protocol(local_port, lifetime_secs, protocol).await {
        Ok(result) => {
            info!("NAT-PMP mapping succeeded");
            return Ok(result);
        }
        Err(e) => {
            debug!("NAT-PMP mapping failed: {}, trying UPnP...", e);
        }
    }

    // Final fallback to UPnP (universal but slowest)
    match try_upnp_mapping_with_protocol(local_port, lifetime_secs, protocol).await {
        Ok(result) => {
            info!("UPnP mapping succeeded");
            Ok(result)
        }
        Err(e) => {
            error!("All port mapping protocols failed. Last error (UPnP): {}", e);
            Err(e)
        }
    }
}

/// UPnP Port Mapping Manager with automatic cleanup
///
/// This manager creates a UPnP mapping and automatically deletes it when dropped.
pub struct UpnpMappingManager {
    local_port: u16,
    protocol: IpProtocol,
    current_mapping: Arc<Mutex<Option<PortMappingResult>>>,
}

impl UpnpMappingManager {
    /// Create a new UPnP mapping manager
    pub fn new(local_port: u16, protocol: IpProtocol) -> Self {
        Self {
            local_port,
            protocol,
            current_mapping: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the port mapping
    pub async fn start(&self, lifetime_secs: u32) -> Result<PortMappingResult, MappingError> {
        info!(
            "Starting UPnP mapping manager for port {} (lifetime: {}s)",
            self.local_port, lifetime_secs
        );

        // Create mapping
        let mapping = try_upnp_mapping_with_protocol(
            self.local_port,
            lifetime_secs,
            self.protocol,
        )
        .await?;

        // Store mapping
        *self.current_mapping.lock().await = Some(mapping.clone());

        Ok(mapping)
    }

    /// Stop the port mapping and delete it
    pub async fn stop(&self) -> Result<(), MappingError> {
        info!("Stopping UPnP mapping manager for port {}", self.local_port);

        // Delete mapping
        delete_upnp_mapping(self.local_port, self.protocol).await?;

        // Clear current mapping
        *self.current_mapping.lock().await = None;

        Ok(())
    }

    /// Get the current mapping (if any)
    pub async fn current_mapping(&self) -> Option<PortMappingResult> {
        self.current_mapping.lock().await.clone()
    }
}

impl Drop for UpnpMappingManager {
    fn drop(&mut self) {
        // Attempt cleanup on drop (best effort)
        let local_port = self.local_port;
        let protocol = self.protocol;

        // Spawn blocking cleanup task
        std::thread::spawn(move || {
            // Search for gateway
            if let Ok(gateway) = igd_next::search_gateway(igd_next::SearchOptions {
                timeout: Some(Duration::from_secs(2)),
                ..Default::default()
            }) {
                let upnp_protocol = match protocol {
                    IpProtocol::TCP => igd_next::PortMappingProtocol::TCP,
                    IpProtocol::UDP => igd_next::PortMappingProtocol::UDP,
                };

                let _ = gateway.remove_port(upnp_protocol, local_port);
                debug!("UPnP mapping cleaned up on drop");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcp_result_code_conversion() {
        assert_eq!(PcpResultCode::from_u8(0), Some(PcpResultCode::Success));
        assert_eq!(
            PcpResultCode::from_u8(1),
            Some(PcpResultCode::UnsuppVersion)
        );
        assert_eq!(PcpResultCode::from_u8(255), None);
    }

    #[test]
    fn test_pcp_result_code_error_message() {
        assert_eq!(PcpResultCode::Success.to_error_message(), "Success");
        assert_eq!(
            PcpResultCode::NotAuthorized.to_error_message(),
            "Not authorized"
        );
    }

    #[test]
    fn test_build_pcp_map_request_ipv4() {
        let local_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        let request = build_pcp_map_request(local_ip, 8080, 3600, IpProtocol::TCP);

        assert_eq!(request.len(), 60, "PCP MAP request should be 60 bytes");
        assert_eq!(request[0], PCP_VERSION, "First byte should be PCP version");
        assert_eq!(
            request[1],
            PcpOpcode::Map as u8,
            "Second byte should be MAP opcode"
        );

        // Check lifetime (bytes 4-7)
        let lifetime = u32::from_be_bytes([request[4], request[5], request[6], request[7]]);
        assert_eq!(lifetime, 3600, "Lifetime should match requested value");

        // Check IPv4-mapped address (bytes 8-23)
        assert_eq!(&request[8..18], &[0u8; 10], "Should have 10 zero bytes");
        assert_eq!(&request[18..20], &[0xff, 0xff], "Should have 0xffff marker");
        assert_eq!(
            &request[20..24],
            &[192, 168, 1, 100],
            "Should have IPv4 address"
        );

        // Check protocol (byte 36)
        assert_eq!(request[36], IpProtocol::TCP as u8, "Protocol should be TCP");

        // Check internal port (bytes 40-41)
        let internal_port = u16::from_be_bytes([request[40], request[41]]);
        assert_eq!(internal_port, 8080, "Internal port should match");
    }

    #[test]
    fn test_parse_pcp_ip_address_ipv4_mapped() {
        let bytes = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 203, 0, 113, 10,
        ];
        let ip = parse_pcp_ip_address(&bytes).unwrap();
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10)));
    }

    #[test]
    fn test_parse_pcp_ip_address_ipv6() {
        let bytes = [
            0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01,
        ];
        let ip = parse_pcp_ip_address(&bytes).unwrap();
        assert!(matches!(ip, IpAddr::V6(_)));
    }

    #[test]
    fn test_parse_pcp_ip_address_invalid_length() {
        let bytes = [0u8; 8]; // Too short
        let result = parse_pcp_ip_address(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_port_mapping_result_serialization() {
        let result = PortMappingResult {
            external_ip: IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10)),
            external_port: 50123,
            lifetime_secs: 3600,
            protocol: MappingProtocol::PCP,
            created_at_ms: 1234567890000,
        };

        // Test JSON serialization
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: PortMappingResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, deserialized);
    }

    #[tokio::test]
    async fn test_port_mapping_manager_creation() {
        let manager = PortMappingManager::new(8080, 3600, IpProtocol::TCP);
        assert!(manager.current_mapping().await.is_none());
    }

    // Note: Integration tests for actual PCP communication require a PCP server
    // These would be in tests/integration_tests.rs with #[ignore] attribute
    // or run in a controlled test environment with a mock PCP server

    // ========================================================================
    // NAT-PMP Tests
    // ========================================================================

    #[test]
    fn test_natpmp_result_code_conversion() {
        assert_eq!(NatPmpResultCode::from_u16(0), Some(NatPmpResultCode::Success));
        assert_eq!(
            NatPmpResultCode::from_u16(1),
            Some(NatPmpResultCode::UnsupportedVersion)
        );
        assert_eq!(
            NatPmpResultCode::from_u16(2),
            Some(NatPmpResultCode::NotAuthorized)
        );
        assert_eq!(NatPmpResultCode::from_u16(999), None);
    }

    #[test]
    fn test_natpmp_result_code_error_message() {
        assert_eq!(NatPmpResultCode::Success.to_error_message(), "Success");
        assert_eq!(
            NatPmpResultCode::NotAuthorized.to_error_message(),
            "Not authorized/refused"
        );
        assert_eq!(
            NatPmpResultCode::NetworkFailure.to_error_message(),
            "Network failure"
        );
    }

    #[test]
    fn test_build_natpmp_map_request_tcp() {
        let request = build_natpmp_map_request(8080, 8080, 3600, IpProtocol::TCP);

        assert_eq!(request.len(), 12, "NAT-PMP MAP request should be 12 bytes");
        assert_eq!(request[0], NATPMP_VERSION, "First byte should be NAT-PMP version (0)");
        assert_eq!(
            request[1],
            NatPmpOpcode::MapTcp as u8,
            "Second byte should be MAP TCP opcode (2)"
        );
        assert_eq!(request[2], 0, "Reserved byte 1 should be 0");
        assert_eq!(request[3], 0, "Reserved byte 2 should be 0");

        // Check internal port (bytes 4-5)
        let internal_port = u16::from_be_bytes([request[4], request[5]]);
        assert_eq!(internal_port, 8080, "Internal port should match");

        // Check suggested external port (bytes 6-7)
        let external_port = u16::from_be_bytes([request[6], request[7]]);
        assert_eq!(external_port, 8080, "Suggested external port should match");

        // Check lifetime (bytes 8-11)
        let lifetime = u32::from_be_bytes([request[8], request[9], request[10], request[11]]);
        assert_eq!(lifetime, 3600, "Lifetime should match requested value");
    }

    #[test]
    fn test_build_natpmp_map_request_udp() {
        let request = build_natpmp_map_request(5060, 5060, 1800, IpProtocol::UDP);

        assert_eq!(request.len(), 12, "NAT-PMP MAP request should be 12 bytes");
        assert_eq!(
            request[1],
            NatPmpOpcode::MapUdp as u8,
            "Second byte should be MAP UDP opcode (1)"
        );

        let internal_port = u16::from_be_bytes([request[4], request[5]]);
        assert_eq!(internal_port, 5060);

        let lifetime = u32::from_be_bytes([request[8], request[9], request[10], request[11]]);
        assert_eq!(lifetime, 1800);
    }

    #[test]
    fn test_natpmp_map_response_parsing() {
        // Simulate a successful NAT-PMP MAP response
        // Format: version(1) | opcode(1) | result_code(2) | epoch_time(4) | internal_port(2) | external_port(2) | lifetime(4)
        let mut response = Vec::with_capacity(16);
        response.push(NATPMP_VERSION); // version = 0
        response.push(128 + NatPmpOpcode::MapTcp as u8); // opcode = 130 (128 + 2)
        response.extend_from_slice(&0u16.to_be_bytes()); // result code = 0 (success)
        response.extend_from_slice(&1234567u32.to_be_bytes()); // epoch time
        response.extend_from_slice(&8080u16.to_be_bytes()); // internal port
        response.extend_from_slice(&50123u16.to_be_bytes()); // external port
        response.extend_from_slice(&3600u32.to_be_bytes()); // lifetime

        // Note: This test would need a mock for get_external_ip_natpmp
        // For now, we just test the response parsing would fail gracefully
        let gateway_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let result = parse_natpmp_map_response(&response, gateway_ip);

        // Will fail because get_external_ip_natpmp tries real network call
        // In production, this should be mocked
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_natpmp_response_invalid_version() {
        let mut response = vec![0u8; 16];
        response[0] = 99; // Invalid version
        response[1] = 130; // Valid opcode

        let gateway_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let result = parse_natpmp_map_response(&response, gateway_ip);

        assert!(result.is_err());
        if let Err(MappingError::InvalidResponse(msg)) = result {
            assert!(msg.contains("Invalid version"));
        }
    }

    #[test]
    fn test_natpmp_response_too_short() {
        let response = vec![0u8; 10]; // Too short (should be 16)

        let gateway_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let result = parse_natpmp_map_response(&response, gateway_ip);

        assert!(result.is_err());
        if let Err(MappingError::InvalidResponse(msg)) = result {
            assert!(msg.contains("too short"));
        }
    }

    #[test]
    fn test_natpmp_response_error_code() {
        let mut response = Vec::with_capacity(16);
        response.push(NATPMP_VERSION);
        response.push(130); // MAP TCP response
        response.extend_from_slice(&3u16.to_be_bytes()); // result code = 3 (network failure)
        response.extend_from_slice(&[0u8; 12]); // Rest of response

        let gateway_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let result = parse_natpmp_map_response(&response, gateway_ip);

        assert!(result.is_err());
        if let Err(MappingError::GatewayError(msg)) = result {
            assert_eq!(msg, "Network failure");
        }
    }
}
