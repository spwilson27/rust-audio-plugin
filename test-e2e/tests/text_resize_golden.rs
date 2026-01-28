use anyhow::Result;
use debug_server::ResizeMsg;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_text_resize_golden() -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap();
    let golden_path = manifest_dir.join("goldens/text_resize_golden.png");

    // Skip if golden doesn't exist (unless we are updating)?
    // verify_golden handles missing golden by failing and providing cp command.
    // But initially we might want to allow running it.

    // Ensure test-e2e is built?
    // `cargo run` inside spawn_test_e2e handles building.

    println!("Starting test-e2e in text-resize mode...");
    let mut child = testlib::spawn_test_e2e(root_dir, "text-resize")?;

    // Wait for RPC
    println!("Waiting for lockfile...");
    let lockfile = match testlib::wait_for_lockfile(Duration::from_secs(10)).await {
        Ok(l) => l,
        Err(e) => {
            let _ = child.kill();
            return Err(e);
        }
    };

    let info = testlib::parse_lockfile(&lockfile)?;
    println!("Connected to test-e2e on port {}", info.port);

    let mut client = match testlib::connect_rpc(info.port, Duration::from_secs(5)).await {
        Ok(c) => c,
        Err(e) => {
            let _ = child.kill();
            return Err(e);
        }
    };

    // Resize
    let width = 1024;
    let height = 768;
    println!("Resizing to {}x{}...", width, height);

    let resize_req = tonic::Request::new(ResizeMsg { width, height });
    match client.resize_window(resize_req).await {
        Ok(ack) => assert!(ack.into_inner().success, "Resize failed"),
        Err(e) => {
            let _ = child.kill();
            anyhow::bail!("RPC resize failed: {}", e);
        }
    }

    sleep(Duration::from_millis(500)).await;

    // Capture
    println!("Capturing screen...");
    let img = match testlib::capture_screenshot(&mut client).await {
        Ok(i) => i,
        Err(e) => {
            let _ = child.kill();
            return Err(e);
        }
    };

    println!("Sending Quit...");
    let _ = testlib::quit_standalone(&mut client).await;
    let _ = child.wait();

    // Verify
    testlib::verify_golden(&img, &golden_path);

    Ok(())
}
