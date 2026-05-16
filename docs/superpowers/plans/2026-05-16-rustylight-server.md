# rustylight-server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust Linux daemon that exposes a TLS-encrypted, HMAC-authenticated REST API for controlling a Kuando Busylight USB device, packaged as `.deb` and `.rpm` for multiple architectures.

**Architecture:** Single Tokio process with an async Axum HTTP server and a dedicated `std::thread` for USB HID polling/keepalive. Shared mutable state (`Arc<Mutex<SharedState>>`) is the only coordination point between the two. TLS is handled by rustls with an auto-generated self-signed ECDSA P-256 cert. Authentication uses HMAC-SHA256 over timestamp + request body, checked on every `/api/light` request.

**Tech Stack:** Rust stable (1.75+), axum 0.7, axum-server 0.6 (rustls), hidapi 2, rcgen 0.13, hmac + sha2, utoipa 4, utoipa-swagger-ui 7, tracing + tracing-appender, toml 0.8, base64 0.22, rand 0.8, thiserror 1, anyhow 1, cargo-deb, cargo-generate-rpm, `cross` (CI only)

---

## File Map

```
rustylight-server/
├── Cargo.toml
├── src/
│   ├── main.rs                   startup orchestration, signal handling
│   ├── config.rs                 Config struct, TOML load/save, PSK generation
│   ├── logging.rs                tracing subscriber init, file appender
│   ├── tls.rs                    rustls ServerConfig, ECC cert auto-generation
│   ├── device/
│   │   ├── mod.rs                BuslightDevice trait, LightState, DeviceError
│   │   ├── models.rs             VID/PID table, ModelVariant enum
│   │   ├── report.rs             64-byte HID report builder
│   │   └── manager.rs            USB polling loop, hot-plug, keepalive
│   └── api/
│       ├── mod.rs                AppState, SharedState, Axum router
│       ├── auth.rs               HMAC request extractor/middleware
│       ├── handlers.rs           GET/POST /api/light
│       └── openapi.rs            utoipa spec + Swagger UI route
├── tests/
│   ├── common/mod.rs             mock device, test app builder helpers
│   ├── test_auth.rs              auth header scenario tests
│   ├── test_api.rs               endpoint status code + body tests
│   └── test_config.rs            config parsing unit tests
└── packaging/
    ├── rustylight.service         systemd unit file
    ├── rustylight.logrotate       logrotate config
    ├── postinst                   deb post-install script
    ├── prerm                      deb pre-remove script
    └── postrm                     deb post-remove script
```

---

## Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs` (stub)
- Create: `src/config.rs` (stub)
- Create: `src/logging.rs` (stub)
- Create: `src/tls.rs` (stub)
- Create: `src/device/mod.rs` (stub)
- Create: `src/device/models.rs` (stub)
- Create: `src/device/report.rs` (stub)
- Create: `src/device/manager.rs` (stub)
- Create: `src/api/mod.rs` (stub)
- Create: `src/api/auth.rs` (stub)
- Create: `src/api/handlers.rs` (stub)
- Create: `src/api/openapi.rs` (stub)

- [ ] **Step 1: Write `Cargo.toml`**

```toml
[package]
name = "rustylight-server"
version = "0.1.0"
edition = "2021"
authors = ["florian@taeger.cc"]
description = "HTTP REST API server for Kuando Busylight USB device control"
license = "MIT"
repository = "https://github.com/rustylight/rustylight-server"

[[bin]]
name = "rustylight-server"
path = "src/main.rs"

[dependencies]
# HTTP server
axum = { version = "0.7", features = ["json", "macros"] }
axum-server = { version = "0.6", features = ["tls-rustls"] }
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.5", features = ["trace"] }

# USB HID
hidapi = "2.6"

# TLS + cert generation
rustls = "0.23"
rcgen = { version = "0.13", features = ["pem"] }
rustls-pemfile = "2"
time = "0.3"

# Auth
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"
base64 = { version = "0.22", features = [] }

# Config + serialisation
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
tracing-appender = "0.2"

# OpenAPI / Swagger
utoipa = { version = "4", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "7", features = ["axum"] }

# Utilities
rand = "0.8"
thiserror = "1"
anyhow = "1"

[dev-dependencies]
tower = { version = "0.4", features = ["util"] }
http-body-util = "0.1"
bytes = "1"

[profile.release]
opt-level = 3
strip = true
lto = true
codegen-units = 1
```

- [ ] **Step 2: Create stub source files**

Create each file with just a module declaration or empty `pub use`:

`src/main.rs`:
```rust
mod config;
mod device;
mod api;
mod logging;
mod tls;

fn main() {}
```

`src/config.rs`, `src/logging.rs`, `src/tls.rs`: empty files.

`src/device/mod.rs`:
```rust
pub mod manager;
pub mod models;
pub mod report;
```

`src/device/models.rs`, `src/device/report.rs`, `src/device/manager.rs`: empty files.

`src/api/mod.rs`:
```rust
pub mod auth;
pub mod handlers;
pub mod openapi;
```

`src/api/auth.rs`, `src/api/handlers.rs`, `src/api/openapi.rs`: empty files.

- [ ] **Step 3: Verify it compiles**

```bash
cargo check
```

Expected: no errors (warnings about unused modules are fine).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml src/
git commit -m "feat: project scaffold with all dependencies"
```

---

## Task 2: Device Types (`src/device/mod.rs`)

**Files:**
- Modify: `src/device/mod.rs`

- [ ] **Step 1: Write tests**

Add to `src/device/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_state_default_is_off() {
        let state = LightState::default();
        assert!(!state.on);
        assert!(!state.blink);
        assert_eq!(state.r, 0);
    }

    #[test]
    fn light_state_serialises_without_blink_fields_when_not_blinking() {
        let state = LightState { on: true, r: 255, g: 0, b: 0, blink: false, ..Default::default() };
        let json = serde_json::to_value(&state).unwrap();
        assert!(json.get("blink_on_ms").is_none());
        assert!(json.get("r2").is_none());
    }

    #[test]
    fn light_state_serialises_blink_fields_when_blinking() {
        let state = LightState {
            on: true, r: 255, g: 0, b: 0,
            blink: true, blink_on_ms: Some(500), blink_off_ms: Some(300),
            r2: Some(0), g2: Some(0), b2: Some(255),
        };
        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(json["blink_on_ms"], 500);
        assert_eq!(json["b2"], 255);
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test device::tests
```

Expected: compile error (types not defined yet).

- [ ] **Step 3: Implement types**

Replace `src/device/mod.rs` with:

```rust
pub mod manager;
pub mod models;
pub mod report;

use thiserror::Error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct LightState {
    pub on: bool,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    #[serde(default)]
    pub blink: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blink_on_ms: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blink_off_ms: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r2: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub g2: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b2: Option<u8>,
}

impl LightState {
    pub fn effective_blink_on_ms(&self) -> u16 {
        self.blink_on_ms.unwrap_or(500)
    }

    pub fn effective_blink_off_ms(&self) -> u16 {
        self.blink_off_ms.unwrap_or(500)
    }
}

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("device not connected")]
    NotConnected,
    #[error("HID error: {0}")]
    Hid(String),
}

