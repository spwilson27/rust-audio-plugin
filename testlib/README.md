# testlib

Test utilities for the splug audio plugin project.

## Purpose

This crate provides shared functionality for integration tests and build tools to avoid code duplication across the workspace.

## Features

- **Lockfile Management**: Clean up and wait for standalone process lockfiles
- **Process Spawning**: Spawn standalone with proper configuration
- **RPC Client**: Connect to debug server with appropriate message size limits
- **Screenshot Capture**: Capture screenshots via RPC
- **Complete Workflow**: High-level `capture_golden()` function encapsulating the entire process

## Usage

### In xtask

```rust
use testlib;

let img = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()?
    .block_on(async {
        testlib::capture_golden(&root_dir).await
    })?;

img.save("golden.png")?;
```

### In Integration Tests

```rust
#[tokio::test]
async fn test_screenshot() {
    let root = std::env::current_dir().unwrap();
    let img = testlib::capture_golden(&root)
        .await
        .expect("Failed to capture");
    // Verify img...
}
```

## API

### High-Level

- `capture_golden(root_dir: &Path) -> Result<RgbaImage>` - Complete workflow from spawn to capture

### Low-Level Components

- `cleanup_lockfiles() -> Result<()>` - Remove old lockfiles
- `spawn_standalone(root_dir: &Path) -> Result<Child>` - Spawn standalone process
- `wait_for_lockfile(timeout: Duration) -> Result<PathBuf>` - Wait for lockfile creation
- `parse_lockfile(path: &Path) -> Result<LockfileInfo>` - Parse lockfile JSON
- `connect_rpc(port: u16, timeout: Duration) -> Result<DebugControlClient>` - Connect with proper limits
- `capture_screenshot(client: &mut DebugControlClient) -> Result<RgbaImage>` - Request screenshot
- `quit_standalone(client: &mut DebugControlClient) -> Result<()>` - Send quit command
