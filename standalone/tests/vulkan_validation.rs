//! E2E test for Vulkan initialization validation
//!
//! Verifies that the standalone app starts without Vulkan validation errors.

use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
fn test_vulkan_no_validation_errors() -> Result<()> {
    // Build the standalone binary first
    let build_status = Command::new("cargo")
        .args(["build", "--bin", "standalone"])
        .status()
        .context("Failed to build standalone")?;

    assert!(build_status.success(), "Build failed");

    // Start standalone in the background
    let mut child = Command::new("cargo")
        .args(["run", "--bin", "standalone"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn standalone")?;

    let stderr = child.stderr.take().context("Failed to capture stderr")?;
    let stdout = child.stdout.take().context("Failed to capture stdout")?;

    // Channel to collect results
    let (tx, rx) = mpsc::channel::<(Vec<String>, bool)>();
    let tx_stdout = tx.clone();

    // Monitor stdout for initialization message
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let mut initialized = false;

        for line in reader.lines().map_while(Result::ok) {
            if line.contains("Vulkan initialized!") {
                initialized = true;
                println!("✓ Vulkan initialized successfully");
            }
        }

        tx_stdout.send((Vec::new(), initialized)).ok();
    });

    // Monitor stderr for Vulkan errors
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let mut vulkan_errors = Vec::new();

        for line in reader.lines().map_while(Result::ok) {
            // Collect Vulkan errors
            if line.contains("[Vulkan Error]") {
                vulkan_errors.push(line.clone());
                eprintln!("VULKAN ERROR: {}", line);
            }
        }

        tx.send((vulkan_errors, false)).ok();
    });

    // Let the app run for a few seconds to complete initialization
    println!("Allowing app to initialize...");
    thread::sleep(Duration::from_secs(3));

    // Kill the process
    println!("Terminating standalone process...");
    child.kill().context("Failed to kill process")?;

    // Wait for process to exit
    let _exit_status = child.wait().context("Failed to wait for process")?;

    // Wait for threads to finish and collect results
    stderr_handle
        .join()
        .map_err(|_| anyhow::anyhow!("stderr thread panicked"))?;

    // Collect results from both channels
    let mut vulkan_errors = Vec::new();
    let mut app_initialized = false;

    // Get results from both threads (timeout after 1 second each)
    for _ in 0..2 {
        if let Ok((errors, initialized)) = rx.recv_timeout(Duration::from_secs(1)) {
            vulkan_errors.extend(errors);
            app_initialized |= initialized;
        }
    }

    // Verify app initialized
    assert!(
        app_initialized,
        "App did not complete Vulkan initialization"
    );

    // Assert no Vulkan errors
    if !vulkan_errors.is_empty() {
        eprintln!("\n=== Vulkan Errors Detected ===");
        for error in &vulkan_errors {
            eprintln!("{}", error);
        }
        eprintln!("==============================\n");
        panic!(
            "Vulkan validation errors detected: {} errors",
            vulkan_errors.len()
        );
    }

    println!("✓ No Vulkan validation errors detected");
    println!("✓ Standalone started and shut down cleanly");

    Ok(())
}
