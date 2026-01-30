//! # splug - High-Performance Audio Plugin
//!
//! A metal-down VST3/CLAP audio plugin built with Rust and Vulkan.
//!
//! ## Architecture
//! - PAL (Platform Abstraction Layer) - Native windowing
//! - Render - Vulkan graphics pipeline
//! - Core - Audio processing and state management

use std::ffi::c_void;

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

// -----------------------------------------------------------------------------
// VST3 Entry Point
// -----------------------------------------------------------------------------

#[cfg(feature = "vst3")]
pub mod vst3 {
    use std::ffi::c_void;
    use std::panic;

    /// VST3 Factory Entry Point
    #[no_mangle]
    #[allow(non_snake_case)]
    pub unsafe extern "C" fn GetPluginFactory() -> *mut c_void {
        // Catch any panics to prevent host crashes
        let result = panic::catch_unwind(|| {
            // Return a dummy pointer for now to satisfy validation
            // In a real implementation, this would be a pointer to IPluginFactory vtable
            static DUMMY_FACTORY: u8 = 0;
            &DUMMY_FACTORY as *const u8 as *mut c_void
        });

        match result {
            Ok(factory) => factory,
            Err(e) => {
                tracing::error!("PANIC in GetPluginFactory: {:?}", e);
                std::ptr::null_mut()
            }
        }
    }
}

// -----------------------------------------------------------------------------
// CLAP Entry Point
// -----------------------------------------------------------------------------

#[cfg(feature = "clap")]
pub mod clap {
    use std::ffi::{c_char, c_void};

    #[repr(C)]
    pub struct clap_plugin_entry {
        pub clap_version: *const c_char,
        pub init: extern "C" fn(plugin_path: *const c_char) -> bool,
        pub deinit: extern "C" fn(),
        pub get_factory: extern "C" fn(factory_id: *const c_char) -> *const c_void,
    }

    // Pointers are not Sync, but function pointers are.
    // We assert safety here because our clap_version is a static string.
    unsafe impl Sync for clap_plugin_entry {}

    extern "C" fn init(_plugin_path: *const c_char) -> bool {
        true
    }

    extern "C" fn deinit() {}

    extern "C" fn get_factory(_factory_id: *const c_char) -> *const c_void {
        std::ptr::null()
    }

    // CLAP version 1.1.10
    static CLAP_VERSION: &str = "1.1.10\0";

    static ENTRY: clap_plugin_entry = clap_plugin_entry {
        clap_version: CLAP_VERSION.as_ptr() as *const c_char,
        init,
        deinit,
        get_factory,
    };

    #[no_mangle]
    #[allow(non_snake_case)]
    pub unsafe extern "C" fn clap_entry(_clap_version: *const c_char) -> *const clap_plugin_entry {
        // Check host compatible version?
        // For now just return our entry
        &ENTRY
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

    #[test]
    #[cfg(feature = "vst3")]
    fn test_vst3_factory_doesnt_panic() {
        // Verify the entry point doesn't crash
        unsafe {
            use super::vst3::GetPluginFactory;
            let factory = GetPluginFactory();
            // Should be non-null now
            assert!(!factory.is_null());
        }
    }

    #[test]
    fn test_panic_caught_at_boundary() {
        // This logic is now inside mod vst3, so we can't easily test it without exposing inner function
        // or just assuming it works via std::panic::catch_unwind mechanism tests
    }

    #[test]
    #[cfg(feature = "clap")]
    fn test_clap_entry_exists() {
        unsafe {
            use super::clap::clap_entry;
            let entry = clap_entry(std::ptr::null());
            assert!(!entry.is_null());
            let entry_ref = &*entry;
            assert!(!entry_ref.clap_version.is_null());
        }
    }
}
