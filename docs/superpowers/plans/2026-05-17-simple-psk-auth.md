# Simple PSK Authentication Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace HMAC-SHA256 + timestamp authentication with a plain `X-Api-Key` header check.

**Architecture:** `AuthGuard` becomes a `FromRequestParts` extractor that reads `X-Api-Key` and compares it to the PSK string stored in `AppState`. The PSK is stored as `Arc<String>` and generated as a 64-char hex string. All crypto crates are removed.

**Tech Stack:** Rust, Axum 0.7, `rand` (PSK generation only)

---

## File Map

| File | Change |
|------|--------|
| `src/api/mod.rs` | `psk: Arc<Vec<u8>>` → `Arc<String>` |
| `src/config.rs` | Remove `decode_psk`, simplify `generate_psk`, remove `base64` import |
| `src/main.rs` | Remove `decode_psk` call, pass `cfg.auth.psk` directly |
| `src/api/auth.rs` | Full rewrite — header check only, no HMAC |
| `src/api/handlers.rs` | `post_light` body via `Bytes`, update utoipa annotations |
| `tests/common/mod.rs` | `test_psk() -> String`, `auth_headers()` takes no body arg |
| `tests/test_auth.rs` | Replace timestamp/signature tests with X-Api-Key tests |
| `tests/test_api.rs` | Update `auth_headers` call sites (remove body arg) |
| `Cargo.toml` | Remove `hmac`, `sha2`, `hex`, `base64` |
| `examples/set-color.sh` | Remove signing logic, pass `X-Api-Key` header |
| `examples/get-state.sh` | Remove signing logic, pass `X-Api-Key` header |
| `docs/API.md` | Rewrite auth section and examples |

---

### Task 1: Update AppState, config, and main

**Files:**
- Modify: `src/api/mod.rs:14`
- Modify: `src/config.rs`
- Modify: `src/main.rs:24-34`

- [ ] **Step 1: Change `AppState.psk` to `Arc<String>`**

In `src/api/mod.rs`, replace line 14:

```rust
pub struct AppState {
    pub psk: Arc<String>,
    pub shared: Arc<Mutex<SharedState>>,
}
```

- [ ] **Step 2: Simplify `generate_psk` and remove `decode_psk` from `src/config.rs`**

Remove the `use base64::...` import at the top. Replace `generate_psk` and remove `decode_psk`:

```rust
pub fn generate_psk() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
```

Delete the entire `decode_psk` function (lines 87–91 in current file).

Also update the unit test `generates_non_empty_psk` — remove the `URL_SAFE.decode` assertion:

```rust
#[test]
fn generates_non_empty_psk() {
    let psk = generate_psk();
    assert!(!psk.is_empty());
    assert_eq!(psk.len(), 64);
}
```

- [ ] **Step 3: Update `src/main.rs` to use the PSK string directly**

Remove line 24 (`let psk_bytes = config::decode_psk...`).
Update the AppState construction (currently lines 32–35):

```rust
let state = api::AppState {
    psk: Arc::new(cfg.auth.psk.clone()),
    shared: Arc::clone(&shared),
};
```

