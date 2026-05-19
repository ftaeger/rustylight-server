# CLAUDE.md — rustylight-server

## What this project is

`rustylight-server` is a Rust HTTPS REST API server that controls a **Kuando Busylight** USB status light. It runs on a **Raspberry Pi 4 (arm64)** and exposes a simple API over TLS so other devices on the network can set the light color, enable blinking, or read the current state.

## Architecture

```
src/
  main.rs          — startup: load config, init logging, spawn USB manager, start TLS server
  config.rs        — TOML config load/save, PSK generation (64-char hex, auto-generated on first start)
  tls.rs           — self-signed ECDSA P-256 cert generation via rcgen; auto-generated on first start
  logging.rs       — tracing-subscriber setup with log file rotation
  api/
    mod.rs         — axum router: /api/light (auth), /api/public/healthcheck, /api/public/version, Swagger UI
    auth.rs        — X-Api-Key header extractor (FromRequestParts, constant-time compare via subtle)
    handlers.rs    — get_light, post_light, get_version, get_healthcheck handlers + response structs
    openapi.rs     — utoipa OpenAPI spec, Swagger UI at /api
  device/
    mod.rs         — LightState struct (serde), SharedState (Arc<Mutex<>>)
    manager.rs     — background thread: polls USB every 2s, writes HID report (65 bytes incl. Report ID)
    models.rs      — known VID/PID table for all supported Busylight models
    report.rs      — builds 65-byte HID report (byte 0 = Report ID 0x00, bytes 1-64 = payload)
```

## API endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/light` | X-Api-Key | Current light state + `connected` field |
| POST | `/api/light` | X-Api-Key | Set new light state |
| GET | `/api/public/healthcheck` | none | Service health status (`busylight_connected`, `log_writable`) |
| GET | `/api/public/version` | none | Server version + current UTC time (RFC 3339) |
| GET | `/api` | none | Swagger UI |
| GET | `/api/openapi.json` | none | OpenAPI spec |

## Authentication

Every `/api/light` request requires header `X-Api-Key: <psk>`. The PSK is a 64-char hex string stored in `/etc/rustylight/rustylight.conf`, auto-generated on first start. Comparison is constant-time via the `subtle` crate.

## Configuration file

Default path: `/etc/rustylight/rustylight.conf`

```toml
[server]
port = 8443

[tls]
cert_file = "/etc/rustylight/tls.crt"
key_file  = "/etc/rustylight/tls.key"

[auth]
psk = ""   # auto-generated 64-char hex on first start

[logging]
level    = "info"
log_file = "/var/log/rustylight/rustylight.log"
```

## Supported hardware

Kuando Busylight models detected by VID/PID — see `src/device/models.rs`. User's device: `27bb:3bcd` (Busylight Omega).

## Development workflow

- **Branch:** always work on a feature branch, never commit directly to `main`
- **PR/release:** only after CI is green on the feature branch
- **Tests:** `cargo test` (61 tests total: unit + integration)
- **Lint:** `cargo fmt --check && cargo clippy -- -D warnings`
- **CI:** `.github/workflows/ci.yml` — runs on push/PR, skips for `.md` and `docs/` changes
- **Release pipeline:** `.github/workflows/release.yml` — triggered by GitHub release publish or `workflow_dispatch`; builds `.deb` and `.rpm` for 4 targets (x86_64, i686, aarch64, armv7)

## Packaging

Built with `cargo-deb` and `cargo-generate-rpm`. Installs to:
- `/usr/sbin/rustylight-server`
- `/lib/systemd/system/rustylight.service`
- `/lib/udev/rules.d/99-busylight.rules` (grants `plugdev` group HID access)
- `/etc/logrotate.d/rustylight`

`postinst` creates the `rustylight` system user, reloads udev, and enables the systemd service.

## HID protocol note

The write buffer is **65 bytes**: `buf[0] = 0x00` (Report ID prefix), `buf[1..65]` is the 64-byte payload. Linux's `usbhid_output_report()` strips `buf[0]` before USB transmission, so the device always receives exactly 64 bytes.

Payload layout (64 bytes, serialised as 8 big-endian uint64s):
- Bytes 0–55: 7 steps × 8 bytes each. Each step is a 64-bit big-endian word:
  - Byte 0: `(opcode << 4) | target_step`. Opcodes: `Jump=0x1`, `KeepAlive=0x8`.
  - Byte 1: `repeat` — 0 = loop forever, 1 = play once then jump to `target_step`.
  - Bytes 2–4: R, G, B — scaled to 0–100 (not raw 0–255). `scale(c) = c * 100 / 255`.
  - Byte 5: `duty_cycle_on` — on-time in 100 ms units (0 = steady on when off=0 too).
  - Byte 6: `duty_cycle_off` — off-time in 100 ms units.
  - Byte 7: flags (audio/ringtone/volume — always 0 for this server).
- Bytes 56–63: footer — `[0x00, 0x00, 0x00, 0x00, 0x0F, 0xFF, checksum_hi, checksum_lo]`. The checksum is the 16-bit big-endian sum of payload bytes 0–61.

Steady-on: `Jump` (0x1), target=0, repeat=0, colour scaled, on=0, off=0.
Single-colour blink: `Jump`, target=0, repeat=0, on=`ms/100`, off=`ms/100`.
Two-colour blink: step 0 → `Jump` target=1, repeat=1; step 1 → `Jump` target=0, repeat=1.

VID 0x27BB PID 0x3BCD is a Busylight **Omega** (misidentified as Alpha in older docs).

## Key files for client developers

- `docs/API.md` — full API reference
- `examples/get-state.sh` — read current state
- `examples/set-color.sh` — set color via curl

## Design docs and plans

Stored in `docs/drafts/specs/` and `docs/drafts/plans/`. These are tracked in git.

## Debugging on the Pi

```bash
RUST_LOG=rustylight_server=trace rustylight-server
```

Shows HID report bytes and write results per cycle.
