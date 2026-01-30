use std::env;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Only build pluginval on macOS for now
    if env::var("CARGO_CFG_TARGET_OS").unwrap() != "macos" {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap();
    let pluginval_source = workspace_root.join("extern/pluginval");

    if !pluginval_source.join("CMakeLists.txt").exists() {
        println!(
            "cargo:warning=pluginval source not found at {:?}, skipping build.",
            pluginval_source
        );
        return;
    }

    println!("cargo:warning=Building pluginval (Debug) using cmake crate...");

    // Build using cmake crate
    let dst = cmake::Config::new(&pluginval_source)
        .build_target("pluginval")
        .profile("Debug")
        // .generator("Xcode") // Optional: Use Xcode if needed, but Ninja/Make usually works
        .build();

    println!("cargo:warning=pluginval build complete. Artifacts in {:?}", dst);

    // Find the built app
    // cmake crate usually installs to `dst`.
    // If no install target, it's in `dst/build`.
    // JUCE apps often end up in `.../pluginval_artefacts/Debug/pluginval.app` or similar.
    
    // We will search for pluginval.app in dst
    let app_path = find_pluginval_app(&dst).or_else(|| {
        // Fallback: look in the source directory if it was an in-source build (unlikely with cmake crate)
        // Or check common JUCE output paths
        let common = dst.join("build/pluginval_artefacts/Debug/pluginval.app");
        if common.exists() { Some(common) } else { None }
    });

    if let Some(src_app) = app_path {
        // Copy to a stable location for tests
        let target_base = workspace_root.join("target/pluginval-debug");
        let dest_app = target_base.join("pluginval_artefacts/Debug/pluginval.app");
        
        // Ensure parent dirs exist
        if let Some(parent) = dest_app.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }

        println!("cargo:warning=Copying pluginval to {:?}", dest_app);
        
        // Recursive copy
        if dest_app.exists() {
            std::fs::remove_dir_all(&dest_app).unwrap();
        }
        copy_dir_recursive(&src_app, &dest_app).expect("Failed to copy pluginval app");
    } else {
         println!("cargo:warning=Could not locate pluginval.app in {:?}", dst);
         // Panic or warn? Panic ensures we know it failed.
         panic!("pluginval build succeeded but artifact not found.");
    }
}

fn find_pluginval_app(root: &Path) -> Option<PathBuf> {
    // Simple recursive search for pluginval.app
    if root.is_dir() {
        for entry in std::fs::read_dir(root).ok()? {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("app") 
               && path.file_stem().and_then(|s| s.to_str()) == Some("pluginval") {
                return Some(path);
            }
            if path.is_dir() {
                if let Some(found) = find_pluginval_app(&path) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy_dir_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}