- [ ] **Step 4: Verify it compiles (tests will fail — that's expected)**

```bash
cargo build 2>&1 | head -40
```

Expected: compile errors only in `auth.rs` and test files (not in `mod.rs`, `config.rs`, or `main.rs`).

- [ ] **Step 5: Commit**

```bash
git add src/api/mod.rs src/config.rs src/main.rs
git commit -m "refactor: change PSK type to Arc<String>, simplify generation"
```

---

### Task 2: Rewrite auth module

**Files:**
- Modify: `src/api/auth.rs`

- [ ] **Step 1: Replace the entire contents of `src/api/auth.rs`**

```rust
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};

pub struct AuthGuard;

pub enum AuthError {
    MissingKey,
    InvalidKey,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::MissingKey => (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": "missing header: X-Api-Key"})),
            )
                .into_response(),
            AuthError::InvalidKey => (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": "invalid API key"})),
            )
                .into_response(),
        }
    }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthGuard
where
    S: Send + Sync,
    crate::api::AppState: axum::extract::FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        use axum::extract::FromRef;
        let app_state = crate::api::AppState::from_ref(state);

        let key = parts
            .headers
            .get("X-Api-Key")
            .ok_or(AuthError::MissingKey)?
            .to_str()
            .map_err(|_| AuthError::InvalidKey)?;

        if key != app_state.psk.as_str() {
            return Err(AuthError::InvalidKey);
        }

        Ok(AuthGuard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_error_missing_key_is_401() {
        let resp = AuthError::MissingKey.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn auth_error_invalid_key_is_401() {
        let resp = AuthError::InvalidKey.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
```

- [ ] **Step 2: Run the unit tests in auth.rs**

```bash
cargo test -p rustylight-server auth::tests 2>&1
```

Expected: 2 tests pass. Integration tests may still fail — that's fine.

- [ ] **Step 3: Commit**

```bash
git add src/api/auth.rs
git commit -m "refactor: replace HMAC auth with X-Api-Key header check"
```

---

### Task 3: Update handlers

**Files:**
- Modify: `src/api/handlers.rs`

- [ ] **Step 1: Update `post_light` signature and body parsing**

`post_light` currently destructures `AuthGuard(body_bytes)` to get the body. Since `AuthGuard` is now a unit struct and implements `FromRequestParts`, replace it with a separate `Bytes` extractor. Also add `use axum::body::Bytes;` to the imports.

Replace the imports block at the top of `src/api/handlers.rs`:

```rust
use axum::{
    body::Bytes,
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::net::SocketAddr;

use crate::{
    api::{auth::AuthGuard, AppState},
    device::LightState,
};
```

Replace the `post_light` function signature and body-reading line:

```rust
pub async fn post_light(
    addr: Option<ConnectInfo<SocketAddr>>,
    State(state): State<AppState>,
    _auth: AuthGuard,
    body: Bytes,
) -> impl IntoResponse {
    let light_state: LightState = match serde_json::from_slice(&body) {
```

(Everything after that line stays identical.)

- [ ] **Step 2: Update utoipa annotations on both handlers**

Replace the `params(...)` block on `get_light`:

```rust
#[utoipa::path(
    get,
    path = "/api/light",
    responses(
        (status = 200, description = "Current busylight state"),
        (status = 401, description = "Missing or invalid API key"),
        (status = 503, description = "Busylight not connected"),
    ),
    params(
        ("X-Api-Key" = String, Header, description = "Pre-shared API key from config"),
    )
)]
```

Replace the `params(...)` block on `post_light`:

```rust
#[utoipa::path(
    post,
    path = "/api/light",
    request_body = LightState,
    responses(
        (status = 200, description = "State applied"),
        (status = 400, description = "Invalid request body"),
        (status = 401, description = "Missing or invalid API key"),
        (status = 503, description = "Busylight not connected"),
    ),
    params(
        ("X-Api-Key" = String, Header, description = "Pre-shared API key from config"),
    )
)]
```

- [ ] **Step 3: Verify handlers compile**

```bash
cargo build 2>&1 | head -40
```

Expected: no errors in `src/`. Only test files may still complain.

- [ ] **Step 4: Commit**

```bash
git add src/api/handlers.rs
git commit -m "refactor: update handlers for unit-struct AuthGuard and X-Api-Key annotation"
```

---

### Task 4: Update integration tests

**Files:**
- Modify: `tests/common/mod.rs`
- Modify: `tests/test_auth.rs`
- Modify: `tests/test_api.rs`

- [ ] **Step 1: Rewrite `tests/common/mod.rs`**

```rust
use rustylight_server::api::{build_router, AppState};
use rustylight_server::device::{LightState, SharedState};
use std::sync::{Arc, Mutex};

pub fn test_psk() -> String {
    "test-psk-for-integration-tests!!".to_string()
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

pub fn auth_headers() -> Vec<(&'static str, String)> {
    vec![("X-Api-Key", test_psk())]
}
```

- [ ] **Step 2: Rewrite `tests/test_auth.rs`**

```rust
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn missing_api_key_returns_401() {
    let app = common::make_app(true);
    let req = Request::builder()
        .method("GET")
        .uri("/api/light")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn wrong_api_key_returns_401() {
    let app = common::make_app(true);
    let req = Request::builder()
        .method("GET")
        .uri("/api/light")
        .header("X-Api-Key", "wrong-key")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn valid_api_key_on_get_returns_200() {
    let app = common::make_app(true);
    let req = Request::builder()
        .method("GET")
        .uri("/api/light")
        .header("X-Api-Key", common::test_psk())
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
```

- [ ] **Step 3: Update `tests/test_api.rs` call sites**

`auth_headers` no longer takes a body argument. Update every call from `common::auth_headers(&body_bytes)` or `common::auth_headers(b"")` to `common::auth_headers()`.

There are 5 occurrences — in `get_light_returns_connected_false_when_device_absent`, `post_light_returns_503_when_device_not_connected`, `post_light_returns_200_when_device_connected`, `post_light_returns_400_for_invalid_blink_ms`, and `post_light_returns_400_for_malformed_json`.

Also remove the now-unused `let body_bytes = body.as_bytes().to_vec();` lines in each POST test.

Updated POST test helper pattern (example):

```rust
#[tokio::test]
async fn post_light_returns_503_when_device_not_connected() {
    let app = common::make_app(false);
    let body = serde_json::json!({"on": true, "r": 255, "g": 0, "b": 0}).to_string();
    let headers = common::auth_headers();
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/light")
        .header("Content-Type", "application/json");
    for (k, v) in &headers {
        builder = builder.header(*k, v);
    }
    let resp = app
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}
```

Apply the same pattern to the other four POST tests.

- [ ] **Step 4: Run the full test suite**

```bash
cargo test 2>&1
```

Expected: all tests pass. Zero compilation errors.

- [ ] **Step 5: Commit**

```bash
git add tests/common/mod.rs tests/test_auth.rs tests/test_api.rs
git commit -m "test: update integration tests for X-Api-Key auth"
```

---

### Task 5: Remove unused dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Remove `hmac`, `sha2`, `hex`, `base64` from `[dependencies]`**

Delete these four lines from `Cargo.toml`:

```toml
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"
base64 = { version = "0.22", features = [] }
```

- [ ] **Step 2: Verify the build still passes**

```bash
cargo test 2>&1
```

Expected: all tests pass, no unused dependency warnings.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: remove hmac, sha2, hex, base64 dependencies"
```

---

### Task 6: Update example scripts

**Files:**
- Modify: `examples/set-color.sh`
- Modify: `examples/get-state.sh`

- [ ] **Step 1: Rewrite `examples/set-color.sh`**

Replace the entire file:

```bash
#!/usr/bin/env bash
# Send a color command to rustylight-server.
#
# Usage:
#   ./set-color.sh [OPTIONS] <r> <g> <b>
#   ./set-color.sh [OPTIONS] off
#
# Options:
#   -h HOST     Server hostname or IP  (default: localhost)
#   -p PORT     Server port            (default: 8443)
#   -k PSK      API key                (default: $RUSTYLIGHT_PSK env var)
#   --blink     Enable blinking
#   --on-ms N   Blink on duration  in ms, 50–10000 (default: 500)
#   --off-ms N  Blink off duration in ms, 50–10000 (default: 500)
#   --r2 N      Secondary blink color red   (0–255)
#   --g2 N      Secondary blink color green (0–255)
#   --b2 N      Secondary blink color blue  (0–255)
#
# Examples:
#   RUSTYLIGHT_PSK=<psk> ./set-color.sh 255 0 0          # solid red
#   RUSTYLIGHT_PSK=<psk> ./set-color.sh off              # turn off
#   RUSTYLIGHT_PSK=<psk> ./set-color.sh --blink 255 0 0  # blinking red
#   RUSTYLIGHT_PSK=<psk> ./set-color.sh --blink --r2 0 --g2 0 --b2 255 255 0 0  # red/blue blink

set -euo pipefail

HOST="localhost"
PORT="8443"
PSK="${RUSTYLIGHT_PSK:-}"
BLINK="false"
ON_MS=""
OFF_MS=""
R2=""
G2=""
B2=""

usage() {
  sed -n '3,28p' "$0" | sed 's/^# \?//'
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h) HOST="$2"; shift 2 ;;
    -p) PORT="$2"; shift 2 ;;
    -k) PSK="$2"; shift 2 ;;
    --blink) BLINK="true"; shift ;;
    --on-ms) ON_MS="$2"; shift 2 ;;
    --off-ms) OFF_MS="$2"; shift 2 ;;
    --r2) R2="$2"; shift 2 ;;
    --g2) G2="$2"; shift 2 ;;
    --b2) B2="$2"; shift 2 ;;
    --) shift; break ;;
    -*) echo "Unknown option: $1" >&2; usage ;;
    *) break ;;
  esac