pub trait BuslightDevice: Send + Sync {
    fn set_state(&self, state: &LightState) -> Result<(), DeviceError>;
    fn is_connected(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_state_default_is_off() {
        let state = LightState::default();
        assert!(!state.on);
        assert!(!state.blink);
        assert_eq!(state.r, 0);
    }

    #[test]
    fn light_state_serialises_without_blink_fields_when_not_blinking() {
        let state = LightState { on: true, r: 255, g: 0, b: 0, blink: false, ..Default::default() };
        let json = serde_json::to_value(&state).unwrap();
        assert!(json.get("blink_on_ms").is_none());
        assert!(json.get("r2").is_none());
    }

    #[test]
    fn light_state_serialises_blink_fields_when_blinking() {
        let state = LightState {
            on: true, r: 255, g: 0, b: 0,
            blink: true, blink_on_ms: Some(500), blink_off_ms: Some(300),
            r2: Some(0), g2: Some(0), b2: Some(255),
        };
        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(json["blink_on_ms"], 500);
        assert_eq!(json["b2"], 255);
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test device::tests
```

Expected: all 3 pass.

- [ ] **Step 5: Commit**

```bash
git add src/device/mod.rs
git commit -m "feat: LightState, BuslightDevice trait, DeviceError"
```

---

## Task 3: Device Model Table (`src/device/models.rs`)

**Files:**
- Modify: `src/device/models.rs`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vid_pid_resolves_to_variant() {
        assert!(ModelVariant::from_vid_pid(0x04D8, 0xF848).is_some());
        assert!(ModelVariant::from_vid_pid(0x27BB, 0x3BCA).is_some());
    }

    #[test]
    fn unknown_vid_pid_returns_none() {
        assert!(ModelVariant::from_vid_pid(0xDEAD, 0xBEEF).is_none());
    }
}
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test device::models::tests
```

Expected: compile error.

- [ ] **Step 3: Implement**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelVariant {
    UcOmega,
    Alpha,
    Uc,
    Lync,
    Uc2,
}

pub struct KnownDevice {
    pub vid: u16,
    pub pid: u16,
    pub variant: ModelVariant,
    pub name: &'static str,
}

pub const KNOWN_DEVICES: &[KnownDevice] = &[
    KnownDevice { vid: 0x04D8, pid: 0xF848, variant: ModelVariant::UcOmega, name: "Busylight UC Omega" },
    KnownDevice { vid: 0x04D8, pid: 0xF8F8, variant: ModelVariant::Uc,      name: "Busylight UC" },
    KnownDevice { vid: 0x04D8, pid: 0x2013, variant: ModelVariant::Lync,    name: "Busylight Lync" },
    KnownDevice { vid: 0x04D8, pid: 0x2014, variant: ModelVariant::Lync,    name: "Busylight Lync Plus" },
    KnownDevice { vid: 0x27BB, pid: 0x3BCA, variant: ModelVariant::Alpha,   name: "Busylight Alpha" },
    KnownDevice { vid: 0x27BB, pid: 0x3BCB, variant: ModelVariant::Alpha,   name: "Busylight Alpha (v2)" },
    KnownDevice { vid: 0x27BB, pid: 0x3BC8, variant: ModelVariant::Uc2,     name: "Busylight UC2" },
    KnownDevice { vid: 0x27BB, pid: 0x3BC9, variant: ModelVariant::Uc2,     name: "Busylight UC2 (v2)" },
];

impl ModelVariant {
    pub fn from_vid_pid(vid: u16, pid: u16) -> Option<ModelVariant> {
        KNOWN_DEVICES.iter()
            .find(|d| d.vid == vid && d.pid == pid)
            .map(|d| d.variant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vid_pid_resolves_to_variant() {
        assert!(ModelVariant::from_vid_pid(0x04D8, 0xF848).is_some());
        assert!(ModelVariant::from_vid_pid(0x27BB, 0x3BCA).is_some());
    }

    #[test]
    fn unknown_vid_pid_returns_none() {
        assert!(ModelVariant::from_vid_pid(0xDEAD, 0xBEEF).is_none());
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test device::models::tests
```

Expected: 2 pass.

- [ ] **Step 5: Commit**

```bash
git add src/device/models.rs
git commit -m "feat: Busylight VID/PID table and ModelVariant"
```

---

## Task 4: HID Report Builder (`src/device/report.rs`)

**Files:**
- Modify: `src/device/report.rs`

> **Protocol note:** The Kuando Busylight accepts 64-byte HID feature reports. The format below is based on open-source reverse-engineering (see the Python `busylight` library from plux and the JavaScript `busylight` library from porsager). Verify byte positions against a working reference before shipping — capture USB traffic with Wireshark + usbmon if the device behaves unexpectedly.
>
> Report layout: 8 steps × 7 bytes (R, G, B, on_hi, on_lo, off_hi, off_lo) = 56 bytes, + 1 byte repeat count, + 7 bytes ring/audio (zeroed), = 64 bytes total. Timing unit: 10 ms. Keepalive: the last byte of the ring section is a "keepalive timeout" in seconds; the USB polling thread must re-send within this window or the device resets. Set to `0x05` (5 s) so the 2-second polling loop has margin.

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::LightState;

    #[test]
    fn report_is_64_bytes() {
        let state = LightState::default();
        let report = build_report(&state);
        assert_eq!(report.len(), 64);
    }

    #[test]
    fn steady_red_sets_step0_color() {
        let state = LightState { on: true, r: 255, g: 0, b: 0, blink: false, ..Default::default() };
        let report = build_report(&state);
        assert_eq!(report[0], 255); // R
        assert_eq!(report[1], 0);   // G
        assert_eq!(report[2], 0);   // B
        // on_time = 0xFFFF (steady), off_time = 0
        assert_eq!(report[3], 0xFF);
        assert_eq!(report[4], 0xFF);
        assert_eq!(report[5], 0x00);
        assert_eq!(report[6], 0x00);
    }

    #[test]
    fn off_state_all_zeros_except_keepalive() {
        let state = LightState { on: false, ..Default::default() };
        let report = build_report(&state);
        // step 0 color bytes must be 0
        assert_eq!(report[0], 0);
        assert_eq!(report[1], 0);
        assert_eq!(report[2], 0);
        // keepalive byte at index 63 must be non-zero
        assert_eq!(report[63], 0x05);
    }

    #[test]
    fn blink_color_to_off_sets_on_off_timing() {
        let state = LightState {
            on: true, r: 0, g: 255, b: 0,
            blink: true, blink_on_ms: Some(500), blink_off_ms: Some(300),
            ..Default::default()
        };
        let report = build_report(&state);
        // on_time = 500ms / 10 = 50 = 0x0032
        assert_eq!(report[3], 0x00);
        assert_eq!(report[4], 50);
        // off_time = 300ms / 10 = 30 = 0x001E
        assert_eq!(report[5], 0x00);
        assert_eq!(report[6], 30);
    }

    #[test]
    fn blink_two_colors_sets_step1_color() {
        let state = LightState {
            on: true, r: 255, g: 0, b: 0,
            blink: true, blink_on_ms: Some(500), blink_off_ms: Some(500),
            r2: Some(0), g2: Some(0), b2: Some(255),
        };
        let report = build_report(&state);
        // step 0: color1
        assert_eq!(report[0], 255);
        assert_eq!(report[2], 0);
        // step 1 starts at byte 7: color2
        assert_eq!(report[7], 0);    // R2
        assert_eq!(report[8], 0);    // G2
        assert_eq!(report[9], 255);  // B2
    }
}
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test device::report::tests
```

Expected: compile error (`build_report` not defined).

- [ ] **Step 3: Implement**

```rust
use crate::device::LightState;

const STEP_SIZE: usize = 7;
const NUM_STEPS: usize = 8;
const REPORT_SIZE: usize = 64; // 8×7 + 1 repeat + 7 ring = 64
const KEEPALIVE_IDX: usize = 63;
const KEEPALIVE_SECS: u8 = 5;

pub fn build_report(state: &LightState) -> [u8; REPORT_SIZE] {
    let mut report = [0u8; REPORT_SIZE];

    if !state.on {
        report[KEEPALIVE_IDX] = KEEPALIVE_SECS;
        return report;
    }

    if state.blink {
        let on_ticks = (state.effective_blink_on_ms() / 10) as u16;
        let off_ticks = (state.effective_blink_off_ms() / 10) as u16;

        let r2 = state.r2.unwrap_or(0);
        let g2 = state.g2.unwrap_or(0);
        let b2 = state.b2.unwrap_or(0);
        let two_color = r2 > 0 || g2 > 0 || b2 > 0;

        if two_color {
            // Step 0: color1 on for blink_on_ms, off for 0
            write_step(&mut report, 0, state.r, state.g, state.b, on_ticks, 0);
            // Step 1: color2 on for blink_off_ms, off for 0
            write_step(&mut report, 1, r2, g2, b2, off_ticks, 0);
        } else {
            // Step 0: color on for blink_on_ms, off for blink_off_ms
            write_step(&mut report, 0, state.r, state.g, state.b, on_ticks, off_ticks);
        }
    } else {
        // Steady: on_time = max (0xFFFF), off_time = 0
        write_step(&mut report, 0, state.r, state.g, state.b, 0xFFFF, 0);
    }

    // repeat = 0 (infinite, byte at index 56)
    report[NUM_STEPS * STEP_SIZE] = 0x00;
    report[KEEPALIVE_IDX] = KEEPALIVE_SECS;
    report
}

fn write_step(report: &mut [u8; REPORT_SIZE], step: usize, r: u8, g: u8, b: u8, on_ticks: u16, off_ticks: u16) {
    let base = step * STEP_SIZE;
    report[base]     = r;
    report[base + 1] = g;
    report[base + 2] = b;
    report[base + 3] = (on_ticks >> 8) as u8;
    report[base + 4] = (on_ticks & 0xFF) as u8;
    report[base + 5] = (off_ticks >> 8) as u8;
    report[base + 6] = (off_ticks & 0xFF) as u8;
}

#[cfg(test)]
mod tests {
    // (paste tests from Step 1 here)
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test device::report::tests
```

Expected: all 5 pass.

- [ ] **Step 5: Commit**

```bash
git add src/device/report.rs
git commit -m "feat: HID report builder with steady and blink step-sequence"
```

---

## Task 5: Config Module (`src/config.rs`)

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_correct() {
        let cfg = Config::default();
        assert_eq!(cfg.server.port, 8443);
        assert_eq!(cfg.tls.cert_file, "/etc/rustylight/tls.crt");
        assert_eq!(cfg.logging.level, "info");
        assert_eq!(cfg.auth.psk, "");
    }

    #[test]
    fn parses_full_toml() {
        let toml = r#"
[server]
port = 9443

[tls]
cert_file = "/tmp/tls.crt"
key_file = "/tmp/tls.key"

[auth]
psk = "abc123"

[logging]
level = "debug"
log_file = "/tmp/rustylight.log"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.server.port, 9443);
        assert_eq!(cfg.auth.psk, "abc123");
        assert_eq!(cfg.logging.level, "debug");
    }

    #[test]
    fn parses_partial_toml_with_defaults() {
        let toml = "[auth]\npsk = \"mykey\"";
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.server.port, 8443);
        assert_eq!(cfg.auth.psk, "mykey");
    }

    #[test]
    fn generates_non_empty_psk() {
        let psk = generate_psk();
        assert!(!psk.is_empty());
        // verify it is valid base64url
        use base64::{engine::general_purpose::URL_SAFE, Engine};
        assert!(URL_SAFE.decode(&psk).is_ok());
    }
}
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test config::tests
```

Expected: compile error.

- [ ] **Step 3: Implement**

```rust
use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const CONFIG_PATH: &str = "/etc/rustylight/rustylight.conf";

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub tls: TlsConfig,
    pub auth: AuthConfig,
    pub logging: LoggingConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            tls: TlsConfig::default(),
            auth: AuthConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub port: u16,
}
impl Default for ServerConfig {
    fn default() -> Self { Self { port: 8443 } }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TlsConfig {
    pub cert_file: String,
    pub key_file: String,
}
impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_file: "/etc/rustylight/tls.crt".to_owned(),
            key_file: "/etc/rustylight/tls.key".to_owned(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AuthConfig {
    pub psk: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
    pub log_file: String,
}
impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_owned(),
            log_file: "/var/log/rustylight/rustylight.log".to_owned(),
        }
    }
}

pub fn load_or_create(path: &str) -> Result<Config> {
    if Path::new(path).exists() {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading config at {path}"))?;
        let cfg: Config = toml::from_str(&raw)
            .with_context(|| "parsing config TOML")?;
        Ok(cfg)
    } else {
        Ok(Config::default())
    }
}

pub fn ensure_psk(cfg: &mut Config, path: &str) -> Result<()> {
    if cfg.auth.psk.is_empty() {
        cfg.auth.psk = generate_psk();
        save(cfg, path).context("writing generated PSK to config")?;
    }
    Ok(())
}

pub fn decode_psk(cfg: &Config) -> Result<Vec<u8>> {
    URL_SAFE.decode(&cfg.auth.psk).context("decoding PSK from base64url")
}

fn save(cfg: &Config, path: &str) -> Result<()> {
    let content = toml::to_string_pretty(cfg).context("serialising config")?;
    std::fs::write(path, content).with_context(|| format!("writing config to {path}"))?;
    Ok(())
}

pub fn generate_psk() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_correct() {
        let cfg = Config::default();
        assert_eq!(cfg.server.port, 8443);
        assert_eq!(cfg.tls.cert_file, "/etc/rustylight/tls.crt");
        assert_eq!(cfg.logging.level, "info");
        assert_eq!(cfg.auth.psk, "");
    }

    #[test]
    fn parses_full_toml() {
        let toml_str = r#"
[server]
port = 9443

[tls]
cert_file = "/tmp/tls.crt"
key_file = "/tmp/tls.key"

[auth]
psk = "abc123"

[logging]
level = "debug"
log_file = "/tmp/rustylight.log"
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.server.port, 9443);
        assert_eq!(cfg.auth.psk, "abc123");
        assert_eq!(cfg.logging.level, "debug");
    }

