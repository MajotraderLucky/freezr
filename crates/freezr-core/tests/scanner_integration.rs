// Integration tests for ProcessScanner
// These tests interact with the actual system and may be skipped in CI/CD

use freezr_core::ProcessScanner;

#[test]
fn test_scanner_creation() {
    let _scanner = ProcessScanner::new();
    // Just verify it can be created
    assert!(true);
}

#[test]
#[ignore] // Ignore by default - requires actual KESL process
fn test_scan_kesl_real_process() {
    let scanner = ProcessScanner::new();

    match scanner.scan_kesl() {
        Ok(Some(process)) => {
            // KESL process found
            println!("Found KESL process:");
            println!("  PID: {}", process.pid);
            println!("  CPU: {:.1}%", process.cpu_percent);
            println!("  Memory: {} MB", process.memory_mb);

            assert!(process.pid > 0);
            assert!(process.is_kesl());
            assert!(process.command.contains("/opt/kaspersky/kesl/libexec/kesl"));
        }
        Ok(None) => {
            println!("KESL process not found (this is OK if KESL is not running)");
        }
        Err(e) => {
            panic!("Error scanning KESL: {}", e);
        }
    }
}

#[test]
#[ignore] // Ignore by default - requires actual Node.js processes
fn test_scan_node_processes_real() {
    let scanner = ProcessScanner::new();

    match scanner.scan_node_processes() {
        Ok(processes) => {
            println!("Found {} Node.js processes", processes.len());

            for proc in &processes {
                println!(
                    "  PID: {}, CPU: {:.1}%, Memory: {} MB",
                    proc.pid, proc.cpu_percent, proc.memory_mb
                );

                assert!(proc.pid > 0);
                assert!(proc.is_node());
            }

            // All found processes should be node processes
            for proc in processes {
                assert!(proc.is_node());
            }
        }
        Err(e) => {
            panic!("Error scanning Node processes: {}", e);
        }
    }
}

#[test]
fn test_scan_kesl_no_crash_on_missing() {
    let scanner = ProcessScanner::new();

    // Should not crash even if KESL is not running
    match scanner.scan_kesl() {
        Ok(_) => {
            // Any result is OK
            assert!(true);
        }
        Err(e) => {
            // Errors should be descriptive
            let error_msg = format!("{}", e);
            assert!(!error_msg.is_empty());
        }
    }
}

#[test]
fn test_scan_node_no_crash_on_empty() {
    let scanner = ProcessScanner::new();

    // Should not crash even if no Node processes
    match scanner.scan_node_processes() {
        Ok(processes) => {
            // Empty list is OK
            println!("Found {} Node processes", processes.len());
            // Length is always >= 0, just verify we got a valid vec
            assert!(true);
        }
        Err(e) => {
            // Errors should be descriptive
            let error_msg = format!("{}", e);
            assert!(!error_msg.is_empty());
        }
    }
}

#[test]
#[ignore] // Requires actual processes
fn test_scanner_performance() {
    use std::time::Instant;

    let scanner = ProcessScanner::new();

    // Test KESL scan performance
    let start = Instant::now();
    let _ = scanner.scan_kesl();
    let kesl_duration = start.elapsed();
    println!("KESL scan took: {:?}", kesl_duration);

    // Should complete within reasonable time (5 seconds)
    // Note: scan_kesl does 3 measurements with 1s sleep each
    assert!(kesl_duration.as_secs() < 5);

    // Test Node scan performance
    let start = Instant::now();
    let _ = scanner.scan_node_processes();
    let node_duration = start.elapsed();
    println!("Node scan took: {:?}", node_duration);

    // Should complete within reasonable time (2 seconds)
    assert!(node_duration.as_secs() < 2);
}

#[test]
#[ignore] // Requires system access
fn test_scanner_concurrent_safety() {
    use std::thread;

    let _scanner = ProcessScanner::new();

    // Spawn multiple threads scanning concurrently
    let handles: Vec<_> = (0..3)
        .map(|i| {
            thread::spawn(move || {
                let scanner = ProcessScanner::new();
                println!("Thread {} scanning...", i);
                let _ = scanner.scan_node_processes();
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // If we reach here, concurrent scanning works
    assert!(true);
}
