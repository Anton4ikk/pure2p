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