    #[test]
    fn parses_partial_toml_with_defaults() {
        let toml_str = "[auth]\npsk = \"mykey\"";
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.server.port, 8443);
        assert_eq!(cfg.auth.psk, "mykey");
    }

    #[test]
    fn generates_non_empty_psk() {
        let psk = generate_psk();
        assert!(!psk.is_empty());
        assert!(URL_SAFE.decode(&psk).is_ok());
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test config::tests
```

Expected: all 4 pass.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: TOML config with PSK generation and base64url encoding"
```

---

## Task 6: Logging Setup (`src/logging.rs`)

**Files:**
- Modify: `src/logging.rs`

> Log rotation is handled by the `logrotate` config (Task 15). The `tracing-appender` here only needs to open a non-rolling file handle. The logrotate `copytruncate` directive truncates the file in-place so the process never needs to reopen it.

- [ ] **Step 1: Implement (no separate unit test; test via integration)**

```rust
use anyhow::Result;
use tracing_subscriber::{fmt, EnvFilter};

pub fn init(level: &str, log_file: &str) -> Result<()> {
    let filter = EnvFilter::try_new(level)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let log_dir = std::path::Path::new(log_file)
        .parent()
        .unwrap_or(std::path::Path::new("/var/log/rustylight"));
    let file_name = std::path::Path::new(log_file)
        .file_name()
        .unwrap_or(std::ffi::OsStr::new("rustylight.log"));

    let file_appender = tracing_appender::rolling::never(log_dir, file_name);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Leak the guard so it lives for the process lifetime
    Box::leak(Box::new(guard));

    fmt()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    Ok(())
}
```

- [ ] **Step 2: Check it compiles**

```bash
cargo check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/logging.rs
git commit -m "feat: tracing subscriber with non-rolling file appender"
```

---

## Task 7: TLS Module (`src/tls.rs`)

**Files:**
- Modify: `src/tls.rs`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_pem_cert_and_key() {
        let (cert_pem, key_pem) = generate_self_signed().unwrap();
        assert!(cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(key_pem.contains("BEGIN"));
    }

    #[test]
    fn load_or_generate_creates_files_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let cert_path = dir.path().join("tls.crt").to_str().unwrap().to_owned();
        let key_path  = dir.path().join("tls.key").to_str().unwrap().to_owned();
        load_or_generate(&cert_path, &key_path).unwrap();
        assert!(std::path::Path::new(&cert_path).exists());
        assert!(std::path::Path::new(&key_path).exists());
    }
}
```

Add `tempfile = "3"` to `[dev-dependencies]` in `Cargo.toml`.

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test tls::tests
```

Expected: compile error.

- [ ] **Step 3: Implement**

```rust
use anyhow::{Context, Result};
use axum_server::tls_rustls::RustlsConfig;
use rcgen::{CertificateParams, KeyPair, PKCS_ECDSA_P256_SHA256};
use std::path::Path;

pub fn generate_self_signed() -> Result<(String, String)> {
    let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
        .context("generating ECC key pair")?;
    let mut params = CertificateParams::new(vec!["rustylight".to_owned()])
        .context("creating cert params")?;
    params.not_after = time::OffsetDateTime::now_utc()
        .checked_add(time::Duration::days(3650))
        .context("computing cert expiry")?;
    let cert = params.self_signed(&key_pair).context("self-signing cert")?;
    Ok((cert.pem(), key_pair.serialize_pem()))
}

pub fn load_or_generate(cert_path: &str, key_path: &str) -> Result<()> {
    let cert_missing = !Path::new(cert_path).exists();
    let key_missing  = !Path::new(key_path).exists();
    if cert_missing || key_missing {
        let (cert_pem, key_pem) = generate_self_signed()
            .context("generating self-signed TLS certificate")?;
        std::fs::write(cert_path, cert_pem)
            .with_context(|| format!("writing cert to {cert_path}"))?;
        std::fs::write(key_path, key_pem)
            .with_context(|| format!("writing key to {key_path}"))?;
        tracing::info!("generated self-signed TLS certificate at {cert_path}");
    }
    Ok(())
}

pub async fn rustls_config(cert_path: &str, key_path: &str) -> Result<RustlsConfig> {
    RustlsConfig::from_pem_file(cert_path, key_path)
        .await
        .context("loading TLS config from PEM files")
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test tls::tests
```

