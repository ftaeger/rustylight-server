# Healthcheck Endpoint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `GET /api/public/healthcheck` (no auth, 200/503) that reports busylight connectivity and log writability; rename `GET /api/version` to `GET /api/public/version`; bump version to 0.1.0.

**Architecture:** `AppState` gains a `log_file: Arc<String>` field so the health handler can attempt an append-open against the configured log path. All public (no-auth) endpoints live under the `/api/public/` prefix. Auth is enforced per-handler via the existing `AuthGuard` extractor, so no router-level middleware change is needed.

**Tech Stack:** Rust, axum 0.8, utoipa 5, tempfile 3 (dev)

---

## File Map

| File | Change |
|------|--------|
| `Cargo.toml` | version `0.0.9` → `0.1.0` |
| `src/api/mod.rs` | add `log_file: Arc<String>` to `AppState`; update routes |
| `src/api/handlers.rs` | update `utoipa` path on `get_version`; add `HealthResponse` + `get_healthcheck` |
| `src/api/openapi.rs` | register `get_healthcheck` and `HealthResponse` |
| `src/main.rs` | pass `cfg.logging.log_file` into `AppState` |
| `tests/common/mod.rs` | add `log_file` to `AppState` construction; add `make_app_with_log` helper |
| `tests/test_api.rs` | update version test URL; add four healthcheck tests |
| `docs/API.md` | document new endpoint; update version endpoint path |
| `CLAUDE.md` | update endpoint table |

---

## Task 1: Bump version to 0.1.0

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Update version**

In `Cargo.toml` change:
```toml
version = "0.0.9"
```
to:
```toml
version = "0.1.0"
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build 2>&1 | tail -3
```
Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: bump version to 0.1.0"
```

---

## Task 2: Add log_file to AppState and update test harness

`AppState` needs a `log_file` field so the health handler can reach the configured log path. All existing tests construct `AppState` directly in `tests/common/mod.rs`, so that file must be updated in the same step to keep the build green.

**Files:**
- Modify: `src/api/mod.rs`
- Modify: `src/main.rs`
- Modify: `tests/common/mod.rs`

- [ ] **Step 1: Add log_file to AppState**

In `src/api/mod.rs`, replace the `AppState` struct:

```rust
#[derive(Clone)]
pub struct AppState {
    pub psk: Arc<String>,
    pub shared: Arc<Mutex<SharedState>>,
    pub log_file: Arc<String>,
}
```

- [ ] **Step 2: Pass log_file from config in main.rs**

In `src/main.rs`, replace the `AppState` construction block:

```rust
    let state = api::AppState {
        psk: Arc::new(cfg.auth.psk.clone()),
        shared: Arc::clone(&shared),
        log_file: Arc::new(cfg.logging.log_file.clone()),
    };
```

- [ ] **Step 3: Update the test harness**

Replace the entire contents of `tests/common/mod.rs`:

```rust
use rustylight_server::api::{build_router, AppState};
use rustylight_server::device::{LightState, SharedState};
use std::sync::{Arc, Mutex};

pub fn test_psk() -> String {
    "test-psk-for-integration-tests!!".to_string()
}

pub fn make_app(connected: bool) -> axum::Router {
    let log_file = std::env::temp_dir()
        .join("rustylight-test.log")
        .to_str()
        .unwrap()
        .to_string();
    make_app_with_log(connected, log_file)
}

pub fn make_app_with_log(connected: bool, log_file: String) -> axum::Router {
    let shared = Arc::new(Mutex::new(SharedState {
        connected,
        light_state: LightState::default(),
        state_dirty: false,
    }));
    let state = AppState {
        psk: Arc::new(test_psk()),
        shared,
        log_file: Arc::new(log_file),
    };
    build_router(state)
}

#[allow(dead_code)]
pub fn auth_headers() -> Vec<(&'static str, String)> {
    vec![("X-Api-Key", test_psk())]
}
```

- [ ] **Step 4: Verify all existing tests still pass**

```bash
cargo test 2>&1 | tail -20
```
Expected: all tests pass, no compilation errors.

- [ ] **Step 5: Commit**

```bash
git add src/api/mod.rs src/main.rs tests/common/mod.rs
git commit -m "refactor: add log_file to AppState"
```

---

## Task 3: Rename /api/version → /api/public/version

**Files:**
- Modify: `src/api/handlers.rs` (utoipa path annotation)
- Modify: `src/api/mod.rs` (route string)
- Modify: `tests/test_api.rs` (update existing test URL)

- [ ] **Step 1: Write a failing test for the new URL**

Add to `tests/test_api.rs` (below the existing tests):

```rust
#[tokio::test]
async fn get_public_version_returns_200_without_auth() {
    let app = common::make_app(false);
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/public/version")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    assert!(json["time"].as_str().is_some());
}
```

- [ ] **Step 2: Run the new test and confirm it fails**

```bash
cargo test get_public_version 2>&1 | tail -10
```
Expected: FAIL — `assertion failed` because `/api/public/version` returns 404.

- [ ] **Step 3: Update the utoipa path annotation on get_version**

In `src/api/handlers.rs`, change `path = "/api/version"` to `path = "/api/public/version"` in the `#[utoipa::path(...)]` block above `get_version`:

