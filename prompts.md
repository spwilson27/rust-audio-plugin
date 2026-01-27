Based on the Architectural Specification we developed, here is the plan to guide the implementation Agents.

### 2. Agent Persona & System Prompt

*Use this "System Prompt" as the preamble for every agent interaction to ensure consistency.*

> **Role:** You are a Principal Rust Audio Engineer specializing in `unsafe` systems programming and lock-free concurrency.
> **Project Context:** We are building "splug" a high-performance audio plugin (VST3/CLAP) using a "metal-down" approach. We **do not** use GUI frameworks like `egui` or `winit` because they fail to handle the parent-child windowing requirements of DAWs. We use raw `ash` (Vulkan) and native windowing APIs (`objc2`/`windows-sys`) for windowing.
> **Core Constraints:**
> 1. **Real-Time Safety:** No allocations (`Box`, `Vec`, `String`) or mutex locking on the audio thread. Use `rtrb` or atomics.
> 
> 
> 2. **Panic Safety:** The C-ABI boundary (VST3/CLAP entry points) must capture panics to prevent crashing the host DAW.
> 3. **Testing:** Every module must include a `test` module. If a component requires a window, create a "Headless" mock or use the RPC server for verification.
> 
> 

---

### 3. Implementation Prompts

Here is the sequential execution plan. Feed these prompts to the agent one by one.

#### Phase 1: The Build System & Skeleton

*Goal: Establish the polyglot build process immediately so we can test artifacts.*

**Prompt:**

