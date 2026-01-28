//! Test utilities for the splug audio plugin
//!
//! Provides shared functionality for integration tests and build tools.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Information parsed from the standalone lockfile
#[derive(Debug)]
pub struct LockfileInfo {
    pub port: u16,
    pub pid: u32,
}

#[derive(serde::Deserialize)]
struct JsonLock {
    port: u16,
    pid: u32,
}

/// Clean up old lockfiles from previous runs
pub fn cleanup_lockfiles() -> Result<()> {
    let temp_dir = std::env::temp_dir();
    if let Ok(entries) = std::fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if (name.starts_with("splug_pid_") || name.starts_with("test_e2e_pid_"))
                    && name.ends_with(".json")
                {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }
    Ok(())
}

/// Spawn standalone process in background with fixed FPS
pub fn spawn_standalone(root_dir: &Path) -> Result<Child> {
    Command::new("cargo")
        .current_dir(root_dir)
        .args(["run", "-p", "standalone", "--", "--fixed-fps"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to spawn standalone")
}

/// Wait for lockfile to appear and return its path
pub async fn wait_for_lockfile(timeout: Duration) -> Result<PathBuf> {
    let start = Instant::now();
    let temp_dir = std::env::temp_dir();

    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("Timeout waiting for lockfile");
        }

        let mut recent_file = None;
        let mut recent_time = std::time::SystemTime::UNIX_EPOCH;

        if let Ok(entries) = std::fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Look for both splug_pid and test_e2e_pid lockfiles
                    if (name.starts_with("splug_pid_") || name.starts_with("test_e2e_pid_"))
                        && name.ends_with(".json")
                    {
                        if let Ok(metadata) = std::fs::metadata(&path) {
                            if let Ok(created) = metadata.created() {
                                if created > recent_time {
                                    recent_time = created;
                                    recent_file = Some(path);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(path) = recent_file {
            return Ok(path);
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Parse lockfile and extract port and PID
pub fn parse_lockfile(path: &Path) -> Result<LockfileInfo> {
    let content = std::fs::read_to_string(path)?;
    let json: JsonLock = serde_json::from_str(&content)?;
    Ok(LockfileInfo {
        port: json.port,
        pid: json.pid,
    })
}

/// Connect to RPC server with proper message size limits
pub async fn connect_rpc(
    port: u16,
    timeout: Duration,
) -> Result<debug_server::DebugControlClient<tonic::transport::Channel>> {
    let start = Instant::now();
    let addr = format!("http://127.0.0.1:{}", port);

    loop {
        let endpoint = match tonic::transport::Endpoint::from_shared(addr.clone()) {
            Ok(e) => e,
            Err(_) => {
                anyhow::bail!("Invalid URI");
            }
        };

        match endpoint.connect().await {
            Ok(channel) => {
                let client = debug_server::DebugControlClient::new(channel)
                    .max_decoding_message_size(16 * 1024 * 1024)
                    .max_encoding_message_size(16 * 1024 * 1024);
                return Ok(client);
            }
            Err(e) => {
                if start.elapsed() > timeout {
                    anyhow::bail!("Timeout connecting to RPC: {}", e);
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

/// Capture screenshot from standalone via RPC
pub async fn capture_screenshot(
    client: &mut debug_server::DebugControlClient<tonic::transport::Channel>,
) -> Result<image::RgbaImage> {
    let response = client
        .capture_screen(debug_server::Empty {})
        .await
        .context("RPC capture failed")?;
    let inner = response.into_inner();

    println!(
        "Received screenshot: {}x{} ({} bytes)",
        inner.width,
        inner.height,
        inner.data.len()
    );

    image::RgbaImage::from_raw(inner.width, inner.height, inner.data)
        .context("Failed to create image buffer")
}

/// Send quit command to standalone via RPC
pub async fn quit_standalone(
    client: &mut debug_server::DebugControlClient<tonic::transport::Channel>,
) -> Result<()> {
    client
        .send_input_event(debug_server::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Quit(
                debug_server::QuitMsg {},
            )),
        })
        .await
        .context("Failed to send quit")?;
    Ok(())
}

/// Complete workflow: spawn standalone, capture screenshot, and quit
pub async fn capture_golden(root_dir: &Path) -> Result<image::RgbaImage> {
    cleanup_lockfiles()?;

    println!("Starting standalone in background...");
    let mut child = spawn_standalone(root_dir)?;

    println!("Waiting for RPC server...");
    let lockfile_path = wait_for_lockfile(Duration::from_secs(10)).await?;
    let info = parse_lockfile(&lockfile_path)?;

    println!(
        "Connected to standalone on port {} (PID {})",
        info.port, info.pid
    );

    let mut client = connect_rpc(info.port, Duration::from_secs(5)).await?;

    // Wait for rendering to stabilize
    tokio::time::sleep(Duration::from_millis(1000)).await;

    println!("Requesting screenshot...");
    let img = capture_screenshot(&mut client).await?;

    println!("Sending Quit command...");
    quit_standalone(&mut client).await?;

    // Wait for child to exit
    let _ = child.wait();

    Ok(img)
}

/// Spawn test-e2e process in background with specified mode
fn spawn_test_e2e(root_dir: &Path, mode: &str) -> Result<Child> {
    cleanup_lockfiles()?;
    Command::new("cargo")
        .current_dir(root_dir)
        .args(["run", "-p", "test-e2e", "--", "--mode", mode])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to spawn test-e2e")
}

/// Complete workflow: spawn test-e2e with mode, capture screenshot, and quit
pub async fn capture_test_e2e_golden(root_dir: &Path, mode: &str) -> Result<image::RgbaImage> {
    println!("Starting test-e2e in {} mode...", mode);
    let mut child = spawn_test_e2e(root_dir, mode)?;

    println!("Waiting for RPC server...");
    let lockfile_path = wait_for_lockfile(Duration::from_secs(10)).await?;
    let info = parse_lockfile(&lockfile_path)?;

    println!(
        "Connected to test-e2e on port {} (PID {})",
        info.port, info.pid
    );

    let mut client = connect_rpc(info.port, Duration::from_secs(5)).await?;

    // Wait for rendering to stabilize
    tokio::time::sleep(Duration::from_millis(1000)).await;

    println!("Requesting screenshot...");
    let img = capture_screenshot(&mut client).await?;

    println!("Sending Quit command...");
    quit_standalone(&mut client).await?;

    // Wait for child to exit
    let _ = child.wait();

    Ok(img)
}
