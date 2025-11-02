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

// Submodules
pub mod cgnat;
pub mod gateway;
pub mod health_check;
pub mod http_ip;
pub mod ipv6;
pub mod manager;
pub mod natpmp;
pub mod orchestrator;
pub mod pcp;
pub mod types;
pub mod upnp;

// Re-export commonly used types
pub use types::{
    ConnectivityResult, IpProtocol, MappingError, MappingProtocol, PortMappingResult,
    StrategyAttempt,
};

// Re-export main functions
pub use cgnat::{detect_cgnat, is_private_ip};
pub use health_check::{verify_external_reachability, ReachabilityStatus};
pub use http_ip::detect_external_ip;
pub use natpmp::{try_natpmp_mapping, try_natpmp_mapping_with_protocol};
pub use orchestrator::{establish_connectivity, verify_connectivity_health};
pub use pcp::{try_pcp_mapping, try_pcp_mapping_with_protocol};
pub use upnp::{delete_upnp_mapping, try_upnp_mapping, try_upnp_mapping_with_protocol};

// Re-export managers
pub use manager::{PortMappingManager, UpnpMappingManager};
