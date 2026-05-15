# rustylight-server — Design Specification

**Date:** 2026-05-16
**Status:** Approved

---

## Overview

`rustylight-server` is a Rust-based Linux server daemon that controls a Kuando Busylight connected via USB. It exposes a TLS-encrypted HTTP REST API protected by HMAC-based authentication, manages the Busylight device (including hot-plug reconnection), and is distributed as `.deb` and `.rpm` packages for multiple architectures.

---

## Architecture

The service is a single process running one Tokio async runtime with two concurrent execution contexts:

```
┌─────────────────────────────────────────────────────┐
│                  rustylight-server                  │
│                                                     │
│  ┌──────────────┐    ┌──────────────────────────┐  │
│  │  USB Thread  │    │    Axum HTTP Server       │  │
│  │  (blocking)  │    │    (async, Tokio)         │  │
│  │              │    │                           │  │
│  │  hidapi poll │◄──►│  /api       → Swagger UI  │  │
│  │  hot-plug    │    │  GET /api/light → status  │  │
│  │  reconnect   │    │  POST /api/light → set    │  │
│  └──────┬───────┘    └──────────────┬────────────┘  │
│         │                           │               │
│         └──────── Arc<Mutex<        ┘               │
│                   BuslightState>>                   │
│                                                     │
│  ┌──────────────────────────────────────────────┐   │
│  │  Config (TOML)  │  TLS (rustls)  │  Logging  │   │
│  └──────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

**Key crates:**
- `axum` — async HTTP framework (Tokio-native)
- `axum-server` — TLS integration with rustls backend
- `hidapi` — USB HID device access (Rust bindings to C hidapi library)
- `rustls` — pure-Rust TLS, no OpenSSL dependency
- `utoipa` + `utoipa-swagger-ui` — OpenAPI 3.0 spec generation and Swagger UI
- `tracing` + `tracing-subscriber` — structured logging
- `cross` — cross-compilation toolchain (CI only)
- `cargo-deb` — `.deb` package generation
- `cargo-generate-rpm` — `.rpm` package generation

---

## USB / Hardware Layer

### Device Support

All Kuando Busylight models are supported via auto-detection. The service maintains a known list of USB Vendor ID (`0x04D8`) and Product ID combinations covering all released models (UC, UC Omega, Alpha, Lync variants). On startup and after reconnect, hidapi enumerates connected HID devices and matches against this list.

### Hot-Plug

A dedicated blocking thread (spawned via `tokio::task::spawn_blocking`) polls the device handle every 100ms:

- If the handle becomes invalid (device unplugged), logs the disconnect event at `info` level and sets the shared handle to `None`.
- If no handle is open, attempts to re-enumerate and open a matching device. On success, logs the connect event at `info` level and restores the last-set color/state.

### HID Protocol

Kuando Busylights accept 64-byte HID feature reports. The command structure varies by model. The device layer uses a per-PID protocol variant enum to select the correct report format. The report encodes:

- RGB color values (3 bytes)
- Brightness byte (0 = off, 100 = full on)
- Keepalive field (model-dependent position)

### Keepalive

Models including the UC Omega and Alpha require a keepalive HID report every ~2 seconds or they revert to their default state. The USB polling thread sends the keepalive automatically in its loop whenever a device handle is open.

### Testability

The USB layer is abstracted behind a `BuslightDevice` trait:

```rust
trait BuslightDevice: Send + Sync {
    fn set_color(&self, r: u8, g: u8, b: u8, on: bool) -> Result<()>;
    fn is_connected(&self) -> bool;
}
```

The real implementation uses hidapi. A mock implementation is used in integration tests, returning configurable results without USB hardware.

---

## HTTP / API Layer

### Endpoints

All endpoints are under `/api/`.

#### `GET /api`
Serves the Swagger UI. Unauthenticated.

#### `GET /api/openapi.json`
Serves the OpenAPI 3.0 spec. Unauthenticated.

#### `GET /api/light`
Returns the current busylight status. Requires authentication.

Response body:
```json
{ "connected": true, "on": true, "r": 255, "g": 0, "b": 0 }
```

#### `POST /api/light`
Sets the busylight color and on/off state. Requires authentication.

Request body:
```json
{ "on": true, "r": 255, "g": 0, "b": 0 }
```

### HTTP Response Codes

| Situation | Code |
|---|---|
| Success (GET) | `200 OK` |
| Success (POST) | `200 OK` |
| Invalid JSON / missing fields | `400 Bad Request` |
| Missing or malformed auth headers | `401 Unauthorized` |
| Invalid HMAC signature | `403 Forbidden` |
| Timestamp outside ±30s window | `403 Forbidden` + `X-Server-Time` response header |
| Busylight not connected | `503 Service Unavailable` |
| Internal error | `500 Internal Server Error` |

All error responses include a JSON body:
```json
{ "error": "human-readable description" }
```

---

## Authentication

### PSK Generation

On first start, if `[auth] psk` is empty in the config, the server generates a cryptographically random 32-byte key, encodes it as base64url, and writes it back to `/etc/rustylight/rustylight.conf`. The config file is created with permissions `0600` owned by the `rustylight` service user. Admins may pre-populate the `psk` field for backup/restore scenarios.

### Request Signing

Every request to `/api/light` (GET and POST) must include two HTTP headers:

```
X-Timestamp: 1747394400
X-Signature: <lowercase hex HMAC-SHA256>
```

The signature is computed as:

```
HMAC-SHA256(psk_bytes, timestamp_string + request_body)
```

Where:
- `psk_bytes` = base64url-decoded PSK from config
- `timestamp_string` = the exact string value of `X-Timestamp`
- `request_body` = raw request body bytes (empty string `""` for GET requests)

### Server Verification

1. Parse `X-Timestamp` — return `401` if header is absent or non-numeric.
2. Check `|server_unix_time - X-Timestamp| <= 30` seconds — return `403` with `X-Server-Time` response header if outside window.
3. Recompute HMAC and compare using constant-time comparison — return `403` if mismatch.

Authentication is checked before any busylight operation. The Swagger UI endpoints (`GET /api`, `GET /api/openapi.json`) are unauthenticated; the OpenAPI spec documents the auth scheme so clients can implement it.

---

## TLS

**Implementation:** `axum-server` with `rustls` backend. No OpenSSL dependency.

**Certificate type:** ECDSA P-256 self-signed certificate.

**Auto-generation:** On startup, if either `cert_file` or `key_file` is missing or unreadable, the server generates a new self-signed ECC certificate and writes both PEM files. The generated cert is valid for 10 years, with CN `rustylight` and the machine's local IP addresses as SANs.

**Config:**
```toml
[tls]
cert_file = "/etc/rustylight/tls.crt"
key_file  = "/etc/rustylight/tls.key"
port      = 8443
```

The port defaults to `8443` so the service does not require root privileges.

---

## Configuration

**Path:** `/etc/rustylight/rustylight.conf`

**Format:** TOML

**Full structure with defaults:**
```toml
[server]
port = 8443

