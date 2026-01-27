Based on the Architectural Specification we developed, here is the plan to guide the implementation Agents.

### 2. Agent Persona & System Prompt

*Use this "System Prompt" as the preamble for every agent interaction to ensure consistency.*

> **Role:** You are a Principal Rust Audio Engineer specializing in `unsafe` systems programming and lock-free concurrency.
> **Project Context:** We are building "splug" a high-performance audio plugin (VST3/CLAP) using a "metal-down" approach. We **do not** use GUI frameworks like `egui` or `winit` because they fail to handle the parent-child windowing requirements of DAWs. We use raw `ash` (Vulkan) and native windowing APIs (`objc2`/`windows-sys`) for windowing.
> **Core Constraints:**
> 1. **Real-Time Safety:** No allocations (`Box`, `Vec`, `String`) or mutex locking on the audio thread. Use `rtrb` or atomics.
> 2. **Panic Safety:** The C-ABI boundary (VST3/CLAP entry points) must capture panics to prevent crashing the host DAW.
> 3. **Testing:** Every module must include a `test` module. If a component requires a window, create a "Headless" mock or use the RPC server for verification.
> 4. **Build System:** Use `cargo-xtask` to manage the complex "polyglot" build process (Rust compilation, Shader compilation, Bundle creation, Codesigning) within the Rust ecosystem.17 
> 5. For any questions you need clarificaiton on discuss them before proceeding with implementation.

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
> 1. Create a workspace with members: `plugin` (lib), `standalone` (bin), `xtask` (bin), and `definitions` (for Protobuf schemas).
> 2. The `standalone` binary must support a `--headless` flag that initializes the audio engine and RPC server without creating a window (for automated testing).
> 3. Implement a `cargo xtask bundle` command that:
> * Compiles the Rust library in `release` mode.
> * Compiles GLSL shaders in `plugin/src/shaders/` to SPIR-V using `glslc` (assume it's in the PATH) and embeds them as bytes.
> * **macOS:** Creates the `.vst3/Contents/MacOS` bundle structure, copies the dylib, and generates a valid `Info.plist`.
> * **macOS Codesigning:** Executes `codesign -s -` on the bundle to prevent macOS from blocking the "unverified developer" binary. This is critical for local testing.
> 
> 
> * **Windows:** Renames the `.dll` to `.vst3`.
> 
> 
> 4. **Test Plan:** Create a dummy `lib.rs` that exports a basic VST3 entry point. Running `cargo xtask bundle` should produce a folder structure recognized by a VST3 host (like Reaper or Bitwig)."
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
> * `fn get_scale_factor(&self) -> f64` - Queries `NSScreen.backingScaleFactor` (macOS) or `GetDpiForWindow` (Windows) for proper DPI-aware rendering.
> 
> 
> 2. **macOS Implementation (`pal/macos.rs`):**
> * Use `objc2` to declare a class `RustPluginView` inheriting from `NSView`.
> * Implement `viewDidMoveToWindow` to detect when the UI is opened/closed. When `[self window]` becomes nil, pause the render loop to prevent swapchain errors.
> * Add a `CAMetalLayer` as the backing layer for Vulkan support. Set `metal_layer.set_opaque(false)` to support transparency.
> * **Animation Timer:** Implement a `CVDisplayLink` to drive the render loop at 60 FPS. Since the plugin doesn't own the message loop (the DAW does), this heartbeat is critical for triggering `vkQueuePresent`.
> * **Input Event Translation:** Implement handlers for `mouseDown`, `mouseUp`, `mouseDragged`, and `keyDown`. Translate Cocoa coordinates (bottom-left origin) to Vulkan coordinates (top-left origin) using: `vulkan_y = view_height - cocoa_y`. Send these to a custom `UIEvent` enum routed to the logic thread.
> 
> 
> 3. **Windows Implementation (`pal/win32.rs`):**
> * Use `windows-sys`. Create a window class with `CS_HREDRAW | CS_VREDRAW`.
> * Handle `WM_SIZE` to trigger swapchain rebuilds.
> * **Crucial:** Handle `WM_ERASEBKGND` by returning 1 to prevent flickering during resize.
> * **Animation Timer:** Use `SetTimer` with a 16ms interval (≈60 FPS) to send `WM_TIMER` messages. In the `WndProc`, handle this message to trigger rendering.
> * **Input Event Translation:** Handle `WM_LBUTTONDOWN`, `WM_LBUTTONUP`, `WM_MOUSEMOVE`, `WM_KEYDOWN`. Translate these into the same `UIEvent` enum used by macOS and send to the logic thread.
> 
> 
> 
> 
> 4. **VST3 Resize Protocol:** Implement the "Size Sentinel" to prevent infinite resize loops in hosts like FL Studio:
> * When the user drags to resize, call `IPlugFrame::resizeView(newRect)`.
> * Only resize the internal structures in the `onSize` callback from the host.
> * If `onSize` is called with dimensions identical to current dimensions, return immediately to break recursion.
> 
> 
> 5. **Test Plan:** Create a `standalone` binary that opens a window and creates a 'Red' background with a 60 FPS counter displayed. Verify resizing doesn't crash and the FPS stays stable. Test mouse click events by changing the background color on click."
> 
> 

#### Phase 3: The Graphics Core (Vulkan & SVG)

*Goal: Set up the rendering loop.*

**Prompt:**

> "Implement the Vulkan rendering backend using the `ash` crate.
> **Requirements:**
> 1. Create a `VulkanContext` struct that initializes the `Instance`.
> * **macOS Special Case:** Enable `VK_KHR_portability_enumeration` and set `VK_INSTANCE_CREATE_ENUMERATE_PORTABILITY_BIT_KHR` flag to discover MoltenVK physical devices.
> * **Graceful Fallback:** If `vkCreateInstance` fails (e.g., broken drivers on old Windows machines), return an error that causes the plugin to enter "No-GUI" mode. Log the failure to a file in the system temp directory. The plugin should still function with generic parameter controls, just without custom UI.
> 
> 
> 
> 
> 2. Implement `Swapchain` management:
> * **macOS Transparency:** Query surface capabilities for `VK_COMPOSITE_ALPHA_POST_MULTIPLIED_BIT_KHR`. If supported, use this in `VkSwapchainCreateInfoKHR` and ensure fragment shaders output premultiplied alpha colors.
> * **Windows:** Default to `VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR` for performance. True per-pixel transparency on Win32 child windows requires `WS_EX_LAYERED` which is CPU-intensive.
> 
> 
> 
> 
> 3. Implement the **SVG Rasterizer**:
> * Use `usvg` to load an SVG tree at startup.
> * On `resize` events (or DPI changes), render the SVG to a pixel buffer using `resvg` at the exact target resolution (using the `scale_factor` from the PAL).
> * Upload this buffer to a Vulkan Image (Texture) via a staging buffer:
>   1. Map a Staging Buffer (Host Visible).
>   2. `memcpy` the rasterized pixels.
>   3. Issue `vkCmdCopyBufferToImage` to transfer to Device Local `VkImage`.
>   4. Transition layout to `SHADER_READ_ONLY_OPTIMAL`.
> 
> 
> * The 60 FPS render loop (driven by the PAL timer) composites this texture to the screen using a simple quad. Heavy rasterization only happens on resize, not every frame.
> 
> 
> 4. **Test Plan:** Render a simple SVG (a circle) to the screen. Resize the window and ensure the circle stays crisp (re-rasterized) and doesn't stretch. Test on different DPI displays (e.g., Retina) to verify scale factor propagation works correctly."
> 
> 

#### Phase 4: The Core Logic & Persistence

*Goal: Connect audio processing and state management.*

**Prompt:**

> "Implement the Core Plugin Logic and State Management.
> **Requirements:**
> 1. **FFI Panic Safety:** All VST3/CLAP C-ABI entry points (e.g., `IPlugView::attached`, `IAudioProcessor::process`) must be wrapped in `std::panic::catch_unwind` boundaries. If a panic occurs, log it to a file and return a safe error code to the host DAW. **Never** let a panic propagate across the FFI boundary as it will crash the host.
> 
> 
> 2. **Audio Processor:** Implement a struct `PluginProcessor`. Use `rtrb` (Real-Time Ring Buffer) to receive parameter changes from the GUI thread. For preset data that's larger than individual parameters, consider using a **Triple Buffer** pattern as an alternative to `rtrb`.
> 
> 
> * *Constraint:* The `process` block must be 100% wait-free. No allocations, no mutex locks, no file I/O.
> * **NaN/Inf Handling:** Add fuzz testing that injects `NaN` and `Inf` values into audio buffers to ensure the processor doesn't panic or produce invalid output. Use `f32::is_finite()` checks at critical points.
> 
> 
> 3. **Persistence (Redb):**
> * Implement a `StateManager` struct that holds a `redb::Database`.
> * **Critical Constraint:** The Audio Thread must **never** interact with Redb directly (no read or write transactions). It must only read from pre-fetched buffers containing POD (Plain Old Data) types populated by the Main Thread.
> * Run database writes (saving presets) on a dedicated background **Worker Thread** to avoid blocking audio or GUI.
> * Example workflow for preset loading:
>   1. User selects "Preset A" in GUI.
>   2. Main Thread requests "Preset A" from Redb (Read Transaction on Worker Thread).
>   3. Data is deserialized into a `Preset` struct.
>   4. Data is sent to Audio Thread via wait-free queue or Triple Buffer.
>   5. Audio Thread applies parameter values at start of next processing block.
> 
> 
> 4. **Protobuf Integration:** Define a `plugin_state.proto` schema in the `definitions/` directory for saving the parameter map. Use `prost` to generate the Rust struct. Implement schema versioning and migration logic for forward compatibility.
> 
> 
> 5. **Test Plan:** Write a unit test that:
> * Spawns the Worker Thread.
> * Saves a 'preset' to a temporary `redb` file.
> * Verifies the Audio Processor receives the new values via the ring buffer without blocking.
> * Includes a fuzz test that processes buffers containing NaN/Inf values and asserts no panics occur."
> 
> 

#### Phase 5: The Test Harness (RPC)

*Goal: Enable the automated E2E testing.*

**Prompt:**

> "Implement the RPC Debug Server for integration testing.
> **Requirements:**
> 1. Create a TCP server that starts when the plugin initializes (on the Worker Thread to avoid blocking Main or Audio threads).
> * Bind to port `0` (ephemeral) to avoid conflicts when running multiple tests in parallel.
> * Write the chosen port and PID to a lockfile in `std::env::temp_dir()` (e.g., `/tmp/myplugin_test/pid_<PID>.json` with content: `{ "port": 54321, "pid": 1234 }`).
> 
> 
> 2. Define Protocol Buffer service in `definitions/debug_control.proto`:
> ```protobuf
> service DebugControl {
>     rpc SetParam(ParamChange) returns (Ack);
>     rpc GetParam(ParamChange) returns (ParamValue);
>     rpc SendKey(KeyMsg) returns (Ack);
>     rpc SendMouse(MouseMsg) returns (Ack);
>     rpc GetMeteringData(Empty) returns (MeterData);
>     rpc InjectMidi(MidiMsg) returns (Ack);
>     rpc GetPerformanceStats(Empty) returns (CpuStats);
> }
> ```
> 
> 
> 3. **Integration:** The server must push commands onto the same `rtrb` queue used by the GUI, ensuring the Audio Thread treats RPC commands exactly like user clicks. This provides true "Grey Box" testing.
> 
> 
> 4. **Host Quirks Testing:** Create a "Headless Host" test harness using `vst3-sys` bindings that mimics the behavior of different DAWs (e.g., Bitwig's resize handling vs Ableton Live's approach). This harness should:
> * Load the plugin dynamically.
> * Call VST3 lifecycle methods in different sequences.
> * Test edge cases like rapid window resize, tab switching, and plugin suspend/resume.
> 
> 
> 5. **Test Plan:** Write a Python script (using `grpcio` or raw TCP with Protobuf framing) that:
> * Launches the standalone plugin with `--headless` flag.
> * Reads the port file to discover the RPC port.
> * Sends a parameter change via `SetParam`.
> * Sends a MIDI note via `InjectMidi`.
> * Waits 100ms then calls `GetMeteringData`.
> * Asserts that the metering RMS level is within expected range (proving audio processing occurred).
> * Tests the Headless Host harness by loading the plugin and executing the resize edge cases."
> 

---

### 4. Expanded Clarifications

#### 4.1. Threading Model Summary

To ensure agents understand the complete threading picture:

* **Audio Thread (Real-Time):**
  * Executes `process()`. Strict no-allocation, no-blocking rules.
  * Reads parameters via atomics or `rtrb::Consumer`.
  * Writes metering data to `rtrb::Producer`.
  * **Never** touches Redb or performs I/O.

* **Main Thread (GUI/OS):**
  * Handles OS Event Loop (Win32 `WndProc`, Cocoa message dispatch).
  * Manages window resizing and Vulkan swapchain recreation.
  * Reads metering data from `rtrb::Consumer`.
  * Writes parameter changes (from UI widgets) to `rtrb::Producer`.
  * Reads preset data from Triple Buffer (populated by Worker Thread).

* **Worker Thread (Background):**
  * Handles all Redb transactions (read/write).
  * Handles RPC server (listens for TCP connections).
  * Decodes/encodes Protobuf messages.
  * Loads presets and writes to Triple Buffer for Main Thread pickup.

#### 4.2. Wait-Free Data Structures

* **`rtrb`**: Single-producer, single-consumer (SPSC) lock-free ring buffer. Use for streaming parameter changes and metering data.
* **Triple Buffer**: Lock-free shared state pattern. Use for larger data structures like full preset snapshots. Allows the writer to update without blocking the reader, with at-most-one-frame latency.
* **Atomics**: Use `std::sync::atomic` for simple flags and normalized parameter values (e.g., `AtomicU32` storing `f32.to_bits()`).

#### 4.3. VST3/CLAP Support Strategy

* **Initial Implementation:** Focus on VST3 for macOS and Windows. VST3 is the industry standard and well-documented.
* **CLAP Support:** CLAP is simpler and more modern. After VST3 is stable, add CLAP support by implementing the `clap_plugin_t` interface. Much of the core logic (audio processing, PAL, graphics) can be shared.
* **VST2:** Not required. VST2 SDK has licensing restrictions for new projects.

#### 4.4. Known Platform Edge Cases

* **macOS:**
  * Logic Pro X sometimes calls `viewDidMoveToWindow` unexpectedly. Always check `[self window] != nil` before accessing window properties.
  * Retina displays require careful DPI handling. The backing scale factor affects both SVG rasterization and mouse coordinate translation.
  * Codesigning is mandatory for plugins loaded into DAWs like GarageBand (strict entitlement checking).

* **Windows:**
  * FL Studio has historically had resize loop bugs. The Size Sentinel mitigation is critical here.
  * Some Windows 7 machines have incomplete Vulkan driver implementations. The graceful fallback to No-GUI mode prevents support issues.
  * Win32 child windows inside a 64-bit DAW from a 32-bit plugin (or vice versa) require careful handle marshaling. For this project, assume we only build 64-bit plugins.

#### 4.5. Protobuf Versioning Strategy

When updating `plugin_state.proto`:
* Never remove fields. Mark deprecated fields as `reserved`.
* Always add new fields with higher field numbers.
* In Rust deserialization code, check the `version` field and apply migration transforms if loading an older state.

Example:
```rust
fn migrate_state(state: PluginState) -> PluginState {
    match state.version {
        1 => migrate_v1_to_v2(state),
        2 => state, // current version
        _ => panic!("Unsupported state version"),
    }
}
```

#### 4.6. Testing Philosophy

* **Unit Tests:** Every Rust module has a `#[cfg(test)]` module. Test pure logic functions (DSP algorithms, state serialization) without requiring OS resources.
* **Integration Tests (Headless):** Use the `--headless` mode and RPC server to test the full audio pipeline without a window. Run these in CI.
* **E2E Tests (Headless Host):** The custom VST3 host harness validates lifecycle correctness and host compatibility quirks.
* **Manual Tests (DAW):** Final validation in real DAWs (Reaper, Ableton, Bitwig) before release. Document any discovered quirks and add them to the Headless Host harness.

---

### 5. Q/A Summary

* **Do we strictly require VST2 support?**
  * No, VST2 is not required due to licensing restrictions.

* **Are we targeting Linux?**
  * No, initial scope is macOS and Windows only. The PAL could be extended with `xcb` for Linux later.

* **Graphics Fallback on No Vulkan?**
  * Yes, the plugin must enter a "No-GUI" mode gracefully. It will expose only generic parameter controls and log the error.

* **What if SVG rasterization is slow?**
  * SVG rasterization only happens on resize/DPI change, not every frame. The 60 FPS loop composites a pre-rasterized texture. For complex SVGs, consider caching multiple resolution levels.

* **How do we handle DAW-specific bugs?**
  * The Headless Host test harness simulates known quirky behaviors. When new bugs are discovered in real DAWs, we add test cases to the harness and implement workarounds in the PAL or plugin core.

---

### 6. Implementation Checklist

Use this as a high-level verification before moving to the next phase:

- [ ] **Phase 1 Complete:**
  - [ ] Workspace structure created (`plugin`, `standalone`, `xtask`, `definitions`)
  - [ ] `cargo xtask bundle` produces valid VST3 bundle on macOS
  - [ ] `cargo xtask bundle` produces valid VST3 on Windows
  - [ ] macOS bundle is codesigned
  - [ ] `standalone --headless` flag runs without window

- [ ] **Phase 2 Complete:**
  - [ ] PAL trait defined with all required methods (`attach`, `get_raw_handle`, `set_size`, `get_scale_factor`)
  - [ ] macOS: `CVDisplayLink` running at 60 FPS
  - [ ] Windows: `WM_TIMER` running at 60 FPS
  - [ ] Mouse/keyboard events translated to `UIEvent` enum
  - [ ] VST3 resize protocol implemented with Size Sentinel
  - [ ] Test: Standalone shows 60 FPS counter and responds to clicks

- [ ] **Phase 3 Complete:**
  - [ ] Vulkan context initializes on macOS (MoltenVK)
  - [ ] Vulkan context initializes on Windows
  - [ ] Graceful fallback on Vulkan init failure
  - [ ] SVG rasterizes crisply at multiple window sizes
  - [ ] DPI scaling works on Retina displays
  - [ ] Test: Circle SVG stays sharp when resizing

- [ ] **Phase 4 Complete:**
  - [ ] All FFI entry points wrapped in `catch_unwind`
  - [ ] Audio `process()` is wait-free (no allocations)
  - [ ] `rtrb` queues for parameter changes and metering
  - [ ] Redb database never accessed from Audio Thread
  - [ ] Worker Thread handles all Redb transactions
  - [ ] Protobuf schema defined with versioning
  - [ ] Fuzz test passes with NaN/Inf inputs
  - [ ] Test: Preset loads without blocking audio

- [ ] **Phase 5 Complete:**
  - [ ] RPC server binds to ephemeral port
  - [ ] Port discovery via lockfile works
  - [ ] `DebugControl` service defined in Protobuf
  - [ ] RPC commands routed through same queue as GUI
  - [ ] Headless Host harness loads plugin and tests lifecycle
  - [ ] Python/Rust E2E test script passes
  - [ ] Test: RPC `SetParam` + `GetMetering` returns expected values

---

This updated prompt document now captures all critical architectural details from `architecture_v2.md`, ensuring agents have complete context for robust implementation.