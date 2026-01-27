use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_golden_image_verification() {
    // Locate the project root relative to this test file
    // CARGO_MANIFEST_DIR points to standalone/ directory
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

    // Run standalone binary using cargo run
    // This ensures we use the same environment and build artifacts
    let status = Command::new("cargo")
        .current_dir(root_dir)
        .args(&[
            "run",
            "-p",
            "standalone",
            "--",
            "--test-screenshot",
            "--golden-image",
            golden_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute cargo run");

    assert!(status.success(), "Golden image verification failed!");
}
