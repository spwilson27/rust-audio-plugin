# Splug AI Specification (spec.md)

This document serves as the primary context for AI agents working on the **splug** audio plugin project. It outlines the project's architecture, core constraints, and development workflows.

## Project Mission
**splug** is a high-performance audio plugin framework (VST3/CLAP) built with a "metal-down" approach. We prioritize performance and direct system control over convenience. Specifically, we **do not** use generic GUI frameworks like `egui` or `winit` because they fail to handle the complex parent-child windowing requirements of professional DAWs.

## Architecture

### Component Dependency Graph

```mermaid
graph TD
    standalone --> pal
    standalone --> gui
    standalone --> debug-server
    plugin --> audio-core
    plugin --> gui
    gui --> pal
    gui --> testlib
    test-e2e --> testlib
    
- **[audio-core](audio-core)**: Low-level DSP and audio processing.
  - **Architecture (Planned)**:
    - Lock-free parameter synchronization using `rtrb` (Ring Buffer).
    - Audio thread never blocks or allocates.
    - State changes are pushed from GUI -> Audio via atomic queues.
    - Audio thread can perform atomic reads of atomic-sized parameters.
- **[pal](pal)**: Platform Abstraction Layer. Handles native windowing, audio, and input events (Cocoa on macOS, Win32 on Windows).
- **[gui](gui)**: Rendering and widget system built on Vulkan (`ash`).
  - **Render Loop**: Driven by `CVDisplayLink` on macOS and vsync-aligned callbacks on Linux.
  - **Integration**: Renderer integration is complete. Widgets draw directly to Vulkan command buffers.
- **[standalone](standalone)**: A binary that hosts the plugin UI.
- **[plugin](plugin)**: A library that implements the VST3 and CLAP plugin interfaces.
- **[xtask](xtask)**: Build automation for shader compilation, bundling, and Docker orchestration.
- **[test-e2e](test-e2e)**: End-to-end tests using RPC and Golden images.

### State Management (Planned)
- **Schema**: All persistent state is defined in Protobuf messages.
- **Global Config State**: `redb` (embedded database) is used for robust, ACID-compliant read/write synchronization across active plugin instances to the shared config save file.
- **VST/Plugin Instance State**: Plugin state will be stored in the VST3/CLAP state chunk for persistence across plugin loads. This will be serialized into a protobuf message.

### 🔗 Inter-Process Communication (RPC)
For testing, we use a custom RPC protocol to communicate between the test runner and the hosted plugin. The plugin supports generic events either from the debug server or from the native PAL. This allows us to mock events in tests without needing to directly interact with the GUI.
- **Schema**: Defined in [definitions/proto](definitions/proto).
- **Capabilities**:
  - Inject input events (Mouse, Keyboard).
  - Query widget state.
  - Capture screenshots for golden verification.

## Core Constraints (Non-Negotiable)

1. **Real-Time Safety**: No allocations (`Box`, `Vec`, `String`) or mutex locking on the audio thread. Use `rtrb` or atomics for communication.
2. **Panic Safety**: The C-ABI boundary (VST3/CLAP entry points) must capture panics using `catch_unwind` to prevent crashing the host DAW.
3. **No Bloat**: Avoid heavy dependencies. We use `ash` for Vulkan and `objc2` for macOS APIs.
4. **No Emoji**: Do not use emojis in documentation or comments.

## Testing Workflow

### Golden Testing
We use pixel-perfect golden testing. 
- **Platform Specificity**: Rendering differs by OS. We maintain `*.macos.png` and `*.linux.png` baselines.
- **Updates**: If a visual change is intentional, follow the `cp` command suggested in the test failure output to update the repository.

### Docker (Linux)
- Use `cargo xtask test --docker` to run tests in a headless Linux environment.
- This ensures the Vulkan/X11 stack works in a clean, reproducible container.

### VirtualBox (macOS) - *Planned*
- We are moving away from Docker-OSX due to Apple Silicon limitations and towards automated VirtualBox VMs for macOS E2E testing.

## Guidance for AI Agents

### Before Implementing:
- Check `architecture_v2.md` for deep design rationale.
- Ensure any new UI features are added to `test-e2e` and verified on both macOS and Linux.
- Review `prompts.md` for historical context on recent feature requests.

### Code Style:
- Prefer explicit over implicit.
- Use `unsafe` blocks only where necessary for FFI/performance, and always document the safety rationale.
- Every module must have a `test` module.
- Any new features at minimum should require a new unit test. Ideally, new features will include an end-to-end test in the test-e2e crate.
- After any changes, run the full suite of tests using the  `cargo xtask test-all`, ensure they are all passing.
- After any changes, ensure formating is correct using `cargo fmt`

### Common Pitfalls to Avoid:
- **Metal Layer Over-Presenting**: On macOS, do not call `[CAMetalLayerDrawable texture]` after already presenting. Use `nextDrawable`.
- **Scaling/DPI**: Always account for `get_scale_factor()` in physical pixel calculations.
- **Coordinate Systems**: Ensure RPC coordinates match the physical/logical pixel expectations of the PAL.

---
> [!NOTE]
> This project is a "metal-down" engineering effort. When in doubt, prefer lower-level system APIs over high-level abstractions.
