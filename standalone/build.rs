// Build script for standalone binary
// Links required system frameworks on macOS

fn main() {
    // On macOS, link AppKit framework for NSApplication/NSWindow
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=AppKit");
    }
}