done

if [[ -z "$PSK" ]]; then
  echo "Error: PSK not set. Use -k <psk> or set RUSTYLIGHT_PSK." >&2
  exit 1
fi

if [[ "${1:-}" == "off" ]]; then
  BODY='{"on":false,"r":0,"g":0,"b":0}'
else
  [[ $# -lt 3 ]] && { echo "Error: expected <r> <g> <b> or 'off'" >&2; usage; }
  R="$1"; G="$2"; B="$3"

  BODY="{\"on\":true,\"r\":${R},\"g\":${G},\"b\":${B},\"blink\":${BLINK}"
  [[ -n "$ON_MS"  ]] && BODY="${BODY},\"blink_on_ms\":${ON_MS}"
  [[ -n "$OFF_MS" ]] && BODY="${BODY},\"blink_off_ms\":${OFF_MS}"
  [[ -n "$R2"     ]] && BODY="${BODY},\"r2\":${R2}"
  [[ -n "$G2"     ]] && BODY="${BODY},\"g2\":${G2}"
  [[ -n "$B2"     ]] && BODY="${BODY},\"b2\":${B2}"
  BODY="${BODY}}"
fi

curl --silent --show-error \
  --insecure \
  --request POST \
  --url "https://${HOST}:${PORT}/api/light" \
  --header "Content-Type: application/json" \
  --header "X-Api-Key: ${PSK}" \
  --data "$BODY"

echo
```

- [ ] **Step 2: Rewrite `examples/get-state.sh`**

Replace the entire file:

```bash
#!/usr/bin/env bash
# Read the current light state from rustylight-server and print it.
#
# Usage:
#   ./get-state.sh [OPTIONS]
#
# Options:
#   -h HOST   Server hostname or IP  (default: localhost)
#   -p PORT   Server port            (default: 8443)
#   -k PSK    API key                (default: $RUSTYLIGHT_PSK env var)
#
# Examples:
#   RUSTYLIGHT_PSK=<psk> ./get-state.sh
#   RUSTYLIGHT_PSK=<psk> ./get-state.sh -h 192.168.1.10 -p 8443

set -euo pipefail

HOST="localhost"
PORT="8443"
PSK="${RUSTYLIGHT_PSK:-}"

usage() {
  sed -n '3,16p' "$0" | sed 's/^# \?//'
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h) HOST="$2"; shift 2 ;;
    -p) PORT="$2"; shift 2 ;;
    -k) PSK="$2"; shift 2 ;;
    --) shift; break ;;
    -*) echo "Unknown option: $1" >&2; usage ;;
    *) break ;;
  esac
