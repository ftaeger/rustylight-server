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
    mod.rs         — axum router: /api/light (auth), /api/version (public), Swagger UI
    auth.rs        — X-Api-Key header extractor (FromRequestParts, constant-time compare via subtle)
    handlers.rs    — get_light, post_light, get_version handlers + VersionResponse struct
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
| GET | `/api/version` | none | Server version + current UTC time (RFC 3339) |
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

Kuando Busylight models detected by VID/PID — see `src/device/models.rs`. User's device: `27bb:3bcd` (Busylight Alpha).

## Development workflow

- **Branch:** always work on a feature branch, never commit directly to `main`
- **PR/release:** only after CI is green on the feature branch
- **Tests:** `cargo test` (52 tests total: unit + integration)
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

The Busylight receives a **65-byte** HID report: byte 0 is the Report ID (`0x00`), bytes 1–56 are 8 color steps × 7 bytes each (R, G, B, on_hi, on_lo, off_hi, off_lo), byte 64 is the keepalive (5 seconds). hidapi on Linux requires the Report ID prefix.

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