[tls]
cert_file = "/etc/rustylight/tls.crt"
key_file  = "/etc/rustylight/tls.key"

[auth]
psk = ""   # auto-generated on first start if empty

[logging]
level    = "info"   # trace | debug | info | warn | error
log_file = "/var/log/rustylight/rustylight.log"
```

**Behavior:**
- The config directory and file are created by the package installer (postinst script).
- Missing optional keys fall back to defaults; the service never fails to start due to a missing optional field.
- `psk` is the only field the server writes back at runtime (first-start generation). All other fields are read-only from the server's perspective.
- Config is read once at startup. Changes require `systemctl restart rustylight`.

---

## Logging

**Implementation:** `tracing` + `tracing-subscriber` with a rolling file appender writing to `/var/log/rustylight/rustylight.log`.

**Format:** `TIMESTAMP LEVEL TARGET - MESSAGE` (ISO 8601 timestamps)

**Log levels:**

| Level | Content |
|---|---|
| `error` | Errors (USB failures, TLS errors, startup failures) |
| `warn` | + Recoverable issues (reconnect attempts, rejected auth requests) |
| `info` | + Busylight connect/disconnect events, service start/stop |
| `debug` | + All API requests: method, path, client IP, response code |
| `trace` | + Full request/response bodies, USB HID report bytes |

**Log rotation:** A logrotate config is installed at `/etc/logrotate.d/rustylight`:

```
/var/log/rustylight/rustylight.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    postrotate
        systemctl kill -s HUP rustylight || true
    endscript
}
```

- `rotate 30` — keeps 30 files (30 days), deletes older ones automatically
- `compress` + `delaycompress` — compresses rotated files, but leaves the most recent rotated file uncompressed for one cycle (files older than 2 days are compressed)
- `postrotate` HUP — causes the service to reopen its log file handle after rotation

---

## Systemd Service

**Unit file:** `/lib/systemd/system/rustylight.service`

```ini
[Unit]
Description=Rustylight Busylight Controller
After=network.target
Documentation=https://github.com/rustylight/rustylight-server

[Service]
Type=simple
User=rustylight
Group=rustylight
ExecStart=/usr/sbin/rustylight-server
Restart=on-failure
RestartSec=5s
SupplementaryGroups=plugdev
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/rustylight /etc/rustylight
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

