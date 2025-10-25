//! Common types for connectivity module

use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use thiserror::Error;

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
    /// Direct IPv6 connectivity (no NAT)
    IPv6,
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

/// Result of attempting a specific connectivity strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StrategyAttempt {
    /// Strategy was not attempted
    NotAttempted,
    /// Strategy succeeded
    Success(PortMappingResult),
    /// Strategy failed with error message
    Failed(String),
}

/// Complete result of connectivity orchestration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectivityResult {
    /// IPv6 attempt result
    pub ipv6: StrategyAttempt,
    /// PCP attempt result
    pub pcp: StrategyAttempt,
    /// NAT-PMP attempt result
    pub natpmp: StrategyAttempt,
    /// UPnP attempt result
    pub upnp: StrategyAttempt,
    /// Final successful mapping (if any)
    pub mapping: Option<PortMappingResult>,
    /// Whether CGNAT was detected (external IP in 100.64.0.0/10 range)
    pub cgnat_detected: bool,
}

impl ConnectivityResult {
    /// Create a new empty connectivity result
    pub fn new() -> Self {
        Self {
            ipv6: StrategyAttempt::NotAttempted,
            pcp: StrategyAttempt::NotAttempted,
            natpmp: StrategyAttempt::NotAttempted,
            upnp: StrategyAttempt::NotAttempted,
            mapping: None,
            cgnat_detected: false,
        }
    }

    /// Check if any strategy succeeded
    pub fn is_success(&self) -> bool {
        self.mapping.is_some()
    }

    /// Get a summary string of all attempts (for UX display)
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        // Add CGNAT warning first if detected
        if self.cgnat_detected {
            parts.push("⚠️  CGNAT".to_string());
        }

        match &self.ipv6 {
            StrategyAttempt::NotAttempted => parts.push("IPv6: not checked".to_string()),
            StrategyAttempt::Success(_) => parts.push("IPv6: ok".to_string()),
            StrategyAttempt::Failed(e) => parts.push(format!("IPv6: {}", e)),
        }

        match &self.pcp {
            StrategyAttempt::NotAttempted => parts.push("PCP: not tried".to_string()),
            StrategyAttempt::Success(_) => parts.push("PCP: ok".to_string()),
            StrategyAttempt::Failed(e) => parts.push(format!("PCP: {}", e)),
        }

        match &self.natpmp {
            StrategyAttempt::NotAttempted => parts.push("NAT-PMP: not tried".to_string()),
            StrategyAttempt::Success(_) => parts.push("NAT-PMP: ok".to_string()),
            StrategyAttempt::Failed(e) => parts.push(format!("NAT-PMP: {}", e)),
        }

        match &self.upnp {
            StrategyAttempt::NotAttempted => parts.push("UPnP: not tried".to_string()),
            StrategyAttempt::Success(_) => parts.push("UPnP: ok".to_string()),
            StrategyAttempt::Failed(e) => parts.push(format!("UPnP: {}", e)),
        }

        parts.join(" → ")
    }
}

impl Default for ConnectivityResult {
    fn default() -> Self {
        Self::new()
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
