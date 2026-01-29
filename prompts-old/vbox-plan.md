# Multi-Platform Testing (Linux Docker & macOS VirtualBox)

We will extend the testing infrastructure to support running the macOS version of the plugin and its E2E tests within a VirtualBox VM. This provides a more reliable environment for macOS virtualization on various hosts compared to Docker.

## User Review Required

> [!IMPORTANT]
> **VirtualBox Setup**:
> - **Manual Setup**: The user must manually create a macOS VM (already in progress) and ensure `VBoxManage` is in the host's PATH.
> - **Guest Additions**: Shared Folders and Clipboard support require Guest Additions to be installed in the VM.
> - **GPU Acceleration**: VirtualBox's 3D acceleration for macOS guests is limited. We will still likely depend on **MoltenVK + SwiftShader** or software rendering for headless/automation reliability.

## Proposed Changes

### [xtask]
We will add VirtualBox orchestration capabilities to the `xtask` tool.

#### [MODIFY] [main.rs](file:///Users/mrwilson/Software/rust-vst-2/xtask/src/main.rs)
- Add a `--vbox` flag to `Test` and `Build` commands.
- Implement `VBoxManager` struct to wrap `VBoxManage` CLI calls:
  - `start_vm(name)`
  - `stop_vm(name)`
  - `execute_command(guest_cmd)` (via Guest Control or SSH)
- Implement code synchronization:
  - Option A: Use VirtualBox **Shared Folders** to mount the host project directory.
  - Option B: Use **rsync/scp** to push the codebase to the VM before testing.

### [E2E Tests]
#### [MODIFY] [testlib](file:///Users/mrwilson/Software/rust-vst-2/testlib/src/lib.rs)
- Ensure the RPC heartbeat and screenshot capture handles network latency/timeouts associated with VM bridges.

## Verification Plan

### Automated Tests
- `cargo xtask test --vbox --vm-name "macOS-CI"`
- Verify that the VM is started, tests are executed, and results (including screenshots/goldens) are retrieved before the VM is shut down.
- `cargo xtask test --docker` (Linux)
- Ensure no regressions in the existing Linux-in-Docker workflow.

### Manual Verification
- Test `xtask`'s ability to take and restore snapshots to ensure a clean state for every test run.
