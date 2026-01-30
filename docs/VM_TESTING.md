# macOS VM Testing with Tart

We use **Tart** to run macOS VMs on Apple Silicon for isolated testing and confirmation of standard C-ABI behavior without local machine pollution.

## Prerequisites

- Apple Silicon Mac
- Homebrew
- [Tart](https://github.com/cirruslabs/tart): `brew install cirruslabs/cli/tart`
- [Tart](https://github.com/cirruslabs/tart): `brew install cirruslabs/cli/tart`
- `sshpass`: `brew install sshpass` (for automated password entry)

## VM Requirements

The VM requires the following tools installed (e.g., in `~/bin` or standard PATH):
- **Rust Toolchain**: `rustup` (cargo, rustc)
- **Protobuf Compiler**: `protoc` (download from [GitHub Releases](https://github.com/protocolbuffers/protobuf/releases))
- **Vulkan SDK**: Download from [LunarG](https://vulkan.lunarg.com/sdk/home)


## Setup

1. **Create the VM**:
   ```bash
   tart create vst-test-vm --from-ipsw=latest --disk-size 100
   ```

2. **Configure VM**:
   - Run the VM: `tart run vst-test-vm`
   - Login (User: `admin`, Pass: `admin`)
   - **Enable SSH**: System Settings -> General -> Sharing -> Enable **Remote Login**
   - **Automatic Login**: System Settings -> Users & Groups -> Automatically login as `admin` (or see [Apple Support](https://support.apple.com/en-us/102316))
   - Install the Vulkan SDK

## Usage

Run tests using `xtask`:

```bash
cargo xtask test --vm
```

This command will automatically:
1. **Build tests on your Host machine** (fast, uses local caches).
2. **Start the VM** (if not running) with the project directory mounted.
3. **Sync** the project to the VM using `rsync` (excludes `target/` and `.git`, but includes necessary artifacts).
4. **Execute** the tests inside the VM via SSH.

## Debugging

- If tests hang, check VM network connectivity.
- `xtask` expects the VM IP to be reachable.
- Logs from the VM are streamed to your console.
