use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_text_resize_golden() -> anyhow::Result<()> {
    let manifest_dir = test_e2e::manifest_dir();
    let root_dir = manifest_dir.parent().unwrap();
    let goldens_dir = manifest_dir.join("goldens");
    std::fs::create_dir_all(&goldens_dir).unwrap();

    let golden_path = goldens_dir.join("text_resize_golden.png");

    println!("Starting test-e2e in text-resize mode...");
    let mut child = test_e2e::spawn_test_e2e(root_dir, "text-resize")?;

    // Wait for RPC
    println!("Waiting for lockfile...");
    let lockfile = match test_e2e::wait_for_lockfile(child.id(), Duration::from_secs(10)).await {
        Ok(l) => l,
        Err(e) => {
            child.kill().ok();
            return Err(e);
        }
    };

    let info = test_e2e::parse_lockfile(&lockfile)?;
    println!("Connected to test-e2e on port {}", info.port);

    let mut client = match test_e2e::connect_rpc(info.port, Duration::from_secs(5)).await {
        Ok(c) => c,
        Err(e) => {
            child.kill().ok();
            return Err(e);
        }
    };

    // Note: To test resize, we would need to implement ResizeMsg support in `test-e2e/src/lib.rs` client extensions
    // or use the raw client. `test_e2e::connect_rpc` returns `debug_server::DebugControlClient`.
    // The previous test logic used `client.resize_window`. `DebugControl` proto likely has this.

    // Resize
    let width = 1024;
    let height = 768;
    println!("Resizing to {}x{}...", width, height);

    // We need to define ResizeMsg if not in scope or included in lib
    let resize_req = tonic::Request::new(debug_server::ResizeMsg { width, height });
    match client.resize_window(resize_req).await {
        Ok(ack) => assert!(ack.into_inner().success, "Resize failed"),
        Err(e) => {
            child.kill().ok();
            // If resize isn't implemented in the server yet (it wasn't in main.rs), this will fail.
            // Looking at `main.rs`, I didn't see `resize_window` implementation in `UIEvent::Custom` -> `GetWidgetStateRequest` etc.
            // Oh wait, `debug-server` crate might have `ResizeMsg`.
            // Let's assume for now we might fail here if server doesn't implement it.
            // But previous test code existed. Let's try.
            anyhow::bail!("RPC resize failed: {}", e);
        }
    }

    sleep(Duration::from_millis(500)).await;

    // Capture
    println!("Capturing screen...");
    let img = match test_e2e::capture_screenshot(&mut client).await {
        Ok(i) => i,
        Err(e) => {
            child.kill().ok();
            return Err(e);
        }
    };

    println!("Sending Quit...");
    let _ = test_e2e::quit_process(&mut client).await;
    let _ = child.wait();

    // Verify
    test_e2e::verify_golden(&img, &golden_path);

    Ok(())
}
