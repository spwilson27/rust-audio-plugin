# macOS VM Testing with Tart

We use **Tart** to run macOS VMs on Apple Silicon for isolated testing and verification.

## Prerequisites

- Apple Silicon Mac
- Homebrew
- [Tart](https://github.com/cirruslabs/tart)

## Installation

```bash
brew install cirruslabs/cli/tart
```

## Setup

1. **Create the VM**:
   We recommend creating a fresh VM from the latest macOS IPSW:
   ```bash
   tart create vst-test-vm --from-ipsw=latest
   ```

2. **Run the VM**:
   ```bash
   tart run vst-test-vm
   ```
   This will boot the VM and show a window.

3. **Login**:
   - Username: `admin`
   - Password: `admin`

## Development Workflow

### Mounting the Codebase
Tart supports mounting directories into the VM. To mount the current project directory:

```bash
tart run --dir=rust-vst-2:$(pwd) vst-test-vm
```
Inside the VM, this will be available at `/Volumes/rust-vst-2` or similar (usually requires Tart Guest tools, but the `--dir` flag uses virtio-fs which is supported by macOS guests).

### Running Tests
Inside the VM:
1. Open Terminal.
2. Navigate to the mounted volume.
3. Run `cargo test` (you will need to install Rust in the VM first).

## Automation (Planned)
We plan to integrate this into `xtask` so you can run:
`cargo xtask test --vm`
which will spin up the VM checkouts the code, runs the tests, and shuts down.