> "Implement the project skeleton and build system using the `cargo-xtask` pattern.
> 
> 
> **Requirements:**
> 1. Create a workspace with members: `plugin` (lib), `standalone` (bin), and `xtask` (bin).
> 2. Implement a `cargo xtask bundle` command that:
> * Compiles the Rust library in `release` mode.
> * Compiles GLSL shaders in `plugin/src/shaders/` to SPIR-V using `glslc` (assume it's in the PATH) and embeds them as bytes.
> * **macOS:** Creates the `.vst3/Contents/MacOS` bundle structure, copies the dylib, and generates a valid `Info.plist`.
> 
> 
> * **Windows:** Renames the `.dll` to `.vst3`.
> 
> 
> 3. **Test Plan:** Create a dummy `lib.rs` that exports a basic VST3 entry point. Running `cargo xtask bundle` should produce a folder structure recognized by a VST3 host (like Reaper or Bitwig)."
> 
> 

#### Phase 2: The Platform Abstraction Layer (PAL)

*Goal: Create the windowing hooks without using `winit`.*

**Prompt:**

> "Implement the Platform Abstraction Layer (PAL) in `src/pal/`. This module handles the 'Parenting' handshake with the host.
> **Requirements:**
> 1. Define a trait `NativeWindow`:
> * `fn attach(parent: *mut c_void) -> Self`
> * `fn get_raw_handle(&self) -> RawWindowHandle` (using `raw-window-handle` crate).
> * `fn set_size(&mut self, width: u32, height: u32)`.
> 
> 
> 2. **macOS Implementation (`pal/macos.rs`):**
> * Use `objc2` to declare a class `RustPluginView` inheriting from `NSView`.
> * Implement `viewDidMoveToWindow` to detect when the UI is opened/closed.
> * Add a `CAMetalLayer` as the backing layer for Vulkan support.
> 
> 
> 3. **Windows Implementation (`pal/win32.rs`):**
> * Use `windows-sys`. Create a window class with `CS_HREDRAW | CS_VREDRAW`.
> * Handle `WM_SIZE` to trigger swapchain rebuilds.
> * **Crucial:** Handle `WM_ERASEBKGND` by returning 1 to prevent flickering during resize.
> 
> 
> 
> 
> 4. **Test Plan:** Create a `standalone` binary that opens a window and creates a 'Red' background. Verify resizing doesn't crash."
> 
> 

#### Phase 3: The Graphics Core (Vulkan & SVG)

*Goal: Set up the rendering loop.*

**Prompt:**

> "Implement the Vulkan rendering backend using the `ash` crate.
> **Requirements:**
> 1. Create a `VulkanContext` struct that initializes the `Instance`.
> * **macOS Special Case:** Enable `VK_KHR_portability_enumeration` to support MoltenVK.
> 
> 
> 
> 
> 2. Implement `Swapchain` management:
> * Support `VK_COMPOSITE_ALPHA_POST_MULTIPLIED_BIT_KHR` on macOS for transparency if supported.
> 
> 
> 
> 
> 3. Implement the **SVG Rasterizer**:
> * Use `usvg` to load an SVG tree at startup.
> * On `resize` events, render the SVG to a pixel buffer using `resvg`.
> 
> 
> * Upload this buffer to a Vulkan Image (Texture).
> 
> 
> 4. **Test Plan:** Render a simple SVG (a circle) to the screen. Resize the window and ensure the circle stays crisp (re-rasterized) and doesn't stretch."
> 
> 

#### Phase 4: The Core Logic & Persistence

*Goal: Connect audio processing and state management.*

**Prompt:**

> "Implement the Core Plugin Logic and State Management.
> **Requirements:**
> 1. **Audio Processor:** Implement a struct `PluginProcessor`. Use `rtrb` (RingBuffer)  to receive parameter changes from the GUI thread.
> 
> 
> * *Constraint:* The `process` block must be 100% wait-free.
> 
> 
> 2. **Persistence (Redb):**
> * Implement a `StateManager` struct that holds a `redb::Database`.
> 
> 
> * Run database writes (saving presets) on a dedicated background thread (Worker Thread) to avoid blocking audio or GUI.
> 
> 
> 3. **Protobuf Integration:** Define a `plugin_state.proto` schema for saving the parameter map. Use `prost` to generate the Rust struct.
> 4. **Test Plan:** Write a unit test that spawns the Worker Thread, saves a 'preset' to a temporary `redb` file, and verifies the Audio Processor receives the new values via the ring buffer."
> 
> 

#### Phase 5: The Test Harness (RPC)

*Goal: Enable the automated E2E testing.*

**Prompt:**

> "Implement the RPC Debug Server for integration testing.
> **Requirements:**
> 1. Create a TCP server that starts when the plugin initializes.
> * Bind to port `0` (ephemeral) to avoid conflicts.
> * Write the chosen port and PID to a lockfile in `std::env::temp_dir()`.
> 
> 
> 2. Define a generic Protocol Buffer service `DebugControl` with methods:
> * `SetParam(id, value)`
> * `GetMeteringData()`
> 
> 
> 3. **Integration:** The server must push commands onto the same `rtrb` queue used by the GUI, ensuring the Audio Thread treats RPC commands exactly like user clicks.
> 4. **Test Plan:** Write a Python script (using `grpcio` or raw TCP) that launches the standalone plugin, reads the port file, sends a parameter change, and asserts that the internal state updated."
> 
> 

### Clarifying Q/A:

* *Do we strictly require VST2 support?* (The prompts assume VST3/CLAP only as VST2 is legally difficult to distribute for new projects).
  * No, VST2 is not required.
* *Are we targeting Linux?* (The prompts focus on Mac/Win as requested, but `ash`/`xcb` could be added to Phase 2 if needed).
  * No, Linux is not required.

* **The "Animation Timer" Gap:** Since we cannot own the message loop (the DAW owns it), and we aren't using `winit`, the plugin needs a heartbeat to trigger `vkQueuePresent`.
* *Decision:* The PAL (Platform Abstraction Layer) prompt must explicitly request a `CVDisplayLink` (macOS) and `SetTimer`/`WM_TIMER` (Windows) implementation to drive the render loop at 60 FPS.


* **Input Event Bridging:** We need to handle mouse/keyboard interaction. Since we are using raw handles, we must manually intercept `mouseUp`/`mouseDown` (Cocoa) and `WM_LBUTTONDOWN` (Win32) and translate them into our own `UIEvent` enum to send to the logic thread.
* *Decision:* The PAL prompt must include requirements for an event translation layer.


* **DPI/Scale Factor Propagation:** For the SVG rasterizer to work correctly, it needs the screen's DPI.
* *Decision:* The PAL must expose a method `get_scale_factor()` querying `NSScreen` or `GetDpiForWindow`, passing this to the Rendering layer.