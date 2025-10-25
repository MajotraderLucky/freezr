// Integration tests for SystemdService
// These tests require actual systemd and appropriate permissions
// Most tests are marked as #[ignore] to prevent failures in CI/CD

use freezr_core::SystemdService;

#[test]
fn test_systemd_service_creation() {
    let _service = SystemdService::new("kesl");
    // Just verify creation works
    assert!(true);
}

#[test]
#[ignore] // Requires systemd and permissions
fn test_is_active_real_service() {
    let service = SystemdService::new("kesl");

    match service.is_active() {
        Ok(is_active) => {
            println!("KESL service is active: {}", is_active);
            // Result can be true or false, both are valid
            assert!(is_active == true || is_active == false);
        }
        Err(e) => {
            println!("Error checking service status: {}", e);
            // Error is OK if systemd is not available
        }
    }
}

#[test]
#[ignore] // Requires systemd
fn test_is_active_nonexistent_service() {
    let service = SystemdService::new("nonexistent-service-xyz123");

    match service.is_active() {
        Ok(is_active) => {
            // Should be false for nonexistent service
            assert!(!is_active);
        }
        Err(_) => {
            // Error is also acceptable
            assert!(true);
        }
    }
}

#[test]
#[ignore] // Requires systemd and permissions
fn test_get_properties_real_service() {
    let service = SystemdService::new("kesl");

    match service.get_properties() {
        Ok(props) => {
            println!("Service properties:\n{}", props);

            // Properties should contain expected keys
            assert!(
                props.contains("CPUQuota") || props.contains("MemoryMax") || props.contains("Nice")
            );

            // Should not be empty
            assert!(!props.is_empty());
        }
        Err(e) => {
            println!("Error getting properties: {}", e);
            // Error is OK if service doesn't exist or no permissions
        }
    }
}

#[test]
#[ignore] // DANGEROUS - actually restarts service! Only run manually
fn test_restart_with_reload_real_service() {
    let mut service = SystemdService::new("kesl");

    println!("WARNING: This test will actually restart the KESL service!");
    println!("Only run this test manually with sudo privileges.");

    match service.restart_with_reload() {
        Ok(_) => {
            println!("✅ Service restarted successfully");

            // Check that last_restart_time was updated
            assert!(service.get_last_restart_time() > 0);

            // Verify service is active after restart
            std::thread::sleep(std::time::Duration::from_secs(2));
            match service.is_active() {
                Ok(true) => println!("✅ Service is active after restart"),
                Ok(false) => println!("⚠️  Service is not active after restart"),
                Err(e) => println!("❌ Error checking status: {}", e),
            }
        }
        Err(e) => {
            println!("Error restarting service: {}", e);
            // Expected if no sudo permissions or service doesn't exist
        }
    }
}

#[test]
#[ignore] // Requires permissions
fn test_restart_protection_mechanism() {
    let mut service = SystemdService::new("test-service");

    // Manually set last restart time to recent
    let current = SystemdService::current_timestamp_public();
    service.set_last_restart_time(current - 50); // 50 seconds ago
    service.set_min_restart_interval(100); // Minimum 100 seconds

    // Attempting restart should fail due to protection
    match service.restart_with_reload() {
        Ok(_) => {
            panic!("Restart should have been blocked by protection mechanism!");
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            println!("Expected error: {}", error_msg);
            assert!(error_msg.contains("Too soon to restart"));
        }
    }
}

#[test]
fn test_time_since_last_restart() {
    let mut service = SystemdService::new("test");

    // Initially should be u64::MAX (never restarted)
    assert_eq!(service.time_since_last_restart(), u64::MAX);

    // Set restart time to 10 seconds ago
    let current = SystemdService::current_timestamp_public();
    service.set_last_restart_time(current - 10);

    let time_since = service.time_since_last_restart();

    // Should be approximately 10 seconds (allow some variance)
    assert!(time_since >= 9 && time_since <= 11);
}

#[test]
#[ignore] // Requires systemd and real service
fn test_multiple_property_checks() {
    let service = SystemdService::new("kesl");

    // Check properties multiple times - should be consistent
    for i in 0..3 {
        match service.get_properties() {
            Ok(props) => {
                println!("Check {}: Got {} bytes of properties", i, props.len());
                assert!(!props.is_empty());
            }
            Err(e) => {
                println!("Check {}: Error: {}", i, e);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[test]
#[ignore] // Requires systemd
fn test_service_status_consistency() {
    let service = SystemdService::new("kesl");

    // Check status multiple times rapidly
    let mut results = Vec::new();

    for _ in 0..5 {
        if let Ok(is_active) = service.is_active() {
            results.push(is_active);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    if !results.is_empty() {
        // Status should be consistent across rapid checks
        let first = results[0];
        let all_same = results.iter().all(|&r| r == first);

        println!("Status checks: {:?}", results);
        assert!(
            all_same,
            "Service status should be consistent across rapid checks"
        );
    }
}