done

if [[ -z "$PSK" ]]; then
  echo "Error: PSK not set. Use -k <psk> or set RUSTYLIGHT_PSK." >&2
  exit 1
fi

RESPONSE="$(curl --silent --show-error \
  --insecure \
  --request GET \
  --url "https://${HOST}:${PORT}/api/light" \
  --header "X-Api-Key: ${PSK}")"

if command -v jq &>/dev/null; then
  printf '%s\n' "$RESPONSE" | jq .
else
  printf '%s\n' "$RESPONSE"
fi
```

- [ ] **Step 3: Commit**

```bash
git add examples/set-color.sh examples/get-state.sh
git commit -m "feat: simplify example scripts to use X-Api-Key header"
```

---

### Task 7: Update API documentation

**Files:**
- Modify: `docs/API.md`
- Modify: `src/api/openapi.rs`

- [ ] **Step 1: Update the OpenAPI description in `src/api/openapi.rs`**

Replace the `info(...)` block:

```rust
info(
    title = "rustylight-server API",
    version = "0.1.0",
    description = "REST API for controlling a Kuando Busylight USB device.\n\n\
        ## Authentication\n\
        Every `/api/light` request requires an `X-Api-Key` header containing \
        the PSK from `/etc/rustylight/rustylight.conf`."
)
```

- [ ] **Step 2: Rewrite the Authentication section in `docs/API.md`**

Replace the entire **Authentication** section (from `## Authentication` through the Rust example block) with:

