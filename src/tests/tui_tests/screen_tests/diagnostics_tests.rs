// DiagnosticsScreen Tests - Testing network diagnostics screen

use crate::connectivity::{ConnectivityResult, MappingProtocol, PortMappingResult, StrategyAttempt};
use crate::tui::screens::DiagnosticsScreen;
use chrono::Utc;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[test]
fn test_diagnostics_screen_new() {
    let screen = DiagnosticsScreen::new(8080);

    assert_eq!(screen.local_port, 8080);
    assert!(!screen.cgnat_detected);
    assert!(!screen.is_refreshing);
    assert!(screen.pcp_status.is_none());
    assert!(screen.natpmp_status.is_none());
    assert!(screen.upnp_status.is_none());
}

#[test]
fn test_diagnostics_screen_set_cgnat_detected() {
    let mut screen = DiagnosticsScreen::new(8080);
    assert!(!screen.cgnat_detected);

    screen.set_cgnat_detected(true);
    assert!(screen.cgnat_detected);

    screen.set_cgnat_detected(false);
    assert!(!screen.cgnat_detected);
}

#[test]
fn test_diagnostics_screen_update_from_connectivity_result() {
    let mut screen = DiagnosticsScreen::new(8080);

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
    result.natpmp = StrategyAttempt::Failed("No gateway".to_string());
    result.mapping = Some(mapping);

    screen.update_from_connectivity_result(&result);

    assert!(screen.cgnat_detected, "CGNAT should be detected");
    assert!(!screen.is_refreshing, "Should not be refreshing after update");
    assert!(screen.pcp_status.is_some(), "PCP status should be set");
    assert!(screen.natpmp_status.is_some(), "NAT-PMP status should be set");

    // Verify PCP succeeded
    if let Some(Ok(pcp_mapping)) = &screen.pcp_status {
        assert_eq!(pcp_mapping.external_port, 60000);
    } else {
        panic!("Expected PCP success status");
    }

    // Verify NAT-PMP failed
    if let Some(Err(error)) = &screen.natpmp_status {
        assert!(error.contains("No gateway"));
    } else {
        panic!("Expected NAT-PMP error status");
    }
}

#[test]
fn test_diagnostics_screen_start_refresh() {
    let mut screen = DiagnosticsScreen::new(8080);
    assert!(!screen.is_refreshing);

    screen.start_refresh();
    assert!(screen.is_refreshing);
    assert!(screen.status_message.is_some());
}

#[test]
fn test_diagnostics_screen_new_fields() {
    let screen = DiagnosticsScreen::new(8080);

    assert!(screen.ipv4_address.is_none());
    assert!(screen.ipv6_address.is_none());
    assert!(screen.external_endpoint.is_none());
    assert!(screen.last_ping_rtt_ms.is_none());
    assert_eq!(screen.queue_size, 0);
}

#[test]
fn test_diagnostics_screen_set_ipv4_address() {
    let mut screen = DiagnosticsScreen::new(8080);

    screen.set_ipv4_address(Some("192.168.1.100".to_string()));
    assert_eq!(screen.ipv4_address, Some("192.168.1.100".to_string()));

    screen.set_ipv4_address(None);
    assert!(screen.ipv4_address.is_none());
}

#[test]
fn test_diagnostics_screen_set_ipv6_address() {
    let mut screen = DiagnosticsScreen::new(8080);

    screen.set_ipv6_address(Some("2001:db8::1".to_string()));
    assert_eq!(screen.ipv6_address, Some("2001:db8::1".to_string()));

    screen.set_ipv6_address(None);
    assert!(screen.ipv6_address.is_none());
}

#[test]
fn test_diagnostics_screen_set_external_endpoint() {
    let mut screen = DiagnosticsScreen::new(8080);

    screen.set_external_endpoint(Some("203.0.113.1:60000".to_string()));
    assert_eq!(screen.external_endpoint, Some("203.0.113.1:60000".to_string()));

    screen.set_external_endpoint(None);
    assert!(screen.external_endpoint.is_none());
}

#[test]
fn test_diagnostics_screen_set_last_ping_rtt() {
    let mut screen = DiagnosticsScreen::new(8080);

    screen.set_last_ping_rtt(Some(42));
    assert_eq!(screen.last_ping_rtt_ms, Some(42));

    screen.set_last_ping_rtt(None);
    assert!(screen.last_ping_rtt_ms.is_none());
}

#[test]
fn test_diagnostics_screen_set_queue_size() {
    let mut screen = DiagnosticsScreen::new(8080);

    screen.set_queue_size(5);
    assert_eq!(screen.queue_size, 5);

    screen.set_queue_size(0);
    assert_eq!(screen.queue_size, 0);
}

