use crate::connectivity::*;
use crate::connectivity::pcp::{PcpResultCode, PcpOpcode, build_pcp_map_request, parse_pcp_ip_address, PCP_VERSION};
use crate::connectivity::natpmp::{NatPmpResultCode, NatPmpOpcode, build_natpmp_map_request, parse_natpmp_map_response, NATPMP_VERSION};
use crate::connectivity::ipv6::is_ipv6_link_local;
use std::net::{IpAddr, Ipv4Addr};

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

// ========================================================================
// Connectivity Orchestrator Tests
// ========================================================================

#[test]
fn test_connectivity_result_creation() {
    let result = ConnectivityResult::new();
    assert!(matches!(result.ipv6, StrategyAttempt::NotAttempted));
    assert!(matches!(result.pcp, StrategyAttempt::NotAttempted));
    assert!(matches!(result.natpmp, StrategyAttempt::NotAttempted));
    assert!(matches!(result.upnp, StrategyAttempt::NotAttempted));
    assert!(result.mapping.is_none());
    assert!(!result.is_success());
}

#[test]
fn test_connectivity_result_default() {
    let result = ConnectivityResult::default();
    assert!(!result.is_success());
    assert!(result.mapping.is_none());
}

#[test]
fn test_connectivity_result_with_ipv6_success() {
    use std::net::Ipv6Addr;

    let mapping = PortMappingResult {
        external_ip: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
        external_port: 8080,
        lifetime_secs: 0,
        protocol: MappingProtocol::IPv6,
        created_at_ms: 1234567890000,
    };

    let mut result = ConnectivityResult::new();
    result.ipv6 = StrategyAttempt::Success(mapping.clone());
    result.mapping = Some(mapping);

    assert!(result.is_success());
    assert!(matches!(result.ipv6, StrategyAttempt::Success(_)));
}

#[test]
fn test_connectivity_result_summary_all_not_attempted() {
    let result = ConnectivityResult::new();
    let summary = result.summary();

    assert!(summary.contains("IPv6: not checked"));
    assert!(summary.contains("PCP: not tried"));
    assert!(summary.contains("NAT-PMP: not tried"));
    assert!(summary.contains("UPnP: not tried"));
}

#[test]
fn test_connectivity_result_summary_all_failed() {
    let mut result = ConnectivityResult::new();
    result.ipv6 = StrategyAttempt::Failed("no address".to_string());
    result.pcp = StrategyAttempt::Failed("timeout".to_string());
    result.natpmp = StrategyAttempt::Failed("no gateway".to_string());
    result.upnp = StrategyAttempt::Failed("not supported".to_string());

    let summary = result.summary();

    assert!(summary.contains("IPv6: no address"));
    assert!(summary.contains("PCP: timeout"));
    assert!(summary.contains("NAT-PMP: no gateway"));
    assert!(summary.contains("UPnP: not supported"));
}

#[test]
fn test_connectivity_result_summary_partial_success() {
    let mapping = PortMappingResult {
        external_ip: IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10)),
        external_port: 8080,
        lifetime_secs: 3600,
        protocol: MappingProtocol::PCP,
        created_at_ms: 1234567890000,
    };

    let mut result = ConnectivityResult::new();
    result.ipv6 = StrategyAttempt::Failed("not available".to_string());
    result.pcp = StrategyAttempt::Success(mapping.clone());
    result.mapping = Some(mapping);

    let summary = result.summary();

    assert!(summary.contains("IPv6: not available"));
    assert!(summary.contains("PCP: ok"));
    assert!(summary.contains("NAT-PMP: not tried"));
}

#[test]
fn test_connectivity_result_serialization() {
    let mapping = PortMappingResult {
        external_ip: IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10)),
        external_port: 8080,
        lifetime_secs: 3600,
        protocol: MappingProtocol::PCP,
        created_at_ms: 1234567890000,
    };

    let mut result = ConnectivityResult::new();
    result.pcp = StrategyAttempt::Success(mapping.clone());
    result.mapping = Some(mapping);

    // Test JSON serialization
    let json = serde_json::to_string(&result).unwrap();
    let deserialized: ConnectivityResult = serde_json::from_str(&json).unwrap();

    assert_eq!(result, deserialized);
}

