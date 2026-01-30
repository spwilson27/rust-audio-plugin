//! Widget showcase golden test
//!
//! Verifies that all 4 widgets render correctly with text

use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_widget_showcase_golden() {
    let manifest_dir = test_e2e::manifest_dir();
    let root_dir = manifest_dir.parent().unwrap();
    let goldens_dir = manifest_dir.join("goldens");
    std::fs::create_dir_all(&goldens_dir).unwrap();

    let mut tester = test_e2e::GoldenTester::new();

    // Start App
    let mut child = test_e2e::spawn_test_e2e(root_dir, "widgets").expect("Failed to start app");

    // Connect RPC
    let lockfile_path = test_e2e::wait_for_lockfile(child.id(), Duration::from_secs(10))
        .await
        .expect("Lockfile timeout");

    let info = test_e2e::parse_lockfile(&lockfile_path).expect("Failed to parse lockfile");
    let mut client = test_e2e::connect_rpc(info.port, Duration::from_secs(5))
        .await
        .expect("RPC connection failed");

    // Wait for render
    sleep(Duration::from_millis(500)).await;

    // Capture
    println!("Capturing Widget Showcase...");
    let img = test_e2e::capture_screenshot(&mut client)
        .await
        .expect("Capture failed");

    // Verify
    tester.check(&img, &goldens_dir.join("widgets_showcase.png"));

    // Cleanup
    test_e2e::quit_process(&mut client).await.ok();
    child.kill().ok();
    tester.assert();
}
