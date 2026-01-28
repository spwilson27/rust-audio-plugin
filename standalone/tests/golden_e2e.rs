use std::path::{Path, PathBuf};
use std::process::Command;

#[tokio::test]
async fn test_golden_image_verification() {
    // Locate the project root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap(); // rust-vst root

    // Path to golden image
    let golden_path = manifest_dir.join("tests/goldens/standalone_screenshot.png");

    // Ensure golden exists before running
    if !golden_path.exists() {
        panic!(
            "Golden image not found at: {}\nRun 'cargo xtask goldens' to generate it.",
            golden_path.display()
        );
    }

    println!("Verifying against golden: {}", golden_path.display());

    // Build standalone first to ensure it's fresh
    let status = Command::new("cargo")
        .current_dir(root_dir)
        .args(["build", "-p", "standalone"])
        .status()
        .expect("Failed to build standalone");
    assert!(status.success());

    // Use testlib to capture screenshot
    let img = testlib::capture_golden(root_dir)
        .await
        .expect("Failed to capture golden");

    // Verify against golden
    verify_golden(&img, &golden_path);
}

fn verify_golden(current_img: &image::RgbaImage, golden_path: &Path) {
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
        let _ = current_img.save("test_failure.png");
        panic!(
            "Images differ by {} pixels. Saved test_failure.png",
            diff_pixels
        );
    }
}
