# splug - High-Performance Audio Plugin

A metal-down VST3/CLAP audio plugin built with Rust and Vulkan.

## Architecture

- **PAL (Platform Abstraction Layer)** - Native windowing without `winit`
- **GUI** - Shared interface components (event handling, UI state)
- **Render** - Vulkan graphics with SVG rasterization
- **Core** - Lock-free audio processing and state management

## Building

### Prerequisites

- Rust toolchain (stable)
- Vulkan SDK (for shader compilation with `glslc`)
  - macOS: `brew install vulkan-tools` or download from [LunarG](https://vulkan.lunarg.com/)
  - Windows: Install from [LunarG Vulkan SDK](https://vulkan.lunarg.com/)
  - The build system will auto-detect glslc in standard locations including `~/.local/VulkanSDK`

### Build Commands

```bash
# Build all workspace crates
cargo build

# Run tests (5 tests passing)
cargo test
# (On golden test failure, follow the printed `cp` command to update golden images)

# Build and bundle the plugin (with shader compilation)
cargo run --package xtask -- bundle

# Run standalone (with GUI, when implemented)
cargo run --bin standalone

# Run standalone in headless mode
cargo run --bin standalone -- --headless

# Run code coverage analysis
cargo run --package xtask -- coverage

# Verify coverage (fails if < 80%)
cargo run --package xtask -- coverage --verify

# Run linting (clippy) with strict settings
cargo run --package xtask -- lint
```

### Bundle Output

After running `cargo run --package xtask -- bundle`, the plugin will be located at:

- **macOS**: `target/bundled/splug.vst3/`
- **Windows**: `target/bundled/splug.vst3`

Compiled shaders: `target/shaders/*.spv`

## Development Status

**Phase 1: Build System & Skeleton** - Complete
- Workspace structure with 6 crates
- cargo-xtask build automation
- Shader compilation pipeline (auto-detects Vulkan SDK)
- Platform-specific bundle generation
- macOS codesigning
- PAL and GUI crate skeletons

**Phase 2: Platform Abstraction Layer** - Complete
- Native windowing without winit (macOS/Cocoa)
- Parent-child window attachment
- High-performance event loop (CVDisplayLink)

**Phase 3: Graphics Core** - Complete
- Vulkan rendering engine
- SDF-based 2D shape rendering (Rect, Circle, RoundedRect)
- GPU-accelerated text rendering (Fontdue + Glyph Packing)
- Golden image testing infrastructure

**Phase 3.5: Documentation & Hardening** - Complete
- Comprehensive docstrings
- Strict linting (0 warnings)
- Code coverage infrastructure (>80%)

**Phase 4: Core Logic & Persistence** - Next
**Phase 5: Test Harness (RPC)** - Planned

## Project Structure

```
rust-vst/
├── plugin/              # Core plugin library (cdylib)
│   └── src/
│       ├── lib.rs       # VST3 entry points with panic safety
│       ├── pal/         # Platform Abstraction Layer (legacy, moved to pal crate)
│       ├── render/      # Vulkan rendering (Phase 3)
│       ├── core/        # Audio processing (Phase 4)
│       └── shaders/     # GLSL shaders
├── pal/                 # Platform Abstraction Layer library
│   └── src/
│       ├── lib.rs       # NativeWindow trait
│       ├── macos.rs     # macOS implementation (Phase 2)
│       └── win32.rs     # Windows implementation (Phase 2)
├── gui/                 # Shared GUI framework
│   └── src/lib.rs       # UIEvent types, GuiContext (Phase 2-3)
├── standalone/          # Standalone host for testing
│   └── src/main.rs
├── xtask/               # Build automation
│   └── src/main.rs      # Shader compilation, bundling, codesigning
└── definitions/         # Protobuf schemas
    └── src/lib.rs       # State and RPC definitions (Phase 4-5)
```

## Real-Time Safety Constraints

This project follows strict real-time audio programming rules:

- No allocations on the audio thread (`Box`, `Vec`, `String`)
- No mutex locking on the audio thread
- Use `rtrb` or atomics for parameter communication
- All FFI entry points wrapped in `catch_unwind` for panic safety

## License

MIT OR Apache-2.0
