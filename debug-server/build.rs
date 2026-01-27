use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Find definitions directory relative to the crate root
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let definitions_dir = root.join("../definitions");
    let proto_file = definitions_dir.join("debug_control.proto");

    // Check if proto file exists
    if !proto_file.exists() {
        println!(
            "cargo:warning=Proto file not found at: {}",
            proto_file.display()
        );
        return Ok(());
    }

    // Tell Cargo to rerun this script if the proto file changes
    println!("cargo:rerun-if-changed={}", proto_file.display());

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&[proto_file], &[definitions_dir])?;

    Ok(())
}