#[test]
fn test_diagnostics_screen_external_endpoint_from_connectivity_result() {
    let mut screen = DiagnosticsScreen::new(8080);

    let mapping = PortMappingResult {
        external_ip: IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)),
        external_port: 60000,
        lifetime_secs: 3600,
        protocol: MappingProtocol::PCP,
        created_at_ms: Utc::now().timestamp_millis(),
    };

    let mut result = ConnectivityResult::new();
    result.pcp = StrategyAttempt::Success(mapping.clone());
    result.mapping = Some(mapping);

    screen.update_from_connectivity_result(&result);

    assert_eq!(screen.external_endpoint, Some("203.0.113.1:60000".to_string()));
    assert_eq!(screen.ipv4_address, Some("203.0.113.1".to_string()));
}

#[test]
fn test_diagnostics_screen_ipv6_detection_from_connectivity_result() {
    let mut screen = DiagnosticsScreen::new(8080);

    let mapping = PortMappingResult {
        external_ip: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
        external_port: 60000,
        lifetime_secs: 3600,
        protocol: MappingProtocol::IPv6,
        created_at_ms: Utc::now().timestamp_millis(),
    };

    let mut result = ConnectivityResult::new();
    result.ipv6 = StrategyAttempt::Success(mapping.clone());
    result.mapping = Some(mapping);

    screen.update_from_connectivity_result(&result);

    assert_eq!(screen.external_endpoint, Some("2001:db8::1:60000".to_string()));
    assert_eq!(screen.ipv6_address, Some("2001:db8::1".to_string()));
}

#[test]
fn test_diagnostics_screen_get_remaining_lifetime_secs() {
    let mut screen = DiagnosticsScreen::new(8080);

    let mapping = PortMappingResult {
        external_ip: IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)),
        external_port: 60000,
        lifetime_secs: 3600, // 1 hour
        protocol: MappingProtocol::PCP,
        created_at_ms: Utc::now().timestamp_millis() - 1000, // Created 1 second ago
    };

    screen.set_pcp_status(Ok(mapping));

    let remaining = screen.get_remaining_lifetime_secs();
    assert!(remaining.is_some());
    let remaining_secs = remaining.unwrap();
    // Should be close to 3599 seconds (3600 - 1)
    assert!(remaining_secs >= 3598 && remaining_secs <= 3600);
}

#[test]
fn test_diagnostics_screen_get_renewal_countdown_secs() {
    let mut screen = DiagnosticsScreen::new(8080);

    let mapping = PortMappingResult {
        external_ip: IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)),
        external_port: 60000,
        lifetime_secs: 3600, // 1 hour
        protocol: MappingProtocol::PCP,
        created_at_ms: Utc::now().timestamp_millis() - 1000, // Created 1 second ago
    };

    screen.set_pcp_status(Ok(mapping));

    let renewal = screen.get_renewal_countdown_secs();
    assert!(renewal.is_some());
    let renewal_secs = renewal.unwrap();
    // Renewal at 80% = 2880 seconds, minus 1 elapsed = ~2879
    assert!(renewal_secs >= 2878 && renewal_secs <= 2880);
}

#[test]
fn test_diagnostics_screen_format_time_remaining() {
    assert_eq!(DiagnosticsScreen::format_time_remaining(30), "30s");
    assert_eq!(DiagnosticsScreen::format_time_remaining(90), "1m 30s");
    assert_eq!(DiagnosticsScreen::format_time_remaining(3661), "1h 1m");
    assert_eq!(DiagnosticsScreen::format_time_remaining(7200), "2h 0m");
}

#[test]
fn test_diagnostics_screen_no_active_mapping() {
    let screen = DiagnosticsScreen::new(8080);

    assert!(screen.get_remaining_lifetime_secs().is_none());
    assert!(screen.get_renewal_countdown_secs().is_none());
}

#[test]
fn test_diagnostics_screen_http_fallback_success() {
    let mut screen = DiagnosticsScreen::new(8080);

    // Create HTTP fallback mapping (protocol: Direct)
    let mapping = PortMappingResult {
        external_ip: IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)),
        external_port: 8080,
        lifetime_secs: 0, // HTTP fallback has no lifetime
        protocol: MappingProtocol::Direct,
        created_at_ms: Utc::now().timestamp_millis(),
    };

    let mut result = ConnectivityResult::new();
    result.pcp = StrategyAttempt::Failed("No gateway".to_string());
    result.natpmp = StrategyAttempt::Failed("No gateway".to_string());
    result.upnp = StrategyAttempt::Failed("No devices found".to_string());
    result.http = StrategyAttempt::Success(mapping.clone());
    result.mapping = Some(mapping.clone());

    screen.update_from_connectivity_result(&result);

    // Verify HTTP fallback status is set
    assert!(screen.http_fallback_status.is_some(), "HTTP fallback status should be set");

    if let Some(Ok(http_mapping)) = &screen.http_fallback_status {
        assert_eq!(http_mapping.external_ip, IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)));
        assert_eq!(http_mapping.protocol, MappingProtocol::Direct);
    } else {
        panic!("Expected HTTP fallback success status");
    }

    // Verify other protocols failed
    assert!(screen.pcp_status.as_ref().unwrap().is_err());
    assert!(screen.natpmp_status.as_ref().unwrap().is_err());
    assert!(screen.upnp_status.as_ref().unwrap().is_err());
}

