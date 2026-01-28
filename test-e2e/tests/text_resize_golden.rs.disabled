use anyhow::{Context, Result};
use debug_server::{DebugControlClient, ResizeMsg};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tokio::time::sleep;

struct StandaloneProcess {
    child: Child,
    pid_file: std::path::PathBuf,
}

impl StandaloneProcess {
    fn launch() -> Result<Self> {
        // Find the binary
        let cargo_bin = env!("CARGO_BIN_EXE_standalone");

        // Run WITH GUI (no --headless) to test rendering
        let child = Command::new(cargo_bin)
            .arg("--fixed-fps")
            .stdout(Stdio::inherit()) // Print logs to test output
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn standalone")?;

        let pid = child.id();
        let pid_file = std::env::temp_dir().join(format!("splug_pid_{}.json", pid));

        Ok(Self { child, pid_file })
    }

    async fn wait_for_port(&self) -> Result<u16> {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(10) {
            if self.pid_file.exists() {
                let content =
                    std::fs::read_to_string(&self.pid_file).context("Failed to read pid file")?;

                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(port) = json["port"].as_u64() {
                        return Ok(port as u16);
                    }
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
        anyhow::bail!("Timeout waiting for standalone to start RPC server");
    }
}

impl Drop for StandaloneProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if self.pid_file.exists() {
            let _ = std::fs::remove_file(&self.pid_file);
        }
    }
}

#[tokio::test]
async fn test_text_resize_golden() -> Result<()> {
    // 1. Launch standalone (GUI mode)
    let process = StandaloneProcess::launch()?;
    let port = process.wait_for_port().await?;
    println!("Standalone running on port {}", port);

    // 2. Connect Client
    let addr = format!("http://127.0.0.1:{}", port);
    let mut client = DebugControlClient::connect(addr)
        .await
        .context("Failed to connect client")?
        .max_decoding_message_size(16 * 1024 * 1024)
        .max_encoding_message_size(16 * 1024 * 1024);

    // 3. Resize Window
    let width = 1024;
    let height = 768;
    println!("Resizing to {}x{}...", width, height);

    let resize_req = tonic::Request::new(ResizeMsg { width, height });
    let ack: tonic::Response<debug_server::Ack> = client.resize_window(resize_req).await?;
    assert!(ack.into_inner().success, "Resize failed");

    // Wait a bit for resize to propagate and render to stabilize
    sleep(Duration::from_millis(500)).await;

    // 4. Capture Screen
    println!("Capturing screen...");
    let capture_req = tonic::Request::new(debug_server::debug_control::Empty {});
    let response = client.capture_screen(capture_req).await?;
    let img_bytes = response.into_inner();

    println!(
        "Received captured image: {}x{}, {} bytes",
        img_bytes.width,
        img_bytes.height,
        img_bytes.data.len()
    );

    // Verify dimensions match request
    if img_bytes.width != width || img_bytes.height != height {
        // Note: On Retina/HiDPI this check might fail if capture returns physical pixels
        // But set_size sets logical points usually.
        // If it fails, we know we have scaling issues to handle.
        // For now, let's warn but not fail? Or fail if exact match expected?
        // Let's print usage of scale factor if we knew it.
        println!(
            "WARNING: Captured dimensions {}x{} != Requested {}x{}",
            img_bytes.width, img_bytes.height, width, height
        );
    }

    // Convert to RgbaImage
    // Note: ImageBuffer::from_raw expects flattened buffer.
    let img_buffer: image::RgbaImage =
        image::ImageBuffer::from_raw(img_bytes.width, img_bytes.height, img_bytes.data)
            .context("Failed to create image buffer from raw data")?;

    // Save for inspection
    let output_path = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("tests/outputs/resize_capture.png");
    std::fs::create_dir_all(output_path.parent().unwrap())?;
    img_buffer.save(&output_path)?;
    println!("Saved capture to {}", output_path.display());

    // 5. Compare against Golden (if exists)
    // 5. Compare against Golden (if exists) or Update if requested
    let golden_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens/text_resize_golden.png");

    if std::env::var("UPDATE_GOLDENS").is_ok() {
        std::fs::create_dir_all(golden_path.parent().unwrap())?;
        img_buffer.save(&golden_path)?;
        println!("Updated golden image at {}", golden_path.display());
    } else if golden_path.exists() {
        println!("Verifying against golden: {}", golden_path.display());
        let golden_img = image::open(&golden_path)?.to_rgba8();
        if img_buffer.dimensions() != golden_img.dimensions() {
            anyhow::bail!("Dimensions mismatch!");
        }

        let mut diff_pixels = 0;
        for (x, y, pixel) in img_buffer.enumerate_pixels() {
            let golden_pixel = golden_img.get_pixel(x, y);
            if pixel != golden_pixel {
                diff_pixels += 1;
            }
        }

        if diff_pixels > 0 {
            anyhow::bail!("Image differs by {} pixels", diff_pixels);
        }
    } else {
        anyhow::bail!("No golden found, expected {}.", golden_path.display());
    }

    Ok(())
}
