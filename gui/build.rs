use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/shaders");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let shader_dir = PathBuf::from("src/shaders");

    if !shader_dir.exists() {
        return;
    }

    for entry in fs::read_dir(shader_dir).expect("Failed to read shader directory") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();

        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy();
            if ext_str == "vert" || ext_str == "frag" {
                let file_name = path.file_name().unwrap().to_string_lossy();
                let out_name = format!("{}.spv", file_name);
                let out_path = out_dir.join(out_name);

                println!("cargo:rerun-if-changed={}", path.display());

                // Compile using glslc
                let status = Command::new("glslc")
                    .arg(&path)
                    .arg("-o")
                    .arg(&out_path)
                    .status();

                if let Ok(status) = status {
                    if !status.success() {
                        println!("cargo:warning=Failed to compile shader {}", path.display());
                    }
                } else {
                    println!("cargo:warning=glslc not found, skipping shader compilation");
                }
            }
        }
    }
}
