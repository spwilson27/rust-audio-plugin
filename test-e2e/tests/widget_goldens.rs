//! Widget showcase golden test
//!
//! Verifies that all 4 widgets render correctly with text

use image::RgbaImage;
use std::path::{Path, PathBuf};
use std::process::Command;

#[tokio::test]
async fn test_widget_showcase_golden() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap();

    // Path to golden image
    let golden_path = manifest_dir.join("goldens/widgets_showcase.png");

    // Skip if golden doesn't exist yet
    if !golden_path.exists() {
        println!(
            "SKIPPING: Golden image not found at: {}",
            golden_path.display()
        );
        println!("Run 'cargo xtask goldens' to generate it.");
        return;
    }

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
    verify_golden(&img, &golden_path);
}

fn verify_golden(current_img: &RgbaImage, golden_path: &Path) {
    let golden_img = image::open(golden_path)
        .expect("Failed to open golden image")
        .to_rgba8();

    assert_eq!(
        current_img.dimensions(),
        golden_img.dimensions(),
        "Dimensions mismatch"
    );

    let mut diff_pixels = 0;
    for (x, y, pixel) in current_img.enumerate_pixels() {
        let golden_pixel = golden_img.get_pixel(x, y);
        if pixel != golden_pixel {
            diff_pixels += 1;
        }
    }

    if diff_pixels > 0 {
        // Save failure image
        let _ = current_img.save("test_failure_widgets.png");
        panic!(
            "Images differ by {} pixels. Saved test_failure_widgets.png",
            diff_pixels
        );
    }
}
