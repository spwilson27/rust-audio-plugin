# Phase 2: Platform Abstraction Layer - Next Steps

## Current Status

✅ **Completed:**
- PAL crate created with [NativeWindow](file:///Users/mrwilson/Software/rust-vst/pal/src/lib.rs#22-54) trait
- macOS and Windows module stubs
- GUI crate skeleton with `UIEvent` types
- Shader compilation working with Vulkan SDK detection
- All 5 tests passing

## Proposed Implementation Roadmap

### 1. macOS Implementation ([pal/src/macos.rs](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs))

**Priority: HIGH** - Focus on macOS first since that's the development platform

#### Tasks:

1. **NSView Creation** (Phase 2.1)
   - Use `objc2` to declare `RustPluginView` subclass
   - Implement `initWithFrame:` 
   - Set up `CAMetalLayer` backing layer
   - Store view handle in [MacOSWindow](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#17-24) struct

2. **Parenting Logic** (Phase 2.2)
   - Implement [attach()](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#26-31) to add view as subview of parent
   - Handle coordinate system transformation (bottom-left → top-left)
   - Implement [get_raw_handle()](file:///Users/mrwilson/Software/rust-vst/pal/src/win32.rs#31-39) to return actual NSView pointer

3. **Visibility Detection** (Phase 2.3)
   - Implement `viewDidMoveToWindow` callback
   - Update [is_visible()](file:///Users/mrwilson/Software/rust-vst/pal/src/win32.rs#51-55) to check `[self window] != nil`
   - Use this to pause rendering when plugin UI is hidden

4. **Render Loop** (Phase 2.4)
   - Create `CVDisplayLink` for 60 FPS timer
   - Implement callback to trigger rendering
   - Handle display link lifecycle (start/stop)

5. **Input Events** (Phase 2.5)
   - Implement `mouseDown:`, `mouseUp:`, `mouseDragged:`
   - Implement `keyDown:`, `keyUp:`
   - Transform events to `UIEvent` enum
   - Send events to GUI layer

6. **DPI Handling** (Phase 2.6)
   - Query `NSScreen.backingScaleFactor`
   - Store scale factor in [MacOSWindow](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#17-24)
   - Update on screen changes (handle notifications)

#### Technical Challenges:

- **objc2 Learning Curve**: Runtime class declaration is tricky
  - **Mitigation**: Start with minimal working example, expand incrementally
- **Memory Management**: Proper retain/release cycles
  - **Mitigation**: Use `objc2`'s safe wrappers where possible
- **CVDisplayLink Threading**: Callback runs on different thread
  - **Mitigation**: Use atomic flags or channels to communicate with main thread

#### Test Strategy:

```rust
#[test]
fn test_macos_window_creation() {
    // Create a test NSWindow as parent
    // Attach our view
    // Verify view hierarchy
}

#[test]
fn test_visibility_detection() {
    // Attach view to window
    // Remove window
    // Verify is_visible() returns false
}
```

### 2. Windows Implementation ([pal/src/win32.rs](file:///Users/mrwilson/Software/rust-vst/pal/src/win32.rs))

**Priority: MEDIUM** - Implement after macOS is working

Similar structure to macOS but using Win32 API:

1. **HWND Creation** - `CreateWindowExW` with `WS_CHILD` style
2. **WndProc** - Handle `WM_SIZE`, `WM_ERASEBKGND`, input events
3. **Timer** - `SetTimer` for 60 FPS render loop
4. **DPI** - `GetDpiForWindow`

### 3. GUI Framework Integration ([gui/src/lib.rs](file:///Users/mrwilson/Software/rust-vst/gui/src/lib.rs))

**Priority: MEDIUM** - Parallel with PAL implementation

1. **GuiContext Implementation**
   - Store [NativeWindow](file:///Users/mrwilson/Software/rust-vst/pal/src/lib.rs#22-54) instance
   - Route events from PAL to application logic
   - Coordinate with render layer (Phase 3)

2. **Event Handling**
   - Implement [handle_event()](file:///Users/mrwilson/Software/rust-vst/gui/src/lib.rs#91-96) with pattern matching
   - Add callback mechanism for application code
   - Handle resize events specially (trigger Vulkan swapchain rebuild)

3. **Standalone Mode**
   - [create_standalone()](file:///Users/mrwilson/Software/rust-vst/gui/src/lib.rs#84-90) creates top-level NSWindow/HWND
   - Then uses same PAL code for child view

### 4. Integration with Standalone Binary

**Priority: LOW** - After PAL + GUI basics work

Update [standalone/src/main.rs](file:///Users/mrwilson/Software/rust-vst/standalone/src/main.rs):

```rust
fn run_with_gui() -> Result<()> {
    let mut gui = GuiContext::create_standalone(800, 600, "splug")?;
    
    loop {
        // Poll events (manual loop for standalone)
        // For plugin, DAW drives the loop
        
        gui.render()?;
    }
}
```

### 5. Verification Plan

#### Manual Testing:

1. **Standalone Window Creation**
   ```bash
   cargo run --bin standalone
   ```
   - Verify window opens
   - Verify can resize, close
   - Check Activity Monitor for proper cleanup

2. **Test DAW Integration** (If available)
   - Load plugin in Reaper/Bitwig
   - Verify window appears
   - Tab away and back - check visibility handling
   - Resize plugin window

#### Automated Tests:

- **CI-Compatible**: Headless tests using mocked windowing
- **Local-Only**: Real window creation tests (require GUI environment)

### 6. Timeline Estimate

| Task | Effort | Dependencies |
|------|--------|--------------|
| macOS NSView Creation | 2-3 hours | None |
| macOS Parenting & Handle | 1-2 hours | NSView |
| macOS Visibility | 1 hour | NSView |
| macOS CVDisplayLink | 2-3 hours | NSView |
| macOS Input Events | 2-3 hours | NSView |
| macOS DPI | 1 hour | NSView |
| GUI Framework | 2-3 hours | PAL |
| Windows Implementation | 4-6 hours | macOS done |
| Integration Testing | 2-3 hours | All above |
| --- | --- | --- |
| **Total** | **17-26 hours** | |

### 7. Risk Mitigation

**Risk: objc2 API Changes**
- Current version: 0.5.2
- Mitigation: Lock dependency versions, document any workarounds

**Risk: CVDisplayLink Complexity**
- Multi-threading can cause subtle bugs
- Mitigation: Start with simple polling, add CVDisplayLink later if needed

**Risk: VST3 Resize Loops**
- Infinite resize callbacks in some DAWs
- Mitigation: Implement "Size Sentinel" as documented in architecture

### 8. Success Criteria

Phase 2 is complete when:

- [ ] Standalone binary opens a native window on macOS
- [ ] Window resizes correctly
- [ ] Mouse clicks are detected and logged
- [ ] Window closes cleanly without leaks
- [ ] All tests pass (including new PAL tests)
- [ ] Windows stub compiles (full impl optional for this phase)

## Recommended Approach

**Start Small, Iterate Fast:**

1. Get a basic NSView showing up in standalone (no events, no rendering)
2. Add proper parenting and handle retrieval
3. Add one event type (mouse click)
4. Expand to full event set
5. Add render loop coordination
6. Polish and test

This incremental approach reduces risk and provides early validation that the architecture works before investing in full implementation.

## Questions for Clarification

1. **Priority**: Should we implement both macOS and Windows in Phase 2, or defer Windows to later?
   - **Recommendation**: macOS first, Windows stub is sufficient for now

2. **Rendering**: Should Phase 2 include a simple test renderer (solid color), or wait for Phase 3?
   - **Recommendation**: Wait for Phase 3, just get windowing and events working

3. **VST3 Testing**: Do you have access to a VST3 host (Reaper/Bitwig/etc.) for testing?
   - If yes, we can test real plugin hosting
   - If no, standalone mode is sufficient for validation

---

**Next Action**: Implement macOS NSView creation in [pal/src/macos.rs](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs)