Expected: both pass.

- [ ] **Step 5: Commit**

```bash
git add src/tls.rs Cargo.toml
git commit -m "feat: TLS module with ECDSA P-256 self-signed cert generation"
```

---

## Task 8: USB Device Manager (`src/device/manager.rs`)

**Files:**
- Modify: `src/device/manager.rs`
- Modify: `src/device/mod.rs` (re-export `SharedState`)

> This module runs in a `std::thread`, not a Tokio task, because hidapi is synchronous and the thread runs indefinitely. It holds the `HidDevice` handle internally and applies state from `SharedState` every 2 seconds (keepalive) or immediately when state changes.

- [ ] **Step 1: Implement**

Add to `src/device/mod.rs` (after existing content):

```rust
use std::sync::{Arc, Mutex};

pub struct SharedState {
    pub connected: bool,
    pub light_state: LightState,
    pub state_dirty: bool, // true when HTTP handler wrote a new state
}

impl Default for SharedState {
    fn default() -> Self {
        Self { connected: false, light_state: LightState::default(), state_dirty: false }
    }
}
```

Create `src/device/manager.rs`:

```rust
use crate::device::{models::{ModelVariant, KNOWN_DEVICES}, report::build_report, LightState, SharedState};
use hidapi::HidApi;
use std::{sync::{Arc, Mutex}, time::Duration, thread};

pub fn spawn_usb_manager(shared: Arc<Mutex<SharedState>>) {
    thread::spawn(move || {
        run_loop(shared);
    });
}

fn run_loop(shared: Arc<Mutex<SharedState>>) {
    let api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("failed to initialise hidapi: {e}");
            return;
        }
    };

    let mut device: Option<hidapi::HidDevice> = None;
    let mut last_variant: Option<ModelVariant> = None;

    loop {
        if device.is_none() {
            device = try_connect(&api, &shared, &mut last_variant);
        }

        if let Some(ref dev) = device {
            let (state, dirty) = {
                let mut s = shared.lock().unwrap();
                let dirty = s.state_dirty;
                s.state_dirty = false;
                (s.light_state.clone(), dirty)
            };

            let report = build_report(&state);
            // Write the full 64-byte report; hidapi prepends report ID 0x00
            if dev.write(&report).is_err() {
                tracing::info!("Busylight disconnected");
                device = None;
                last_variant = None;
                shared.lock().unwrap().connected = false;
            } else if dirty {
                tracing::debug!("sent new state to Busylight");
            }
        }

        thread::sleep(Duration::from_secs(2));
    }
}

fn try_connect(
    api: &HidApi,
    shared: &Arc<Mutex<SharedState>>,
    last_variant: &mut Option<ModelVariant>,
) -> Option<hidapi::HidDevice> {
    // Re-enumerate devices on each attempt
    let api = HidApi::new().ok()?;
    for dev_info in api.device_list() {
        if let Some(variant) = ModelVariant::from_vid_pid(dev_info.vendor_id(), dev_info.product_id()) {
            match dev_info.open_device(&api) {
                Ok(dev) => {
                    *last_variant = Some(variant);
                    let mut s = shared.lock().unwrap();
                    s.connected = true;
                    s.state_dirty = true; // force state push on reconnect
                    tracing::info!(
                        "Busylight connected: {:?} ({:#06x}:{:#06x})",
                        variant, dev_info.vendor_id(), dev_info.product_id()
                    );
                    return Some(dev);
                }
                Err(e) => {
                    tracing::warn!("found Busylight but failed to open: {e}");
                }
            }
        }
    }
    None
}
```

- [ ] **Step 2: Check it compiles**

```bash
cargo check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/device/manager.rs src/device/mod.rs
git commit -m "feat: USB polling loop with hot-plug detection and keepalive"
```

---

## Task 9: AppState and Axum Router (`src/api/mod.rs`)

**Files:**
- Modify: `src/api/mod.rs`

- [ ] **Step 1: Implement**

```rust
pub mod auth;
pub mod handlers;
pub mod openapi;

use crate::device::SharedState;
use axum::{routing::{get, post}, Router};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub psk: Arc<Vec<u8>>,
    pub shared: Arc<Mutex<SharedState>>,
}

pub fn build_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/light", get(handlers::get_light))
        .route("/light", post(handlers::post_light))
        .with_state(state);

    let swagger_routes = openapi::swagger_router();

    Router::new()
        .nest("/api", api_routes)
        .merge(swagger_routes)
}
```

- [ ] **Step 2: Check**

```bash
cargo check
```

Expected: compile errors about undefined handlers — that's expected at this stage.

- [ ] **Step 3: Add placeholder handlers so it compiles**

In `src/api/handlers.rs`:

```rust
use axum::{extract::State, Json};
use crate::api::AppState;
use crate::device::LightState;

pub async fn get_light(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"connected": false, "on": false, "r": 0, "g": 0, "b": 0, "blink": false}))
}

pub async fn post_light(
    State(_state): State<AppState>,
    Json(_body): Json<LightState>,
) -> axum::http::StatusCode {
    axum::http::StatusCode::OK
}
```

In `src/api/openapi.rs`:

```rust
use axum::Router;

pub fn swagger_router() -> Router {
    Router::new()
}
```

- [ ] **Step 4: Check compiles**

```bash
cargo check
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/api/mod.rs src/api/handlers.rs src/api/openapi.rs
git commit -m "feat: AppState, Axum router, placeholder handlers"
```

---

## Task 10: Auth Middleware (`src/api/auth.rs`)

**Files:**
- Modify: `src/api/auth.rs`
- Modify: `src/api/handlers.rs` (add `AuthGuard` extractor to handlers)

- [ ] **Step 1: Write unit tests**

Add `src/api/auth.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn psk() -> Vec<u8> {
        b"test-psk-bytes-32-chars-xxxxxxxxx".to_vec()
    }

    #[test]
    fn valid_signature_returns_ok() {
        let ts = "1747394400";
        let body = b"{}";
        let sig = compute_signature(&psk(), ts, body);
        assert!(verify_signature(&psk(), ts, body, &sig));
    }

    #[test]
    fn wrong_signature_returns_false() {
        let ts = "1747394400";
        assert!(!verify_signature(&psk(), ts, b"{}", "deadbeef"));
    }

    #[test]
    fn timestamp_within_window_is_ok() {
        let now = current_unix_time();
        assert!(timestamp_in_window(now));
        assert!(timestamp_in_window(now + 29));
        assert!(timestamp_in_window(now - 29));
    }

    #[test]
    fn timestamp_outside_window_is_rejected() {
        let now = current_unix_time();
        assert!(!timestamp_in_window(now + 31));
        assert!(!timestamp_in_window(now - 31));
    }
}
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test api::auth::tests
```

Expected: compile error.

- [ ] **Step 3: Implement**