#[test]
fn test_diagnostics_screen_http_fallback_failure() {
    let mut screen = DiagnosticsScreen::new(8080);

    // All protocols including HTTP fallback failed
    let mut result = ConnectivityResult::new();
    result.pcp = StrategyAttempt::Failed("No gateway".to_string());
    result.natpmp = StrategyAttempt::Failed("No gateway".to_string());
    result.upnp = StrategyAttempt::Failed("No devices found".to_string());
    result.http = StrategyAttempt::Failed("All services timed out".to_string());
    result.mapping = None; // No mapping at all

    screen.update_from_connectivity_result(&result);

    // Verify HTTP fallback status is set to error
    assert!(screen.http_fallback_status.is_some(), "HTTP fallback status should be set");

    if let Some(Err(error)) = &screen.http_fallback_status {
        assert!(error.contains("All services timed out"));
    } else {
        panic!("Expected HTTP fallback error status");
    }
}

#[test]
fn test_diagnostics_screen_pcp_success_no_http_fallback() {
    let mut screen = DiagnosticsScreen::new(8080);

    // PCP succeeded, no need for HTTP fallback
    let mapping = PortMappingResult {
        external_ip: IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)),
        external_port: 60000,
        lifetime_secs: 3600,
        protocol: MappingProtocol::PCP,
        created_at_ms: Utc::now().timestamp_millis(),
    };

    let mut result = ConnectivityResult::new();
    result.pcp = StrategyAttempt::Success(mapping.clone());
    result.mapping = Some(mapping);

    screen.update_from_connectivity_result(&result);

    // Verify HTTP fallback is NOT set (wasn't needed)
    assert!(screen.http_fallback_status.is_none(), "HTTP fallback should not be set when NAT traversal succeeds");

    // Verify PCP succeeded
    assert!(screen.pcp_status.as_ref().unwrap().is_ok());
}

#[test]
fn test_diagnostics_screen_http_fallback_field_initialized() {
    let screen = DiagnosticsScreen::new(8080);

    // Verify new field is initialized to None
    assert!(screen.http_fallback_status.is_none(), "HTTP fallback status should be None initially");
}

#[test]
fn test_diagnostics_screen_set_http_fallback_status() {
    let mut screen = DiagnosticsScreen::new(8080);

    let mapping = PortMappingResult {
        external_ip: IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)),
        external_port: 8080,
        lifetime_secs: 0,
        protocol: MappingProtocol::Direct,
        created_at_ms: Utc::now().timestamp_millis(),
    };

    // Test setter method
    screen.set_http_fallback_status(Ok(mapping.clone()));

    assert!(screen.http_fallback_status.is_some());
    assert!(screen.http_fallback_status.as_ref().unwrap().is_ok());
    assert!(!screen.is_refreshing, "Should stop refreshing after HTTP fallback");
}

#[test]
fn test_diagnostics_screen_http_fallback_from_connectivity_result() {
    let mut screen = DiagnosticsScreen::new(8080);

    // Create a connectivity result where all NAT methods failed but HTTP succeeded
    let http_mapping = PortMappingResult {
        external_ip: IpAddr::V4(Ipv4Addr::new(187, 33, 153, 17)),
        external_port: 64275,
        lifetime_secs: 0, // HTTP fallback has no NAT mapping lifetime
        protocol: MappingProtocol::Direct,
        created_at_ms: Utc::now().timestamp_millis(),
    };

    let mut result = ConnectivityResult::new();
    result.pcp = StrategyAttempt::Failed("Mapping request timed out".to_string());
    result.natpmp = StrategyAttempt::Failed("Mapping request timed out".to_string());
    result.upnp = StrategyAttempt::Failed("No gateway found".to_string());
    result.http = StrategyAttempt::Success(http_mapping.clone());
    result.mapping = Some(http_mapping.clone());
    result.cgnat_detected = false;

    screen.update_from_connectivity_result(&result);

    // Verify all NAT methods show as failed
    assert!(screen.pcp_status.is_some());
    assert!(screen.pcp_status.as_ref().unwrap().is_err());
    assert!(screen.natpmp_status.is_some());
    assert!(screen.natpmp_status.as_ref().unwrap().is_err());
    assert!(screen.upnp_status.is_some());
    assert!(screen.upnp_status.as_ref().unwrap().is_err());

    // Verify HTTP fallback succeeded
    assert!(screen.http_fallback_status.is_some());
    assert!(screen.http_fallback_status.as_ref().unwrap().is_ok());

    // Verify external endpoint was set from HTTP fallback
    assert_eq!(
        screen.external_endpoint,
        Some("187.33.153.17:64275".to_string())
    );
    assert_eq!(screen.ipv4_address, Some("187.33.153.17".to_string()));

    // Verify CGNAT not detected
    assert!(!screen.cgnat_detected);
    assert!(!screen.is_refreshing);
}
