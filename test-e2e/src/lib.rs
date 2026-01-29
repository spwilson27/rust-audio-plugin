//! Test helpers for splitting E2E binary and shared test logic
//!
//! Exposes `ProcessGuard` and helper functions for spawning test processes and
//! interacting with them via RPC.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Information parsed from the lockfile
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

/// RAII wrapper to ensure child process is killed on drop
pub struct ProcessGuard(pub Child);

impl ProcessGuard {
    pub fn spawn(bin_path: &str, args: &[&str]) -> Result<Self> {
        let child = Command::new(bin_path)
            .args(args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn process")?;
        Ok(Self(child))
    }

    pub fn id(&self) -> u32 {
        self.0.id()
    }

    pub fn kill(&mut self) -> Result<()> {
        let _ = self.0.kill();
        let _ = self.0.wait();
        Ok(())
    }

    pub fn wait(&mut self) -> Result<std::process::ExitStatus> {
        self.0.wait().context("Failed to wait on child")
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
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

/// Spawn test-e2e process in background with specified mode
pub fn spawn_test_e2e(root_dir: &Path, mode: &str) -> Result<ProcessGuard> {
    cleanup_lockfiles()?;

    // We use "cargo run" which spawns the actual binary.
    // Note: Cargo itself spawns a child. If we kill cargo, the child might persist unless we handle signals.
    // For E2E tests, usually it's better to build first then spawn binary directly if possible,
    // but `cargo run` is convenient for dev.
    // `ProcessGuard` will kill `cargo`, which *should* propagate or at least we hope so.
    // Ideally we build the binary and run it directly.

    // For now, mirroring `testlib` behavior but wrapping in ProcessGuard.
    let child = Command::new("cargo")
        .current_dir(root_dir)
        .args(["run", "-p", "test-e2e", "--", "--mode", mode])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to spawn test-e2e")?;

    Ok(ProcessGuard(child))
}

/// Spawn standalone process in background with fixed FPS
/// Spawn standalone process in background with specified arguments
pub fn spawn_standalone(root_dir: &Path, args: &[&str]) -> Result<ProcessGuard> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(root_dir)
        .args(["run", "-p", "standalone", "--"]);

    cmd.args(args);

    let child = cmd
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to spawn standalone")?;

    Ok(ProcessGuard(child))
}

/// Wait for lockfile to appear and return its path
pub async fn wait_for_lockfile(pid: u32, timeout: Duration) -> Result<PathBuf> {
    let start = Instant::now();
    let temp_dir = std::env::temp_dir();
    // Match specific PID lockfiles for splug (standalone) or test-e2e
    let patterns = [
        format!("splug_pid_{}.json", pid),
        format!("test_e2e_pid_{}.json", pid),
    ];

    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("Timeout waiting for lockfile for PID {}", pid);
        }

        for name in &patterns {
            let path = temp_dir.join(name);
            if path.exists() {
                return Ok(path);
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
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

/// Capture screenshot from process via RPC
pub async fn capture_screenshot(
    client: &mut debug_server::DebugControlClient<tonic::transport::Channel>,
) -> Result<image::RgbaImage> {
    let response = client
        .capture_screen(debug_server::Empty {})
        .await
        .context("RPC capture failed")?;
    let inner = response.into_inner();

    image::RgbaImage::from_raw(inner.width, inner.height, inner.data)
        .context("Failed to create image buffer")
}

/// Send quit command via RPC
pub async fn quit_process(
    client: &mut debug_server::DebugControlClient<tonic::transport::Channel>,
) -> Result<()> {
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Quit(
                debug_server::debug_control::QuitMsg {},
            )),
        })
        .await
        .context("Failed to send quit")?;
    Ok(())
}

/// Verify an actual image against a golden reference.
pub fn verify_golden(actual_img: &image::RgbaImage, golden_path: &Path) {
    // Determine platform suffix
    let suffix = if cfg!(target_os = "macos") {
        ".macos.png"
    } else if cfg!(target_os = "linux") {
        ".linux.png"
    } else if cfg!(target_os = "windows") {
        ".windows.png"
    } else {
        ".png"
    };

    // Construct platform-specific golden path
    let mut golden_path_with_suffix = golden_path.to_path_buf();
    if let Some(ext) = golden_path.extension() {
        if ext == "png" {
            let file_stem = golden_path.file_stem().unwrap().to_str().unwrap();
            golden_path_with_suffix =
                golden_path.with_file_name(format!("{}{}", file_stem, suffix));
        }
    }

    let has_golden = golden_path_with_suffix.exists();
    let golden_img_opt = if has_golden {
        Some(
            image::open(&golden_path_with_suffix)
                .expect("Failed to open golden image")
                .to_rgba8(),
        )
    } else {
        None
    };

    let mismatch = if let Some(golden_img) = &golden_img_opt {
        if actual_img.dimensions() != golden_img.dimensions() {
            Some(format!(
                "Dimensions mismatch: actual {:?} vs golden {:?}",
                actual_img.dimensions(),
                golden_img.dimensions()
            ))
        } else {
            let mut diff_pixels = 0;
            for (x, y, pixel) in actual_img.enumerate_pixels() {
                if pixel != golden_img.get_pixel(x, y) {
                    diff_pixels += 1;
                }
            }
            if diff_pixels > 0 {
                Some(format!("Pixel mismatch count: {}", diff_pixels))
            } else {
                None
            }
        }
    } else {
        Some("Golden image missing".to_string())
    };

    if let Some(reason) = mismatch {
        let file_name = golden_path_with_suffix
            .file_name()
            .unwrap_or(std::ffi::OsStr::new("unknown.png"));
        let file_name_str = file_name.to_string_lossy();

        // Find project root (search for Cargo.lock)
        let mut root_dir = std::env::current_dir().unwrap();
        while !root_dir.join("Cargo.lock").exists() {
            if !root_dir.pop() {
                // Fallback to current dir
                root_dir = std::env::current_dir().unwrap();
                break;
            }
        }

        let target_dir = root_dir.join("target/golden_updates");
        std::fs::create_dir_all(&target_dir).unwrap();
        let actual_path = target_dir.join(file_name_str.as_ref());

        actual_img
            .save(&actual_path)
            .expect("Failed to save actual image");

        // Try to get absolute paths for nicer output
        let golden_abs = if golden_path_with_suffix.is_absolute() {
            golden_path_with_suffix.clone()
        } else {
            std::env::current_dir()
                .unwrap()
                .join(&golden_path_with_suffix)
        };

        println!("\n❌ Golden Verification Failed: {}", reason);
        println!("Golden Path: file://{}", golden_abs.display());
        println!("Actual Path: file://{}", actual_path.display());

        // Use host path if provided (for Docker runs)
        let suggest_actual = if let Ok(host_path) = std::env::var("SPLUG_GOLDEN_HOST_PATH") {
            PathBuf::from(host_path).join(file_name_str.as_ref())
        } else {
            actual_path
        };

        // For the destination, try to make it relative to project root for copy-paste friendliness
        let suggest_dest = if let Ok(rel) = golden_path_with_suffix.strip_prefix(&root_dir) {
            rel.to_path_buf()
        } else {
            golden_path_with_suffix
        };

        println!("\n📋 To update the golden image, run:\n");
        println!(
            "cp \"{}\" \"{}\"",
            suggest_actual.display(),
            suggest_dest.display()
        );
        println!("\nThen view the diff to verify it's correct.\n");

        panic!("Golden verification failed. See output above.");
    } else {
        println!(
            "✅ Golden verification passed for {}",
            golden_path.display()
        );
    }
}