```rust
use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

pub const TIMESTAMP_WINDOW_SECS: u64 = 30;

pub fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn compute_signature(psk: &[u8], timestamp: &str, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(psk).expect("HMAC accepts any key size");
    mac.update(timestamp.as_bytes());
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

pub fn verify_signature(psk: &[u8], timestamp: &str, body: &[u8], provided_hex: &str) -> bool {
    let Ok(provided_bytes) = hex::decode(provided_hex) else { return false; };
    let mut mac = HmacSha256::new_from_slice(psk).expect("HMAC accepts any key size");
    mac.update(timestamp.as_bytes());
    mac.update(body);
    mac.verify_slice(&provided_bytes).is_ok()
}

pub fn timestamp_in_window(ts: u64) -> bool {
    let now = current_unix_time();
    now.abs_diff(ts) <= TIMESTAMP_WINDOW_SECS
}

pub struct AuthGuard(pub Bytes);

pub enum AuthError {
    MissingHeader(&'static str),
    NonNumericTimestamp,
    TimestampOutOfWindow { server_time: u64 },
    InvalidSignature,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::MissingHeader(h) => (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": format!("missing header: {h}")})),
            ).into_response(),
            AuthError::NonNumericTimestamp => (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": "X-Timestamp must be a unix timestamp"})),
            ).into_response(),
            AuthError::TimestampOutOfWindow { server_time } => {
                let mut resp = (
                    StatusCode::FORBIDDEN,
                    axum::Json(serde_json::json!({"error": "timestamp outside ±30s window"})),
                ).into_response();
                resp.headers_mut().insert(
                    "X-Server-Time",
                    server_time.to_string().parse().unwrap(),
                );
                resp
            }
            AuthError::InvalidSignature => (
                StatusCode::FORBIDDEN,
                axum::Json(serde_json::json!({"error": "invalid signature"})),
            ).into_response(),
        }
    }
}

#[axum::async_trait]
impl<S> FromRequest<S> for AuthGuard
where
    S: Send + Sync,
    crate::api::AppState: axum::extract::FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        use axum::extract::FromRef;
        let app_state = crate::api::AppState::from_ref(state);
        let psk = app_state.psk.as_ref();

        let ts_header = req
            .headers()
            .get("X-Timestamp")
            .ok_or(AuthError::MissingHeader("X-Timestamp"))?
            .to_str()
            .map_err(|_| AuthError::NonNumericTimestamp)?
            .to_owned();

        let sig_header = req
            .headers()
            .get("X-Signature")
            .ok_or(AuthError::MissingHeader("X-Signature"))?
            .to_str()
            .map_err(|_| AuthError::InvalidSignature)?
            .to_owned();

        let ts: u64 = ts_header
            .parse()
            .map_err(|_| AuthError::NonNumericTimestamp)?;

        if !timestamp_in_window(ts) {
            return Err(AuthError::TimestampOutOfWindow { server_time: current_unix_time() });
        }

        let body = Bytes::from_request(req, state)
            .await
            .unwrap_or_default();

        if !verify_signature(psk, &ts_header, &body, &sig_header) {
            return Err(AuthError::InvalidSignature);
        }

        Ok(AuthGuard(body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn psk() -> Vec<u8> {
        b"test-psk-bytes-32-chars-xxxxxxxxx".to_vec()
    }

    #[test]
    fn valid_signature_returns_ok() {
        let ts = "1747394400";
        let body = b"{}";
        let sig = compute_signature(&psk(), ts, body);
        assert!(verify_signature(&psk(), ts, body, &sig));
    }

    #[test]
    fn wrong_signature_returns_false() {
        let ts = "1747394400";
        assert!(!verify_signature(&psk(), ts, b"{}", "deadbeef"));
    }

    #[test]
    fn timestamp_within_window_is_ok() {
        let now = current_unix_time();
        assert!(timestamp_in_window(now));
        assert!(timestamp_in_window(now + 29));
        assert!(timestamp_in_window(now - 29));
    }

    #[test]
    fn timestamp_outside_window_is_rejected() {
        let now = current_unix_time();
        assert!(!timestamp_in_window(now + 31));
        assert!(!timestamp_in_window(now - 31));
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test api::auth::tests
```

Expected: all 4 pass.

- [ ] **Step 5: Commit**

```bash
git add src/api/auth.rs
git commit -m "feat: HMAC-SHA256 auth extractor with timestamp replay protection"
```

---

## Task 11: API Handlers (`src/api/handlers.rs`)

**Files:**
- Modify: `src/api/handlers.rs`

Replace the placeholder content:

- [ ] **Step 1: Write tests**

Add these tests inside `src/api/handlers.rs` (they'll compile after the implementation):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::LightState;

    #[test]
    fn validate_light_state_rejects_blink_ms_below_minimum() {
        let state = LightState {
            on: true, r: 255, g: 0, b: 0,
            blink: true, blink_on_ms: Some(40), blink_off_ms: Some(500),
            ..Default::default()
        };
        assert!(validate_post_body(&state).is_err());
    }

    #[test]
    fn validate_light_state_rejects_blink_ms_above_maximum() {
        let state = LightState {
            on: true, r: 255, g: 0, b: 0,
            blink: true, blink_on_ms: Some(500), blink_off_ms: Some(11000),
            ..Default::default()
        };
        assert!(validate_post_body(&state).is_err());
    }

    #[test]
    fn validate_light_state_accepts_valid_blink() {
        let state = LightState {
            on: true, r: 255, g: 0, b: 0,
            blink: true, blink_on_ms: Some(500), blink_off_ms: Some(500),
            ..Default::default()
        };
        assert!(validate_post_body(&state).is_ok());
    }
}
```

- [ ] **Step 2: Implement**

```rust
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use crate::{api::{auth::AuthGuard, AppState}, device::LightState};

pub fn validate_post_body(state: &LightState) -> Result<(), String> {
    if state.blink {
        let on_ms = state.blink_on_ms.unwrap_or(500);
        let off_ms = state.blink_off_ms.unwrap_or(500);
        if on_ms < 50 || on_ms > 10000 {
            return Err(format!("blink_on_ms must be 50–10000, got {on_ms}"));
        }
        if off_ms < 50 || off_ms > 10000 {
            return Err(format!("blink_off_ms must be 50–10000, got {off_ms}"));
        }
    }
    Ok(())
}

pub async fn get_light(
    _auth: AuthGuard,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let shared = state.shared.lock().unwrap();
    let mut body = serde_json::to_value(&shared.light_state).unwrap();
    body["connected"] = serde_json::Value::Bool(shared.connected);
    Json(body)
}

pub async fn post_light(
    AuthGuard(body_bytes): AuthGuard,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let light_state: LightState = match serde_json::from_slice(&body_bytes) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid JSON: {e}")})),
            ).into_response();
        }
    };

    if let Err(msg) = validate_post_body(&light_state) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": msg})),
        ).into_response();
    }

    let mut shared = state.shared.lock().unwrap();

    if !shared.connected {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "Busylight not connected"})),
        ).into_response();
    }

    shared.light_state = light_state;
    shared.state_dirty = true;

    (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::LightState;

    #[test]
    fn validate_light_state_rejects_blink_ms_below_minimum() {
        let state = LightState {
            on: true, r: 255, g: 0, b: 0,
            blink: true, blink_on_ms: Some(40), blink_off_ms: Some(500),
            ..Default::default()
        };
        assert!(validate_post_body(&state).is_err());
    }

    #[test]
    fn validate_light_state_rejects_blink_ms_above_maximum() {
        let state = LightState {
            on: true, r: 255, g: 0, b: 0,
            blink: true, blink_on_ms: Some(500), blink_off_ms: Some(11000),
            ..Default::default()
        };
        assert!(validate_post_body(&state).is_err());
    }

    #[test]
    fn validate_light_state_accepts_valid_blink() {
        let state = LightState {
            on: true, r: 255, g: 0, b: 0,
            blink: true, blink_on_ms: Some(500), blink_off_ms: Some(500),
            ..Default::default()
        };
        assert!(validate_post_body(&state).is_ok());
    }
}
```

- [ ] **Step 3: Run unit tests**

```bash
cargo test api::handlers::tests
```

Expected: 3 pass.

- [ ] **Step 4: Commit**

```bash
git add src/api/handlers.rs
git commit -m "feat: GET/POST /api/light handlers with validation"
```

---

## Task 12: OpenAPI / Swagger (`src/api/openapi.rs`)

**Files:**
- Modify: `src/api/openapi.rs`
- Modify: `src/api/handlers.rs` (add `#[utoipa::path]` annotations)
- Modify: `src/device/mod.rs` (add `#[derive(utoipa::ToSchema)]` to `LightState`)

- [ ] **Step 1: Add ToSchema to LightState**

In `src/device/mod.rs`, change `LightState` derive line from:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
```
to:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq, utoipa::ToSchema)]
```

- [ ] **Step 2: Annotate handlers**

In `src/api/handlers.rs`, add above `pub async fn get_light`:

```rust
#[utoipa::path(
    get,
    path = "/api/light",
    responses(
        (status = 200, description = "Current busylight state", body = serde_json::Value),
        (status = 401, description = "Missing auth headers"),
        (status = 403, description = "Invalid signature or timestamp"),
        (status = 503, description = "Busylight not connected"),
    ),
    params(
        ("X-Timestamp" = String, Header, description = "Unix timestamp (seconds UTC)"),
        ("X-Signature" = String, Header, description = "HMAC-SHA256(psk, timestamp+body) as lowercase hex"),
    )
)]
```

