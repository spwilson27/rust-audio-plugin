# splug - High-Performance Audio Plugin

A metal-down VST3/CLAP audio plugin built with Rust and Vulkan.

## Architecture

- **PAL (Platform Abstraction Layer)** - Native windowing without `winit`
- **Render** - Vulkan graphics with SVG rasterization
- **Core** - Lock-free audio processing and state management

## Building

### Prerequisites

- Rust toolchain (stable)
- Vulkan SDK (for shader compilation with `glslc`)
  - macOS: `brew install vulkan-tools`
  - Windows: Install from [LunarG Vulkan SDK](https://vulkan.lunarg.com/)

### Build Commands

```bash
# Build all workspace crates
cargo build --workspace

# Run tests
cargo test --workspace

# Build and bundle the plugin
cargo xtask bundle

# Run standalone (with GUI, when implemented)
cargo run --bin standalone

# Run standalone in headless mode
cargo run --bin standalone -- --headless
```

### Bundle Output

After running `cargo xtask bundle`, the plugin will be located at:

- **macOS**: `target/bundled/splug.vst3/`
- **Windows**: `target/bundled/splug.vst3`

## Development Status

✅ **Phase 1: Build System & Skeleton** - Complete
- Workspace structure with 4 crates
- cargo-xtask build automation
- Shader compilation pipeline
- Platform-specific bundle generation
- macOS codesigning

⏳ **Phase 2: Platform Abstraction Layer** - Not started
⏳ **Phase 3: Graphics Core** - Not started
⏳ **Phase 4: Core Logic & Persistence** - Not started
⏳ **Phase 5: Test Harness (RPC)** - Not started

## Project Structure

```
rust-vst/
├── plugin/              # Core plugin library (cdylib)
│   └── src/
│       ├── lib.rs       # VST3 entry points with panic safety
│       ├── pal/         # Platform Abstraction Layer (Phase 2)
│       ├── render/      # Vulkan rendering (Phase 3)
│       ├── core/        # Audio processing (Phase 4)
│       └── shaders/     # GLSL shaders
├── standalone/          # Standalone host for testing
│   └── src/main.rs
├── xtask/               # Build automation
│   └── src/main.rs
└── definitions/         # Protobuf schemas
    └── src/lib.rs
```

## Real-Time Safety Constraints

This project follows strict real-time audio programming rules:

- ❌ No allocations on the audio thread (`Box`, `Vec`, `String`)
- ❌ No mutex locking on the audio thread
- ✅ Use `rtrb` or atomics for parameter communication
- ✅ All FFI entry points wrapped in `catch_unwind` for panic safety

## License

MIT OR Apache-2.0
