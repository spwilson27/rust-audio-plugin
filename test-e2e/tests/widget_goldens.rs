//! Widget showcase golden test
//!
//! Verifies that all 4 widgets render correctly with text

use std::path::PathBuf;
use std::process::Command;

#[tokio::test]
async fn test_widget_showcase_golden() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap();

    // Path to golden image
    let golden_path = manifest_dir.join("goldens/widgets_showcase.png");

    println!("Verifying against golden: {}", golden_path.display());

    // Build test-e2e
    let status = Command::new("cargo")
        .current_dir(root_dir)
        .args(["build", "-p", "test-e2e"])
        .status()
        .expect("Failed to build test-e2e");
    assert!(status.success());

    // Capture via testlib
    let img = testlib::capture_test_e2e_golden(root_dir, "widgets")
        .await
        .expect("Failed to capture golden");

    // Verify against golden
    testlib::verify_golden(&img, &golden_path);
}