Add above `pub async fn post_light`:

```rust
#[utoipa::path(
    post,
    path = "/api/light",
    request_body = LightState,
    responses(
        (status = 200, description = "State applied"),
        (status = 400, description = "Invalid request body"),
        (status = 401, description = "Missing auth headers"),
        (status = 403, description = "Invalid signature or timestamp"),
        (status = 503, description = "Busylight not connected"),
    ),
    params(
        ("X-Timestamp" = String, Header, description = "Unix timestamp (seconds UTC)"),
        ("X-Signature" = String, Header, description = "HMAC-SHA256(psk, timestamp+body) as lowercase hex"),
    )
)]
```

- [ ] **Step 3: Implement openapi.rs**

```rust
use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use crate::device::LightState;
use crate::api::handlers;

#[derive(OpenApi)]
#[openapi(
    paths(handlers::get_light, handlers::post_light),
    components(schemas(LightState)),
    info(
        title = "rustylight-server API",
        version = "0.1.0",
        description = "REST API for controlling a Kuando Busylight USB device.\n\n\
            ## Authentication\n\
            Every `/api/light` request requires two headers:\n\
            - `X-Timestamp`: Unix timestamp (seconds, UTC)\n\
            - `X-Signature`: `HMAC-SHA256(psk_bytes, timestamp_string + request_body)` as lowercase hex\n\n\
            The server rejects requests with timestamps outside ±30 seconds of server time."
    )
)]
pub struct ApiDoc;

pub fn swagger_router() -> Router {
    Router::new().merge(
        SwaggerUi::new("/api")
            .url("/api/openapi.json", ApiDoc::openapi()),
    )
}
```

- [ ] **Step 4: Check it compiles**

```bash
cargo check
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/api/openapi.rs src/api/handlers.rs src/device/mod.rs
git commit -m "feat: OpenAPI spec and Swagger UI at /api"
```

---

## Task 13: Main Entry Point (`src/main.rs`)

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement**

```rust
mod api;
mod config;
mod device;
mod logging;
mod tls;

use anyhow::{Context, Result};
use axum_server::Handle;
use device::{manager, SharedState};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    let mut cfg = config::load_or_create(config::CONFIG_PATH)
        .context("loading configuration")?;

    logging::init(&cfg.logging.level, &cfg.logging.log_file)
        .context("initialising logging")?;

    tracing::info!("rustylight-server starting");

    config::ensure_psk(&mut cfg, config::CONFIG_PATH)
        .context("ensuring PSK is set")?;

    let psk_bytes = config::decode_psk(&cfg).context("decoding PSK")?;

    tls::load_or_generate(&cfg.tls.cert_file, &cfg.tls.key_file)
        .context("ensuring TLS certificate")?;

    let shared = Arc::new(Mutex::new(SharedState::default()));
    manager::spawn_usb_manager(Arc::clone(&shared));

    let state = api::AppState {
        psk: Arc::new(psk_bytes),
        shared: Arc::clone(&shared),
    };

    let router = api::build_router(state);
    let addr: SocketAddr = format!("0.0.0.0:{}", cfg.server.port).parse()?;
    let tls_config = tls::rustls_config(&cfg.tls.cert_file, &cfg.tls.key_file)
        .await
        .context("loading TLS config")?;

    let handle = Handle::new();
    let handle_clone = handle.clone();

    tokio::spawn(async move {
        shutdown_signal().await;
        tracing::info!("shutdown signal received");
        handle_clone.graceful_shutdown(Some(std::time::Duration::from_secs(5)));
    });

    tracing::info!("listening on https://{addr}");
    axum_server::bind_rustls(addr, tls_config)
        .handle(handle)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .context("server error")?;

    tracing::info!("rustylight-server stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async { signal::ctrl_c().await.expect("failed to install Ctrl+C handler") };
    #[cfg(unix)]
    let sigterm = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {}
        _ = sigterm => {}
    }
}
```

- [ ] **Step 2: Build in debug mode**

```bash
cargo build
```

Expected: binary at `target/debug/rustylight-server`. Warnings are fine; errors are not.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: main entry point with startup orchestration and graceful shutdown"
```

---

## Task 14: Integration Tests

**Files:**
- Create: `tests/common/mod.rs`
- Create: `tests/test_auth.rs`
- Create: `tests/test_api.rs`

- [ ] **Step 1: Create mock device and test app builder (`tests/common/mod.rs`)**

```rust
use rustylight_server::api::{build_router, AppState};
use rustylight_server::device::{BuslightDevice, DeviceError, LightState, SharedState};
use std::sync::{Arc, Mutex};

pub struct MockDevice {
    pub connected: bool,
    pub fail_set: bool,
}

impl BuslightDevice for MockDevice {
    fn set_state(&self, _state: &LightState) -> Result<(), DeviceError> {
        if self.fail_set {
            Err(DeviceError::NotConnected)
        } else {
            Ok(())
        }
    }
    fn is_connected(&self) -> bool {
        self.connected
    }
}

pub fn test_psk() -> Vec<u8> {
    b"test-psk-for-integration-tests!!".to_vec()
}

pub fn make_app(connected: bool) -> axum::Router {
    let shared = Arc::new(Mutex::new(SharedState {
        connected,
        light_state: LightState::default(),
        state_dirty: false,
    }));
    let state = AppState {
        psk: Arc::new(test_psk()),
        shared,
    };
    build_router(state)
}

