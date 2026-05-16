# Contributing

## Prerequisites

- Rust stable (1.75+): https://rustup.rs
- `libhidapi-dev` (for hidapi-rs): `sudo apt install libhidapi-dev`
- Docker (for cross-compilation): https://docs.docker.com/get-docker/
- `cross` (CI cross-compiler): `cargo install cross`
- `cargo-deb` (for .deb packaging): `cargo install cargo-deb`
- `cargo-generate-rpm` (for .rpm packaging): `cargo install cargo-generate-rpm`

## Local Build

```bash
cargo build
cargo test
cargo build --release
```

## Cross-Compilation

```bash
cross build --target aarch64-unknown-linux-gnu --release
cross build --target armv7-unknown-linux-gnueabihf --release
```

## Packaging

Build `.deb` for native arch:
```bash
cargo build --release
cargo deb
```

Build `.rpm`:
```bash
cargo build --release
cargo generate-rpm
```

## Running Tests

```bash
cargo test                    # all tests
cargo test device::           # device layer unit tests only
cargo test test_auth          # auth integration tests only
```

## HID Protocol

The Busylight HID report format is documented in `src/device/report.rs`. If a new Busylight model is released, add its VID/PID to `src/device/models.rs`. If it uses a different report format, add a new `ModelVariant` and handle it in `build_report`.

To debug HID traffic: `sudo modprobe usbmon` then capture with Wireshark on the `usbmonN` interface.

## Cutting a Release

1. Update `version` in `Cargo.toml`
2. Commit: `git commit -am "chore: bump version to x.y.z"`
3. Tag: `git tag v<x.y.z>`
4. Push: `git push && git push --tags`

The `release.yml` workflow builds all packages and creates a GitHub Release automatically.
