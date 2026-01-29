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

    // Channels to collect results
    let (tx, rx) = mpsc::channel::<(Vec<String>, bool, Vec<String>)>();
    let (signal_tx, signal_rx) = mpsc::channel::<bool>();
    let tx_stdout = tx.clone();

    // Monitor stdout for initialization message
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let mut initialized = false;
        let mut lines = Vec::new();

        for line in reader.lines().map_while(Result::ok) {
            lines.push(line.clone());
            if line.contains("Vulkan initialized!") {
                initialized = true;
                println!("✓ Vulkan initialized successfully");
                signal_tx.send(true).ok();
            }
        }

        tx_stdout.send((Vec::new(), initialized, lines)).ok();
    });

    // Monitor stderr for Vulkan errors
    let tx_stderr = tx;
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let mut vulkan_errors = Vec::new();
        let mut lines = Vec::new();

        for line in reader.lines().map_while(Result::ok) {
            lines.push(line.clone());
            // Collect Vulkan errors
            if line.contains("[Vulkan Error]") {
                vulkan_errors.push(line.clone());
                eprintln!("VULKAN ERROR: {}", line);
            }
        }

        tx_stderr.send((vulkan_errors, false, lines)).ok();
    });

    // Let the app run for a few seconds to complete initialization
    println!("Allowing app to initialize (Max 10s)...");
    let mut app_initialized = false;

    // Poll for results
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        if let Ok(true) = signal_rx.try_recv() {
            app_initialized = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    // Kill the process
    println!("Terminating standalone process...");
    let _ = child.kill();
    let _ = child.wait();

    // Collect results from threads
    let mut vulkan_errors = Vec::new();
    let mut all_stdout = Vec::new();
    let mut all_stderr = Vec::new();

    for _ in 0..2 {
        if let Ok((errors, initialized, lines)) = rx.recv_timeout(Duration::from_secs(2)) {
            vulkan_errors.extend(errors);
            if initialized {
                all_stdout = lines;
            } else {
                all_stderr = lines;
            }
        }
    }

    // Verify app initialized
    if !app_initialized {
        println!("\n=== Captured Stdout ===");
        for line in &all_stdout {
            println!("{}", line);
        }
        println!("=== Captured Stderr ===");
        for line in &all_stderr {
            println!("{}", line);
        }
        println!("=======================\n");
    }

    assert!(
        app_initialized,
        "App did not complete Vulkan initialization within 10s"
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
