// Request Log Tests - Testing request logging functionality for network debugging

use crate::storage::storage_db::Storage;

#[test]
fn test_log_request_creates_entry() {
    let storage = Storage::new_in_memory().unwrap();

    // Log a successful outgoing ping
    storage.log_request(
        "outgoing",
        "ping",
        Some("test_uid_123"),
        Some("192.168.1.100:8080"),
        Some(200),
        true,
        None,
        Some("ok"),
    ).unwrap();

    // Retrieve logs
    let logs = storage.get_request_logs(10).unwrap();
    assert_eq!(logs.len(), 1);

    let log = &logs[0];
    assert_eq!(log.direction, "outgoing");
    assert_eq!(log.request_type, "ping");
    assert_eq!(log.target_uid, Some("test_uid_123".to_string()));
    assert_eq!(log.target_ip, Some("192.168.1.100:8080".to_string()));
    assert_eq!(log.status_code, Some(200));
    assert!(log.success);
    assert_eq!(log.error_message, None);
    assert_eq!(log.response_data, Some("ok".to_string()));
}

#[test]
fn test_log_request_failed_with_error() {
    let storage = Storage::new_in_memory().unwrap();

    // Log a failed outgoing message
    storage.log_request(
        "outgoing",
        "text",
        Some("contact_uid_456"),
        Some("10.0.0.5:9000"),
        None,
        false,
        Some("Connection refused"),
        None,
    ).unwrap();

    let logs = storage.get_request_logs(10).unwrap();
    assert_eq!(logs.len(), 1);

    let log = &logs[0];
    assert_eq!(log.direction, "outgoing");
    assert_eq!(log.request_type, "text");
    assert!(!log.success);
    assert_eq!(log.status_code, None);
    assert_eq!(log.error_message, Some("Connection refused".to_string()));
    assert_eq!(log.response_data, None);
}

#[test]
fn test_log_incoming_request() {
    let storage = Storage::new_in_memory().unwrap();

    // Log an incoming ping
    storage.log_request(
        "incoming",
        "ping",
        Some("sender_uid_789"),
        Some("203.0.113.5:12345"),
        Some(200),
        true,
        None,
        None,
    ).unwrap();

    let logs = storage.get_request_logs(10).unwrap();
    assert_eq!(logs.len(), 1);

    let log = &logs[0];
    assert_eq!(log.direction, "incoming");
    assert_eq!(log.request_type, "ping");
    assert_eq!(log.target_uid, Some("sender_uid_789".to_string()));
    assert!(log.success);
}

#[test]
fn test_get_request_logs_ordering() {
    let storage = Storage::new_in_memory().unwrap();

    // Log multiple requests
    storage.log_request("outgoing", "ping", Some("uid1"), Some("ip1"), Some(200), true, None, None).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    storage.log_request("outgoing", "text", Some("uid2"), Some("ip2"), Some(200), true, None, None).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    storage.log_request("incoming", "ping", Some("uid3"), Some("ip3"), Some(200), true, None, None).unwrap();

    let logs = storage.get_request_logs(10).unwrap();
    assert_eq!(logs.len(), 3);

    // Should be in reverse chronological order (newest first)
    assert_eq!(logs[0].request_type, "ping");
    assert_eq!(logs[0].direction, "incoming");
    assert_eq!(logs[1].request_type, "text");
    assert_eq!(logs[2].request_type, "ping");
    assert_eq!(logs[2].direction, "outgoing");
}