**Package install (postinst):**
- Creates `rustylight` system user and group (no login shell, no home directory)
- Creates `/etc/rustylight/` and `/var/log/rustylight/` with correct ownership (`rustylight:rustylight`)
- Adds `rustylight` user to `plugdev` group for USB HID access without root
- Runs `systemctl enable --now rustylight`

**Package removal:**
- `prerm`: stops the service (`systemctl stop rustylight`)
- `postrm`: disables the unit (`systemctl disable rustylight`)
- Config files and logs are preserved on normal uninstall; only removed on `apt purge` / `rpm -e --allfiles`

---

## Packaging

### Build Targets

| Package | Arch | Rust target triple |
|---|---|---|
| `.deb` | amd64 | `x86_64-unknown-linux-gnu` |
| `.deb` | i386 | `i686-unknown-linux-gnu` |
| `.deb` | arm64 | `aarch64-unknown-linux-gnu` |
| `.deb` | armv7 | `armv7-unknown-linux-gnueabihf` |
| `.rpm` | x86_64 | `x86_64-unknown-linux-gnu` |
| `.rpm` | aarch64 | `aarch64-unknown-linux-gnu` |

The arm64 and armv7 `.deb` packages cover Raspberry Pi 3, Pi 4, and Pi Zero 2W running 64-bit and 32-bit Raspberry Pi OS respectively.

### Tools

- **`cargo-deb`** — generates `.deb` packages from metadata in `Cargo.toml` `[package.metadata.deb]`; includes postinst/prerm/postrm scripts and the logrotate config as package assets
- **`cargo-generate-rpm`** — generates `.rpm` packages from `[package.metadata.generate-rpm]` in `Cargo.toml`
- **`cross`** — Docker-based cross-compilation, handles the C toolchain required for hidapi-rs on non-native targets

### RPM Compatibility

RPMs are built inside an AlmaLinux 8 container to ensure glibc 2.17+ compatibility, making the same binary installable on RHEL/Rocky/Alma Linux 8, 9, and 10.

### Release Artifacts

Packages are named:
- `rustylight-server_<version>_<arch>.deb`
- `rustylight-server-<version>-1.<arch>.rpm`

---

## CI / CD

### Workflows

**`ci.yml`** — triggered on every push and pull request:
1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo test` (native, on `ubuntu-latest`)
4. Build check for all 6 target triples via `cross` (compile only, no packaging)

**`release.yml`** — triggered on version tag push (`v*.*.*`):
1. Matrix build for all 6 targets via `cross`
2. `cargo-deb` for the 4 `.deb` targets
3. `cargo-generate-rpm` inside an AlmaLinux 8 container for the 2 `.rpm` targets
4. Upload all 6 packages as GitHub Release assets

### Dependabot

`.github/dependabot.yml` configures:
- Weekly `cargo` dependency update PRs
- Weekly `github-actions` version pin update PRs

### Caching

`actions/cache` caches `~/.cargo/registry`, `~/.cargo/git`, and `target/` directories between runs, keyed by `Cargo.lock` hash.

---

## Testing

### Unit Tests

Located in-module (`#[cfg(test)]`), covering:
- HMAC signature computation and verification
- Timestamp window validation (boundary conditions: exactly ±30s, ±31s)
- Config file parsing (all fields, missing optional fields, empty PSK)
- HID report construction for each model variant
- Log level parsing

### Integration Tests

Located in `tests/`, using the mock `BuslightDevice` implementation:
- All endpoints with valid authentication → correct `2xx` responses and bodies
- Missing `X-Timestamp` or `X-Signature` headers → `401`
- Incorrect HMAC → `403`
- Timestamp outside ±30s window → `403` with `X-Server-Time` header
- Valid auth but device not connected → `503`
- Malformed POST body → `400`

Integration tests use `axum::test` (no real network) against the actual router with the mock USB backend.

---

## Developer Documentation

**`README.md`:**
- What it is and what hardware it supports
- Installation instructions (`.deb` / `.rpm`)
- Configuration reference
- How to obtain and distribute the PSK to clients
- API usage examples with `curl`
- Systemd service management (`start`, `stop`, `status`, `logs`)

**`CONTRIBUTING.md`:**
- Prerequisites (Rust stable, `cross`, Docker for cross-compilation)
- Local build: `cargo build`
- Running tests: `cargo test`
- Cross-compiling for a specific target
- How to cut a release (tag + push)

**Inline rustdoc** on all public types, traits, and functions.

**OpenAPI spec** served at `/api/openapi.json` is the authoritative API reference, documenting all endpoints, request/response schemas, authentication headers, and error codes.
