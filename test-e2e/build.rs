use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Only build pluginval on macOS for now as per current setup
    if env::var("CARGO_CFG_TARGET_OS").unwrap() != "macos" {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap();
    let pluginval_source = workspace_root.join("extern/pluginval");
    let build_dir = workspace_root.join("target/pluginval-debug");
    let binary_path =
        build_dir.join("pluginval_artefacts/Debug/pluginval.app/Contents/MacOS/pluginval");

    if !pluginval_source.exists() {
        println!(
            "cargo:warning=pluginval source not found at {:?}, skipping build.",
            pluginval_source
        );
        return;
    }

    // Check if binary already exists to avoid rebuilding on every run unless source changes
    // Ideally we'd use rerun-if-changed on the source dir, but that's expensive.
    if binary_path.exists() {
        // Simple check: if it exists, assume it's good.
        // User can 'cargo clean' to force rebuild.
        return;
    }

    println!("cargo:warning=Building pluginval (Debug)... this may take a while.");

    // Configure CMake
    let status = Command::new("cmake")
        .arg("-S")
        .arg(&pluginval_source)
        .arg("-B")
        .arg(&build_dir)
        .arg("-DCMAKE_BUILD_TYPE=Debug")
        .status()
        .expect("Failed to run cmake configure");

    if !status.success() {
        panic!("pluginval cmake configure failed");
    }

    // Build
    let status = Command::new("cmake")
        .arg("--build")
        .arg(&build_dir)
        .arg("--config")
        .arg("Debug")
        .arg("--parallel")
        .arg("4") // Use parallel build
        .status()
        .expect("Failed to run cmake build");

    if !status.success() {
        panic!("pluginval build failed");
    }
}