```rust
#[utoipa::path(
    get,
    path = "/api/public/version",
    responses(
        (status = 200, description = "Server version and current UTC time", body = VersionResponse),
    )
)]
pub async fn get_version() -> impl IntoResponse {
```

- [ ] **Step 4: Update the route in mod.rs**

In `src/api/mod.rs`, replace `"/version"` with `"/public/version"` in `build_router`:

```rust
pub fn build_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/light", get(handlers::get_light))
        .route("/light", post(handlers::post_light))
        .route("/public/version", get(handlers::get_version))
        .with_state(state);

    let swagger_routes = openapi::swagger_router();

    Router::new().nest("/api", api_routes).merge(swagger_routes)
}
```

- [ ] **Step 5: Update the old version test to use the new URL**

In `tests/test_api.rs`, update the existing `get_version_returns_200_without_auth` test — change `uri("/api/version")` to `uri("/api/public/version")` and rename it to avoid confusion:

```rust
#[tokio::test]
async fn get_version_returns_200_without_auth() {
    let app = common::make_app(false);
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/public/version")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    assert!(json["time"].as_str().is_some());
}
```

Then delete the `get_public_version_returns_200_without_auth` test you added in Step 1 (it's now a duplicate).

- [ ] **Step 6: Run all tests and confirm they pass**

```bash
cargo test 2>&1 | tail -20
```
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/api/handlers.rs src/api/mod.rs tests/test_api.rs
git commit -m "feat: rename /api/version to /api/public/version"
```

---

## Task 4: Add GET /api/public/healthcheck

**Files:**
- Modify: `src/api/handlers.rs` (new struct + handler)
- Modify: `src/api/mod.rs` (new route)
- Modify: `src/api/openapi.rs` (register handler + schema)
- Modify: `tests/test_api.rs` (four new tests)

- [ ] **Step 1: Write four failing tests**

Add to the bottom of `tests/test_api.rs`:

```rust
#[tokio::test]
async fn healthcheck_returns_200_when_healthy() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let log_path = log.path().to_str().unwrap().to_string();
    let app = common::make_app_with_log(true, log_path);
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/public/healthcheck")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "ok");
    assert_eq!(json["busylight_connected"], true);
    assert_eq!(json["log_writable"], true);
}

#[tokio::test]
async fn healthcheck_requires_no_auth() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let log_path = log.path().to_str().unwrap().to_string();
    let app = common::make_app_with_log(true, log_path);
    // No X-Api-Key header — must still return 200
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/public/healthcheck")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn healthcheck_returns_503_when_device_disconnected() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let log_path = log.path().to_str().unwrap().to_string();
    let app = common::make_app_with_log(false, log_path);
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/public/healthcheck")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "degraded");
    assert_eq!(json["busylight_connected"], false);
    assert_eq!(json["log_writable"], true);
}

#[tokio::test]
async fn healthcheck_returns_503_when_log_not_writable() {
    let app = common::make_app_with_log(
        true,
        "/nonexistent/directory/rustylight.log".to_string(),
    );
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/public/healthcheck")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "degraded");
    assert_eq!(json["busylight_connected"], true);
    assert_eq!(json["log_writable"], false);
}
```

- [ ] **Step 2: Confirm the tests fail**

```bash
cargo test healthcheck 2>&1 | tail -15
```
Expected: compile error or 404 — handler doesn't exist yet.

- [ ] **Step 3: Add HealthResponse struct and get_healthcheck handler**

In `src/api/handlers.rs`, add after the `get_version` function:

```rust
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
    pub busylight_connected: bool,
    pub log_writable: bool,
}