```markdown
## Authentication

Every `/api/light` request requires one header:

| Header | Value |
|--------|-------|
| `X-Api-Key` | The exact value of `auth.psk` from the config file |

### Example (curl)

```bash
curl --insecure \
  -H "X-Api-Key: <your-psk>" \
  https://localhost:8443/api/light
```

### Example (Python)

```python
import requests

PSK = "your_psk_here"
resp = requests.get(
    "https://localhost:8443/api/light",
    headers={"X-Api-Key": PSK},
    verify=False,
)
```

### Example (JavaScript / Node.js)

```js
const resp = await fetch("https://localhost:8443/api/light", {
  headers: { "X-Api-Key": process.env.PSK },
});
```

### Example (Rust)

```rust
let resp = client
    .get("https://localhost:8443/api/light")
    .header("X-Api-Key", &psk)
    .send()
    .await?;
```
```

- [ ] **Step 3: Update the Error Reference table in `docs/API.md`**

Remove the 403 rows for timestamp/signature. The table becomes:

```markdown
| HTTP Status | Error message | Cause |
|-------------|---------------|-------|
| 400 | `invalid JSON: <detail>` | POST body is not valid JSON or wrong type |
| 400 | `blink_on_ms must be 50–10000, got <n>` | Out of range while `blink` is `true` |
| 400 | `blink_off_ms must be 50–10000, got <n>` | Out of range while `blink` is `true` |
| 401 | `missing header: X-Api-Key` | `X-Api-Key` header absent |
| 401 | `invalid API key` | `X-Api-Key` value does not match PSK |
| 503 | `Busylight not connected` | No compatible USB device detected |
```

- [ ] **Step 4: Update the Quick Start Checklist in `docs/API.md`**

Replace steps 3 and 4 of the checklist:

```markdown
3. For every request, send `X-Api-Key: <psk>` as a header.
4. If you receive a 503, the USB device is not plugged in — retry after reconnecting.
```

- [ ] **Step 5: Run full test suite one final time**

```bash
cargo test 2>&1
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/api/openapi.rs docs/API.md
git commit -m "docs: update API.md and OpenAPI description for X-Api-Key auth"
```

---

### Final: Push

```bash
git push
```