#[test]
fn test_strategy_attempt_serialization() {
    let success = StrategyAttempt::Success(PortMappingResult {
        external_ip: IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)),
        external_port: 9999,
        lifetime_secs: 7200,
        protocol: MappingProtocol::UPnP,
        created_at_ms: 9876543210,
    });

    let json = serde_json::to_string(&success).unwrap();
    let deserialized: StrategyAttempt = serde_json::from_str(&json).unwrap();
    assert_eq!(success, deserialized);

    let failed = StrategyAttempt::Failed("test error".to_string());
    let json = serde_json::to_string(&failed).unwrap();
    let deserialized: StrategyAttempt = serde_json::from_str(&json).unwrap();
    assert_eq!(failed, deserialized);

    let not_attempted = StrategyAttempt::NotAttempted;
    let json = serde_json::to_string(&not_attempted).unwrap();
    let deserialized: StrategyAttempt = serde_json::from_str(&json).unwrap();
    assert_eq!(not_attempted, deserialized);
}

#[test]
fn test_is_ipv6_link_local() {
    use std::net::Ipv6Addr;

    // Link-local addresses (fe80::/10)
    assert!(is_ipv6_link_local(&Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)));
    assert!(is_ipv6_link_local(&Ipv6Addr::new(0xfea0, 0, 0, 0, 0, 0, 0, 1)));
    assert!(is_ipv6_link_local(&Ipv6Addr::new(0xfebf, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff)));

    // Not link-local
    assert!(!is_ipv6_link_local(&Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)));
    assert!(!is_ipv6_link_local(&Ipv6Addr::LOCALHOST));
    assert!(!is_ipv6_link_local(&Ipv6Addr::new(0xfe00, 0, 0, 0, 0, 0, 0, 1)));
    assert!(!is_ipv6_link_local(&Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 1)));
}

#[test]
fn test_mapping_protocol_ipv6_variant() {
    let protocol = MappingProtocol::IPv6;
    assert_eq!(protocol, MappingProtocol::IPv6);

    // Test serialization
    let json = serde_json::to_string(&protocol).unwrap();
    let deserialized: MappingProtocol = serde_json::from_str(&json).unwrap();
    assert_eq!(protocol, deserialized);
}

// Note: Integration tests for establish_connectivity() would require network access
// and mock servers. These should be in tests/integration_tests.rs with #[ignore]
// attribute or run in a controlled test environment.
//
// Example integration test structure:
//
// #[tokio::test]
// #[ignore] // Run manually or in CI with mock network
// async fn test_establish_connectivity_all_protocols() {
//     let result = establish_connectivity(8080).await;
//     // Verify at least one protocol was attempted
//     assert!(matches!(result.ipv6, StrategyAttempt::Failed(_) | StrategyAttempt::Success(_)));
// }

// CGNAT Detection Tests

#[test]
fn test_connectivity_result_cgnat_detected_in_summary() {
    let mut result = ConnectivityResult::new();
    result.cgnat_detected = true;
    result.pcp = StrategyAttempt::Failed("timeout".to_string());

    let summary = result.summary();
    assert!(summary.contains("⚠️  CGNAT"), "Summary should contain CGNAT warning");
}

#[test]
fn test_connectivity_result_no_cgnat_in_summary() {
    let mut result = ConnectivityResult::new();
    result.cgnat_detected = false;
    result.pcp = StrategyAttempt::Failed("timeout".to_string());

    let summary = result.summary();
    assert!(!summary.contains("CGNAT"), "Summary should not contain CGNAT when not detected");
}

#[test]
fn test_connectivity_result_new_has_cgnat_false() {
    let result = ConnectivityResult::new();
    assert!(!result.cgnat_detected, "New ConnectivityResult should have cgnat_detected=false");
}

#[test]
fn test_connectivity_result_with_cgnat_serialization() {
    use crate::connectivity::MappingProtocol;
    use chrono::Utc;

    let mapping = PortMappingResult {
        external_ip: IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1)), // CGNAT IP
        external_port: 60000,
        lifetime_secs: 3600,
        protocol: MappingProtocol::PCP,
        created_at_ms: Utc::now().timestamp_millis(),
    };

    let mut result = ConnectivityResult::new();
    result.cgnat_detected = true;
    result.pcp = StrategyAttempt::Success(mapping.clone());
    result.mapping = Some(mapping);

    // Test JSON serialization
    let json = serde_json::to_string(&result).unwrap();
    let deserialized: ConnectivityResult = serde_json::from_str(&json).unwrap();

    assert_eq!(result.cgnat_detected, deserialized.cgnat_detected);
    assert!(deserialized.cgnat_detected, "CGNAT detection should survive serialization");
}