#[utoipa::path(
    get,
    path = "/api/public/healthcheck",
    responses(
        (status = 200, description = "All checks pass", body = HealthResponse),
        (status = 503, description = "One or more checks failed", body = HealthResponse),
    )
)]
pub async fn get_healthcheck(State(state): State<AppState>) -> impl IntoResponse {
    let busylight_connected = state.shared.lock().unwrap().connected;
    let log_writable = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(state.log_file.as_str())
        .is_ok();

    let healthy = busylight_connected && log_writable;
    let response = HealthResponse {
        status: if healthy { "ok" } else { "degraded" },
        busylight_connected,
        log_writable,
    };
    let status_code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status_code, Json(response)).into_response()
}
```

- [ ] **Step 4: Add the route in mod.rs**

In `src/api/mod.rs`, add the healthcheck route to `build_router`:

```rust
pub fn build_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/light", get(handlers::get_light))
        .route("/light", post(handlers::post_light))
        .route("/public/version", get(handlers::get_version))
        .route("/public/healthcheck", get(handlers::get_healthcheck))
        .with_state(state);

    let swagger_routes = openapi::swagger_router();

    Router::new().nest("/api", api_routes).merge(swagger_routes)
}
```

- [ ] **Step 5: Register in OpenAPI**

Replace the entire contents of `src/api/openapi.rs`:

```rust
use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::handlers::{self, HealthResponse, VersionResponse};
use crate::device::LightState;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::get_light,
        handlers::post_light,
        handlers::get_version,
        handlers::get_healthcheck,
    ),
    components(schemas(LightState, VersionResponse, HealthResponse)),
    info(
        title = "rustylight-server API",
        version = "0.1.0",
        description = "REST API for controlling a Kuando Busylight USB device.\n\n\
            ## Authentication\n\
            Every `/api/light` request requires an `X-Api-Key` header containing \
            the PSK from `/etc/rustylight/rustylight.conf`."
    )
)]
pub struct ApiDoc;

pub fn swagger_router() -> Router {
    Router::new().merge(SwaggerUi::new("/api").url("/api/openapi.json", ApiDoc::openapi()))
}
```

- [ ] **Step 6: Run all tests and confirm they pass**

```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test 2>&1 | tail -25
```
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/api/handlers.rs src/api/mod.rs src/api/openapi.rs tests/test_api.rs
git commit -m "feat: add GET /api/public/healthcheck"
```

---

## Task 5: Update documentation

**Files:**
- Modify: `docs/API.md`
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update the endpoint table in docs/API.md**

In `docs/API.md`, find the endpoints table (under `## Endpoints`) and add the new row and update the version row:

Change the existing `GET /api/version` row to `GET /api/public/version`, and add a new row for `GET /api/public/healthcheck`:

```markdown
| GET | `/api/light` | X-Api-Key | Current light state + `connected` field |
| POST | `/api/light` | X-Api-Key | Set new light state |
| GET | `/api/public/version` | none | Server version + current UTC time (RFC 3339) |
| GET | `/api/public/healthcheck` | none | Service health status |
| GET | `/api` | none | Swagger UI |
| GET | `/api/openapi.json` | none | OpenAPI spec |
```

- [ ] **Step 2: Add the healthcheck endpoint section to docs/API.md**

Find the `### GET /api/version` section and rename it to `### GET /api/public/version`.

Then add a new section for the healthcheck endpoint after it:

```markdown
### GET /api/public/healthcheck

Returns service health status. No authentication required.

**Request headers**: none

**Request body**: none

**Success response — 200 OK**
```json
{
  "status": "ok",
  "busylight_connected": true,
  "log_writable": true
}
```

**Degraded response — 503 Service Unavailable**
```json
{
  "status": "degraded",
  "busylight_connected": false,
  "log_writable": true
}
```

| Field | Type | Description |
|-------|------|-------------|
| `status` | string | `"ok"` when all checks pass, `"degraded"` when any check fails |
| `busylight_connected` | boolean | Whether a Busylight USB device is currently detected |
| `log_writable` | boolean | Whether the configured log file can be written |
```

- [ ] **Step 3: Update the endpoint table in CLAUDE.md**

In `CLAUDE.md`, find the API endpoints table and update it:

```markdown
| GET | `/api/light` | X-Api-Key | Current light state + `connected` field |
| POST | `/api/light` | X-Api-Key | Set new light state |
| GET | `/api/public/version` | none | Server version + current UTC time (RFC 3339) |
| GET | `/api/public/healthcheck` | none | Service health: busylight connected + log writable |
| GET | `/api` | none | Swagger UI |
| GET | `/api/openapi.json` | none | OpenAPI spec |
```

- [ ] **Step 4: Run tests one final time**

```bash
cargo test 2>&1 | tail -10
```
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add docs/API.md CLAUDE.md
git commit -m "docs: add healthcheck endpoint, rename /version to /public/version"
```