pub fn auth_headers(body: &[u8]) -> Vec<(&'static str, String)> {
    use rustylight_server::api::auth::{compute_signature, current_unix_time};
    let ts = current_unix_time().to_string();
    let sig = compute_signature(&test_psk(), &ts, body);
    vec![("X-Timestamp", ts), ("X-Signature", sig)]
}
```

> **Note:** For `tests/common/mod.rs` to work, the crate must expose its modules. Add `pub mod api; pub mod device;` etc. to `src/lib.rs` (create it if needed — it can re-export everything from `main.rs`'s modules).

Create `src/lib.rs`:

```rust
pub mod api;
pub mod config;
pub mod device;
pub mod logging;
pub mod tls;
```

And change `src/main.rs` to not re-declare modules (use the lib instead):

```rust
use rustylight_server::{api, config, device, logging, tls};
// ... rest of main
```

- [ ] **Step 2: Write auth tests (`tests/test_auth.rs`)**

```rust
mod common;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn missing_timestamp_header_returns_401() {
    let app = common::make_app(true);
    let req = Request::builder()
        .method("GET")
        .uri("/api/light")
        .header("X-Signature", "deadbeef")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn missing_signature_header_returns_401() {
    let app = common::make_app(true);
    let req = Request::builder()
        .method("GET")
        .uri("/api/light")
        .header("X-Timestamp", "1747394400")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_signature_returns_403() {
    let app = common::make_app(true);
    use rustylight_server::api::auth::current_unix_time;
    let ts = current_unix_time().to_string();
    let req = Request::builder()
        .method("GET")
        .uri("/api/light")
        .header("X-Timestamp", &ts)
        .header("X-Signature", "badsignature")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn stale_timestamp_returns_403_with_server_time_header() {
    let app = common::make_app(true);
    let stale_ts = "1000000000"; // year 2001
    let sig = rustylight_server::api::auth::compute_signature(
        &common::test_psk(), stale_ts, b"",
    );
    let req = Request::builder()
        .method("GET")
        .uri("/api/light")
        .header("X-Timestamp", stale_ts)
        .header("X-Signature", sig)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert!(resp.headers().contains_key("X-Server-Time"));
}

#[tokio::test]
async fn valid_auth_returns_200() {
    let app = common::make_app(true);
    let headers = common::auth_headers(b"");
    let mut builder = Request::builder().method("GET").uri("/api/light");
    for (k, v) in &headers {
        builder = builder.header(*k, v);
    }
    let req = builder.body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
```

- [ ] **Step 3: Write API tests (`tests/test_api.rs`)**

```rust
mod common;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn get_light_returns_connected_false_when_device_absent() {
    let app = common::make_app(false);
    let headers = common::auth_headers(b"");
    let mut builder = Request::builder().method("GET").uri("/api/light");
    for (k, v) in &headers {
        builder = builder.header(*k, v);
    }
    let resp = app.oneshot(builder.body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["connected"], false);
}

#[tokio::test]
async fn post_light_returns_503_when_device_not_connected() {
    let app = common::make_app(false);
    let body = serde_json::json!({"on": true, "r": 255, "g": 0, "b": 0}).to_string();
    let body_bytes = body.as_bytes();
    let headers = common::auth_headers(body_bytes);
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/light")
        .header("Content-Type", "application/json");
    for (k, v) in &headers {
        builder = builder.header(*k, v);
    }
    let resp = app.oneshot(builder.body(Body::from(body)).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn post_light_returns_200_when_device_connected() {
    let app = common::make_app(true);
    let body = serde_json::json!({"on": true, "r": 0, "g": 255, "b": 0}).to_string();
    let body_bytes = body.as_bytes().to_vec();
    let headers = common::auth_headers(&body_bytes);
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/light")
        .header("Content-Type", "application/json");
    for (k, v) in &headers {
        builder = builder.header(*k, v);
    }
    let resp = app.oneshot(builder.body(Body::from(body)).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn post_light_returns_400_for_invalid_blink_ms() {
    let app = common::make_app(true);
    let body = serde_json::json!({
        "on": true, "r": 255, "g": 0, "b": 0,
        "blink": true, "blink_on_ms": 10, "blink_off_ms": 500
    }).to_string();
    let body_bytes = body.as_bytes().to_vec();
    let headers = common::auth_headers(&body_bytes);
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/light")
        .header("Content-Type", "application/json");
    for (k, v) in &headers {
        builder = builder.header(*k, v);
    }
    let resp = app.oneshot(builder.body(Body::from(body)).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_light_returns_400_for_malformed_json() {
    let app = common::make_app(true);
    let body = "not json at all";
    let headers = common::auth_headers(body.as_bytes());
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/light")
        .header("Content-Type", "application/json");
    for (k, v) in &headers {
        builder = builder.header(*k, v);
    }
    let resp = app.oneshot(builder.body(Body::from(body)).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
```

- [ ] **Step 4: Run all integration tests**

```bash
cargo test
```

Expected: all tests pass. Fix any compile errors (typically missing `pub` on module items).

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs tests/
git commit -m "test: integration tests for auth and API endpoints"
```

---

## Task 15: Packaging Files

**Files:**
- Create: `packaging/rustylight.service`
- Create: `packaging/rustylight.logrotate`
- Create: `packaging/postinst`
- Create: `packaging/prerm`
- Create: `packaging/postrm`
- Modify: `Cargo.toml` (add `[package.metadata.deb]` and `[package.metadata.generate-rpm]`)

- [ ] **Step 1: Create systemd unit file (`packaging/rustylight.service`)**

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

- [ ] **Step 2: Create logrotate config (`packaging/rustylight.logrotate`)**

```
/var/log/rustylight/rustylight.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    copytruncate
}
```

- [ ] **Step 3: Create deb maintainer scripts**

`packaging/postinst`:
```bash
#!/bin/sh
set -e

# Create service user
if ! id -u rustylight >/dev/null 2>&1; then
    useradd --system --no-create-home --shell /usr/sbin/nologin rustylight
fi

# Add to plugdev for USB HID access
if getent group plugdev >/dev/null 2>&1; then
    usermod -aG plugdev rustylight
fi

# Create directories
install -d -o rustylight -g rustylight -m 750 /etc/rustylight
install -d -o rustylight -g rustylight -m 750 /var/log/rustylight

# Create default config if not present
if [ ! -f /etc/rustylight/rustylight.conf ]; then
    cat > /etc/rustylight/rustylight.conf <<'EOF'
[server]
port = 8443

[tls]
cert_file = "/etc/rustylight/tls.crt"
key_file  = "/etc/rustylight/tls.key"

[auth]
psk = ""

[logging]
level    = "info"
log_file = "/var/log/rustylight/rustylight.log"
EOF
    chown rustylight:rustylight /etc/rustylight/rustylight.conf
    chmod 600 /etc/rustylight/rustylight.conf
fi

systemctl daemon-reload
systemctl enable --now rustylight.service || true
```

`packaging/prerm`:
```bash
#!/bin/sh
set -e
if [ "$1" = "remove" ] || [ "$1" = "upgrade" ]; then
    systemctl stop rustylight.service || true
fi
```

`packaging/postrm`:
```bash
#!/bin/sh
set -e
if [ "$1" = "remove" ]; then
    systemctl disable rustylight.service || true
fi
if [ "$1" = "purge" ]; then
    rm -rf /etc/rustylight /var/log/rustylight
    if id -u rustylight >/dev/null 2>&1; then
        userdel rustylight || true
    fi
fi
systemctl daemon-reload || true
```

Make scripts executable:
```bash
chmod +x packaging/postinst packaging/prerm packaging/postrm
```

- [ ] **Step 4: Add packaging metadata to `Cargo.toml`**

Append to `Cargo.toml`:

```toml
[package.metadata.deb]
maintainer = "florian@taeger.cc"
copyright = "2026, Florian Taeger"
license-file = ["LICENSE", "0"]
extended-description = "rustylight-server controls a Kuando Busylight via USB and exposes a TLS-encrypted REST API."
depends = "$auto"
section = "utility"
priority = "optional"
assets = [
    ["target/release/rustylight-server", "usr/sbin/", "755"],
    ["packaging/rustylight.service", "lib/systemd/system/rustylight.service", "644"],
    ["packaging/rustylight.logrotate", "etc/logrotate.d/rustylight", "644"],
]
maintainer-scripts = "packaging/"
systemd-units = { enable = true }

[package.metadata.generate-rpm]
assets = [
    { source = "target/release/rustylight-server", dest = "/usr/sbin/rustylight-server", mode = "755" },
    { source = "packaging/rustylight.service", dest = "/lib/systemd/system/rustylight.service", mode = "644" },
    { source = "packaging/rustylight.logrotate", dest = "/etc/logrotate.d/rustylight", mode = "644" },
]

[package.metadata.generate-rpm.requires]
systemd = "*"
```

- [ ] **Step 5: Verify release build**

```bash
cargo build --release
```

Expected: binary at `target/release/rustylight-server`.

- [ ] **Step 6: Commit**

```bash
git add packaging/ Cargo.toml
git commit -m "feat: packaging files, systemd unit, logrotate config, postinst scripts"
```

---

## Task 16: GitHub Actions CI/CD + Dependabot

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`
- Create: `.github/dependabot.yml`

- [ ] **Step 1: Create CI workflow (`.github/workflows/ci.yml`)**

```yaml
name: CI

on:
  push:
    branches: ["**"]
  pull_request:

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo test

  cross-check:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - x86_64-unknown-linux-gnu
          - i686-unknown-linux-gnu
          - aarch64-unknown-linux-gnu
          - armv7-unknown-linux-gnueabihf
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-cross-${{ matrix.target }}-${{ hashFiles('**/Cargo.lock') }}
      - uses: taiki-e/install-action@v2
        with:
          tool: cross
      - run: cross build --target ${{ matrix.target }} --release
```

- [ ] **Step 2: Create release workflow (`.github/workflows/release.yml`)**

```yaml
name: Release

on:
  push:
    tags:
      - "v*.*.*"

env:
  CARGO_TERM_COLOR: always

jobs:
  build-deb:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            deb_arch: amd64
          - target: i686-unknown-linux-gnu
            deb_arch: i386
          - target: aarch64-unknown-linux-gnu
            deb_arch: arm64
          - target: armv7-unknown-linux-gnueabihf
            deb_arch: armhf
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ matrix.target }}-${{ hashFiles('**/Cargo.lock') }}
      - uses: taiki-e/install-action@v2
        with:
          tool: cross
      - run: cross build --target ${{ matrix.target }} --release
      - run: cargo install cargo-deb
      - run: cargo deb --target ${{ matrix.target }} --no-build
      - uses: actions/upload-artifact@v4
        with:
          name: deb-${{ matrix.deb_arch }}
          path: target/${{ matrix.target }}/debian/*.deb

  build-rpm:
    runs-on: ubuntu-latest
    container: almalinux:8
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            rpm_arch: x86_64
          - target: aarch64-unknown-linux-gnu
            rpm_arch: aarch64
    steps:
      - uses: actions/checkout@v4
      - run: dnf install -y curl gcc openssl-devel hidapi-devel
      - uses: dtolnay/rust-toolchain@stable
      - uses: taiki-e/install-action@v2
        with:
          tool: cross
      - run: cross build --target ${{ matrix.target }} --release
      - run: cargo install cargo-generate-rpm
      - run: cargo generate-rpm --target ${{ matrix.target }}
      - uses: actions/upload-artifact@v4
        with:
          name: rpm-${{ matrix.rpm_arch }}
          path: target/${{ matrix.target }}/generate-rpm/*.rpm

  publish:
    needs: [build-deb, build-rpm]
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: artifacts
          merge-multiple: true
      - uses: softprops/action-gh-release@v2
        with:
          files: artifacts/**
```

- [ ] **Step 3: Create Dependabot config (`.github/dependabot.yml`)**

```yaml
version: 2
updates:
  - package-ecosystem: cargo
    directory: "/"
    schedule:
      interval: weekly
    open-pull-requests-limit: 10

  - package-ecosystem: github-actions
    directory: "/"
    schedule:
      interval: weekly
    open-pull-requests-limit: 5
```

- [ ] **Step 4: Commit**

```bash
git add .github/
git commit -m "ci: GitHub Actions workflows for CI and release, Dependabot config"
```

---

## Task 17: README and CONTRIBUTING

**Files:**
- Create: `README.md`
- Create: `CONTRIBUTING.md`

- [ ] **Step 1: Write `README.md`**

```markdown
# rustylight-server

A Rust Linux daemon that controls a [Kuando Busylight](https://www.kuando.com/busylight/) USB device via a TLS-encrypted REST API.

## Supported Hardware

All Kuando Busylight models are supported: UC Omega, Alpha, UC, Lync, UC2. The device is detected automatically by USB VID/PID at startup and on reconnection.

## Installation

### Debian / Ubuntu / Raspberry Pi

Download the `.deb` for your architecture from the [latest release](../../releases/latest):

```bash
sudo dpkg -i rustylight-server_<version>_<arch>.deb
```

Supported architectures: `amd64`, `i386`, `arm64` (Pi 3/4/Zero 2W 64-bit), `armhf` (Pi 3/4/Zero 2W 32-bit).

### RHEL / Rocky Linux / AlmaLinux 8, 9, 10

```bash
sudo rpm -i rustylight-server-<version>-1.<arch>.rpm
```

## Configuration

Config file: `/etc/rustylight/rustylight.conf`

```toml
[server]
port = 8443            # HTTPS port (default: 8443)

[tls]
cert_file = "/etc/rustylight/tls.crt"   # auto-generated if missing
key_file  = "/etc/rustylight/tls.key"

[auth]
psk = ""               # auto-generated on first start

[logging]
level    = "info"      # trace | debug | info | warn | error
log_file = "/var/log/rustylight/rustylight.log"
```

On first start, a random PSK is generated and written to the config. Share the `psk` value with API clients; they need it to compute HMAC signatures.

## Authentication

Every `/api/light` request must include:

```
X-Timestamp: <unix timestamp, seconds UTC>
X-Signature: <lowercase hex HMAC-SHA256(base64url-decoded-psk, timestamp_string + request_body)>
```

For GET requests, `request_body` is an empty string.

### Example with curl

```bash
PSK_RAW=$(cat /etc/rustylight/rustylight.conf | grep ^psk | awk -F'"' '{print $2}')
TS=$(date +%s)
SIG=$(python3 -c "
import hmac, hashlib, base64, sys
psk = base64.urlsafe_b64decode('${PSK_RAW}' + '==')
body = b''
sig = hmac.new(psk, '${TS}'.encode() + body, hashlib.sha256).hexdigest()
print(sig)")

curl -sk \
  -H "X-Timestamp: $TS" \
  -H "X-Signature: $SIG" \
  https://localhost:8443/api/light
```

## API

Full API documentation is available at `https://<host>:8443/api` (Swagger UI).

### `GET /api/light`

Returns current busylight state:

```json
{"connected": true, "on": true, "r": 255, "g": 0, "b": 0, "blink": false}
```

### `POST /api/light`

Set steady color:
```json
{"on": true, "r": 0, "g": 255, "b": 0}
```

Blink (color to off):
```json
{"on": true, "r": 255, "g": 0, "b": 0, "blink": true, "blink_on_ms": 500, "blink_off_ms": 500}
```

Blink (two colors):
```json
{"on": true, "r": 255, "g": 0, "b": 0, "blink": true, "blink_on_ms": 500, "blink_off_ms": 500, "r2": 0, "g2": 0, "b2": 255}
```

Turn off:
```json
{"on": false}
```

## Service Management

```bash
sudo systemctl start rustylight
sudo systemctl stop rustylight
sudo systemctl status rustylight
sudo journalctl -u rustylight -f
```

## Logs

`/var/log/rustylight/rustylight.log` — rotated daily, compressed after 2 days, deleted after 30 days.
```

- [ ] **Step 2: Write `CONTRIBUTING.md`**

```markdown
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
```

- [ ] **Step 3: Commit**

```bash
git add README.md CONTRIBUTING.md
git commit -m "docs: README with API usage and CONTRIBUTING with dev setup guide"
```

---

## Self-Review

**Spec coverage check:**
- [x] All Busylight models via VID/PID — Task 3
- [x] HTTP REST API (GET + POST /api/light) — Tasks 9, 11
- [x] HTTP response codes (400/401/403/500/503) — Tasks 10, 11, integration tests
- [x] Swagger at /api — Task 12
- [x] HTTPS + self-signed ECC cert, auto-generation — Task 7
- [x] Configurable port — Task 5, main.rs
- [x] PSK auto-generation, stored in config — Task 5
- [x] HMAC-SHA256(psk, timestamp + body), X-Timestamp + X-Signature — Task 10
- [x] ±30s timestamp window, X-Server-Time response header — Task 10
- [x] Config at /etc/rustylight/rustylight.conf, TOML — Task 5
- [x] Logging to file, configurable level, tracing — Task 6
- [x] Log rotation daily, compress after 2 days, delete after 30 — Task 15 (logrotate)
- [x] Log: device connect/disconnect (info), API access with client IP (debug) — Tasks 8, 11
- [x] Systemd service — Task 15
- [x] .deb packages (amd64, i386, arm64, armhf) — Tasks 15, 16
- [x] .rpm packages (x86_64, aarch64) for RHEL 8/9/10 — Tasks 15, 16
- [x] GitHub Actions CI + release — Task 16
- [x] Dependabot — Task 16
- [x] Unit + integration tests — Tasks 2–11, 14
- [x] Developer documentation — Tasks 17, inline rustdoc
- [x] Blink: color-to-off and color-to-color — Tasks 2, 4, 11
- [x] Hot-plug reconnection — Task 8
- [x] Keepalive — Tasks 4 (keepalive byte in report), 8 (2s polling loop)
- [x] Client IP in API logs — Needs explicit check: `tower-http` trace layer logs requests but not client IP by default. Add `ConnectInfo<SocketAddr>` extractor to handlers to log client IP at debug level.

**Placeholder scan:** No TBDs or "implement later" found.

**Type consistency check:**
- `LightState` — defined in Task 2, used in Tasks 4, 5, 8, 9, 10, 11, 14 ✓
- `SharedState` — defined in Task 8 (device/mod.rs addition), used in Tasks 9, 11, 14 ✓
- `AppState` — defined in Task 9, used in Tasks 10, 11, 14 ✓
- `BuslightDevice` trait — defined in Task 2, mock in Task 14 ✓
- `build_report` — defined in Task 4, called in Task 8 ✓
- `AuthGuard` — defined in Task 10, used in Task 11 ✓
- `compute_signature` / `verify_signature` / `current_unix_time` — defined in Task 10, used in Task 14 ✓

**Gap fix — client IP logging:** In `src/api/handlers.rs`, add `ConnectInfo<SocketAddr>` to both handlers:

```rust
use axum::extract::ConnectInfo;
use std::net::SocketAddr;

pub async fn get_light(
    _auth: AuthGuard,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::debug!("GET /api/light from {addr}");
    // ... rest unchanged
}

pub async fn post_light(
    AuthGuard(body_bytes): AuthGuard,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // ... log after auth passes
    tracing::debug!("POST /api/light from {addr}");
    // ... rest unchanged
}
```

This fix should be applied as part of Task 11 implementation.
```