#[test]
fn test_get_request_logs_limit() {
    let storage = Storage::new_in_memory().unwrap();

    // Log 5 requests
    for i in 0..5 {
        storage.log_request(
            "outgoing",
            "ping",
            Some(&format!("uid_{}", i)),
            Some(&format!("ip_{}", i)),
            Some(200),
            true,
            None,
            None,
        ).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    // Request only 3 logs
    let logs = storage.get_request_logs(3).unwrap();
    assert_eq!(logs.len(), 3);

    // Request all logs
    let all_logs = storage.get_request_logs(10).unwrap();
    assert_eq!(all_logs.len(), 5);
}

#[test]
fn test_get_request_logs_for_contact() {
    let storage = Storage::new_in_memory().unwrap();

    // Log requests for different contacts
    storage.log_request("outgoing", "ping", Some("alice"), Some("ip1"), Some(200), true, None, None).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    storage.log_request("outgoing", "text", Some("bob"), Some("ip2"), Some(200), true, None, None).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    storage.log_request("outgoing", "ping", Some("alice"), Some("ip1"), Some(200), true, None, None).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    storage.log_request("incoming", "ping", Some("alice"), Some("ip1"), Some(200), true, None, None).unwrap();

    // Get logs for alice only
    let alice_logs = storage.get_request_logs_for_contact("alice", 10).unwrap();
    assert_eq!(alice_logs.len(), 3);
    for log in &alice_logs {
        assert_eq!(log.target_uid, Some("alice".to_string()));
    }

    // Get logs for bob
    let bob_logs = storage.get_request_logs_for_contact("bob", 10).unwrap();
    assert_eq!(bob_logs.len(), 1);
    assert_eq!(bob_logs[0].target_uid, Some("bob".to_string()));

    // Get logs for non-existent contact
    let charlie_logs = storage.get_request_logs_for_contact("charlie", 10).unwrap();
    assert_eq!(charlie_logs.len(), 0);
}

#[test]
fn test_get_request_logs_for_contact_with_limit() {
    let storage = Storage::new_in_memory().unwrap();

    // Log 5 requests for the same contact
    for i in 0..5 {
        storage.log_request(
            "outgoing",
            &format!("msg_{}", i),
            Some("target_contact"),
            Some("ip"),
            Some(200),
            true,
            None,
            None,
        ).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    // Request only 2 logs
    let logs = storage.get_request_logs_for_contact("target_contact", 2).unwrap();
    assert_eq!(logs.len(), 2);

    // Should be newest first
    assert_eq!(logs[0].request_type, "msg_4");
    assert_eq!(logs[1].request_type, "msg_3");
}

#[test]
fn test_clear_old_request_logs() {
    let storage = Storage::new_in_memory().unwrap();

    // Create a log entry
    storage.log_request("outgoing", "ping", Some("uid"), Some("ip"), Some(200), true, None, None).unwrap();

    // Verify it exists
    let logs_before = storage.get_request_logs(10).unwrap();
    assert_eq!(logs_before.len(), 1);

    // Clear logs older than 30 days (should not delete recent log)
    storage.clear_old_request_logs(30).unwrap();
    let logs_after = storage.get_request_logs(10).unwrap();
    assert_eq!(logs_after.len(), 1);

    // Clear logs older than 0 days (should delete everything)
    storage.clear_old_request_logs(0).unwrap();
    let logs_cleared = storage.get_request_logs(10).unwrap();
    assert_eq!(logs_cleared.len(), 0);
}

#[test]
fn test_request_log_with_none_values() {
    let storage = Storage::new_in_memory().unwrap();

    // Log a request with minimal information
    storage.log_request(
        "outgoing",
        "unknown",
        None,
        None,
        None,
        false,
        Some("No contact info available"),
        None,
    ).unwrap();

    let logs = storage.get_request_logs(10).unwrap();
    assert_eq!(logs.len(), 1);

    let log = &logs[0];
    assert_eq!(log.target_uid, None);
    assert_eq!(log.target_ip, None);
    assert_eq!(log.status_code, None);
    assert!(!log.success);
    assert_eq!(log.error_message, Some("No contact info available".to_string()));
}

#[test]
fn test_request_log_timestamp_present() {
    let storage = Storage::new_in_memory().unwrap();

    let before = chrono::Utc::now().timestamp_millis();
    storage.log_request("outgoing", "ping", Some("uid"), Some("ip"), Some(200), true, None, None).unwrap();
    let after = chrono::Utc::now().timestamp_millis();

    let logs = storage.get_request_logs(10).unwrap();
    assert_eq!(logs.len(), 1);

    let log_timestamp = logs[0].timestamp;
    assert!(log_timestamp >= before);
    assert!(log_timestamp <= after);
}

#[test]
fn test_request_log_multiple_types() {
    let storage = Storage::new_in_memory().unwrap();

    // Log different request types
    storage.log_request("outgoing", "ping", Some("u1"), Some("ip1"), Some(200), true, None, None).unwrap();
    storage.log_request("outgoing", "text", Some("u2"), Some("ip2"), Some(200), true, None, None).unwrap();
    storage.log_request("outgoing", "delete", Some("u3"), Some("ip3"), Some(200), true, None, None).unwrap();
    storage.log_request("incoming", "ping", Some("u4"), Some("ip4"), Some(200), true, None, None).unwrap();
    storage.log_request("incoming", "text", Some("u5"), Some("ip5"), Some(200), true, None, None).unwrap();

    let logs = storage.get_request_logs(10).unwrap();
    assert_eq!(logs.len(), 5);

    // Check that all types are preserved
    let types: Vec<String> = logs.iter().map(|l| l.request_type.clone()).collect();
    assert!(types.contains(&"ping".to_string()));
    assert!(types.contains(&"text".to_string()));
    assert!(types.contains(&"delete".to_string()));
}

#[test]
fn test_request_log_various_status_codes() {
    let storage = Storage::new_in_memory().unwrap();

    // Log requests with different status codes
    storage.log_request("outgoing", "ping", Some("u1"), Some("ip"), Some(200), true, None, None).unwrap();
    storage.log_request("outgoing", "ping", Some("u2"), Some("ip"), Some(400), false, Some("Bad request"), None).unwrap();
    storage.log_request("outgoing", "ping", Some("u3"), Some("ip"), Some(404), false, Some("Not found"), None).unwrap();
    storage.log_request("outgoing", "ping", Some("u4"), Some("ip"), Some(500), false, Some("Server error"), None).unwrap();

    let logs = storage.get_request_logs(10).unwrap();
    assert_eq!(logs.len(), 4);

    // Verify status codes are preserved
    let status_codes: Vec<i32> = logs.iter().filter_map(|l| l.status_code).collect();
    assert!(status_codes.contains(&200));
    assert!(status_codes.contains(&400));
    assert!(status_codes.contains(&404));
    assert!(status_codes.contains(&500));
}

#[test]
fn test_clear_all_includes_request_logs() {
    let storage = Storage::new_in_memory().unwrap();

    // Add some request logs
    storage.log_request("outgoing", "ping", Some("uid"), Some("ip"), Some(200), true, None, None).unwrap();
    storage.log_request("incoming", "text", Some("uid2"), Some("ip2"), Some(200), true, None, None).unwrap();

    // Verify logs exist
    let logs_before = storage.get_request_logs(10).unwrap();
    assert_eq!(logs_before.len(), 2);

    // Clear all data
    storage.clear_all().unwrap();

    // Verify request logs are cleared
    let logs_after = storage.get_request_logs(10).unwrap();
    assert_eq!(logs_after.len(), 0);
}
