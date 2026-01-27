//! # splug - High-Performance Audio Plugin
//!
//! A metal-down VST3/CLAP audio plugin built with Rust and Vulkan.
//!
//! ## Architecture
//! - PAL (Platform Abstraction Layer) - Native windowing
//! - Render - Vulkan graphics pipeline
//! - Core - Audio processing and state management

use std::ffi::c_void;
use std::panic;

// Module declarations for future phases
pub mod pal {
    //! Platform Abstraction Layer
    //! Handles native window creation and management
}

pub mod render {
    //! Vulkan rendering backend
    //! SVG rasterization and GPU compositing
}

pub mod core {
    //! Audio processing and state management
    //! Lock-free parameter communication
}

/// VST3 Factory Entry Point
///
/// This is the main entry point called by the VST3 host to get the plugin factory.
/// We wrap it in catch_unwind to prevent panics from crashing the host DAW.
/// # Safety
///
/// This function is the entry point for the VST3 host. It must be called by a VST3-compatible host.
/// The returned pointer must be a valid `IPluginFactory` interface.
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn GetPluginFactory() -> *mut c_void {
    // Catch any panics to prevent host crashes
    let result = panic::catch_unwind(|| {
        // TODO: Return actual VST3 factory in Phase 4
        // For now, return null to indicate "not yet implemented"
        std::ptr::null_mut()
    });

    match result {
        Ok(factory) => factory,
        Err(e) => {
            // Log the panic (in a real implementation, write to file)
            eprintln!("PANIC in GetPluginFactory: {:?}", e);
            std::ptr::null_mut()
        }
    }
}

/// Module entry point (Windows)
/// Required for DLL initialization on Windows
#[cfg(target_os = "windows")]
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn DllMain(
    _hinst_dll: *mut c_void,
    _fdw_reason: u32,
    _lpv_reserved: *mut c_void,
) -> i32 {
    1 // TRUE
}

/// Bundle entry point (macOS)
/// Called when the bundle is loaded on macOS
/// # Safety
///
/// Called by the host when the bundle is loaded.
#[cfg(target_os = "macos")]
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn bundleEntry(_bundle: *mut c_void) -> bool {
    true
}

/// Bundle exit point (macOS)
/// Called when the bundle is unloaded on macOS
/// # Safety
///
/// Called by the host when the bundle is unloaded.
#[cfg(target_os = "macos")]
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn bundleExit() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_factory_doesnt_panic() {
        // Verify the entry point doesn't crash
        unsafe {
            let factory = GetPluginFactory();
            // For now it should return null (not implemented)
            assert!(factory.is_null());
        }
    }

    #[test]
    fn test_panic_caught_at_boundary() {
        // Simulate a panic inside the entry point
        let result = panic::catch_unwind(|| {
            panic!("Test panic");
        });

        assert!(result.is_err());
    }
}
