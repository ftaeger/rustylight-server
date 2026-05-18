# /api/version Endpoint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a public `GET /api/version` endpoint that returns the running server version and current UTC time in RFC 3339 format, with no authentication required.

**Architecture:** A `get_version` handler and `VersionResponse` struct are added to `src/api/handlers.rs`. The route is registered in `src/api/mod.rs` without `AuthGuard`. The `time` crate's `formatting` feature is enabled to produce RFC 3339 timestamps. The endpoint is added to the OpenAPI spec in `src/api/openapi.rs`.

**Tech Stack:** Rust, axum 0.8, utoipa 5, time 0.3 (formatting feature)

---

## File Map

| File | Change |
|------|--------|
| `Cargo.toml` | Enable `formatting` feature on `time` crate |
| `src/api/handlers.rs` | Add `VersionResponse` struct and `get_version` handler |
| `src/api/mod.rs` | Register `GET /api/version` route without `AuthGuard` |
| `src/api/openapi.rs` | Add `get_version` to paths, `VersionResponse` to schemas |
| `tests/test_api.rs` | Add integration test for `GET /api/version` |

---

### Task 1: Write the failing integration test

**Files:**
- Modify: `tests/test_api.rs`

- [ ] **Step 1: Add the failing test to `tests/test_api.rs`**

Append at the end of the file (after the last `}`):

```rust
#[tokio::test]
async fn get_version_returns_200_without_auth() {
    let app = common::make_app(false);
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/version")
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

- [ ] **Step 2: Run the test to confirm it fails**

```bash
cargo test get_version_returns_200_without_auth 2>&1
```

Expected: FAIL — `404 Not Found` or similar (route doesn't exist yet).

---

### Task 2: Implement the handler, route, and OpenAPI spec

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/api/handlers.rs`
- Modify: `src/api/mod.rs`
- Modify: `src/api/openapi.rs`

- [ ] **Step 1: Enable `formatting` feature on `time` in `Cargo.toml`**

Change:
```toml
time = "0.3"
```
To:
```toml
time = { version = "0.3", features = ["formatting"] }
```

- [ ] **Step 2: Add `VersionResponse` struct and `get_version` handler to `src/api/handlers.rs`**

Add after the existing imports at the top of the file (the `use` block ends at line 13):

```rust
use time::format_description::well_known::Rfc3339;
```

Then append after the last `}` of `post_light` (before `#[cfg(test)]`):

```rust
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct VersionResponse {
    pub version: &'static str,
    pub time: String,
}

#[utoipa::path(
    get,
    path = "/api/version",
    responses(
        (status = 200, description = "Server version and current UTC time", body = VersionResponse),
    )
)]
pub async fn get_version() -> impl IntoResponse {
    let time = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
        time,
    })
}
```

- [ ] **Step 3: Register the route in `src/api/mod.rs`**

The current `build_router` function (lines 18–26) reads:

```rust
pub fn build_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/light", get(handlers::get_light))
        .route("/light", post(handlers::post_light))
        .with_state(state);

    let swagger_routes = openapi::swagger_router();

    Router::new().nest("/api", api_routes).merge(swagger_routes)
}
```

Change it to:

```rust
pub fn build_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/light", get(handlers::get_light))
        .route("/light", post(handlers::post_light))
        .route("/version", get(handlers::get_version))
        .with_state(state);

    let swagger_routes = openapi::swagger_router();

    Router::new().nest("/api", api_routes).merge(swagger_routes)
}
```

- [ ] **Step 4: Add `get_version` and `VersionResponse` to `src/api/openapi.rs`**

Current file:

```rust
use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::handlers;
use crate::device::LightState;

#[derive(OpenApi)]
#[openapi(
    paths(handlers::get_light, handlers::post_light),
    components(schemas(LightState)),
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

Replace with:

```rust
use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::handlers::{self, VersionResponse};
use crate::device::LightState;

#[derive(OpenApi)]
#[openapi(
    paths(handlers::get_light, handlers::post_light, handlers::get_version),
    components(schemas(LightState, VersionResponse)),
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

- [ ] **Step 5: Run all tests**

```bash
cargo test 2>&1
```

Expected: all tests pass including `get_version_returns_200_without_auth`.

- [ ] **Step 6: Check formatting and clippy**

```bash
cargo fmt && cargo clippy 2>&1
```

Expected: no warnings, no errors.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/api/handlers.rs src/api/mod.rs src/api/openapi.rs tests/test_api.rs
git commit -m "feat: add GET /api/version endpoint (no auth, RFC 3339 time)"
```

---

### Task 3: Bump version and create GitHub release

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Bump version in `Cargo.toml`**

Change:
```toml
version = "0.0.4"
```
To:
```toml
version = "0.0.5"
```

- [ ] **Step 2: Verify the build still compiles cleanly**

```bash
cargo check 2>&1 | tail -3
```

Expected: `Finished` line with version `0.0.5`.

- [ ] **Step 3: Commit version bump**

```bash
git add Cargo.toml
git commit -m "chore: bump version to 0.0.5"
git push
```

- [ ] **Step 4: Create GitHub release v0.0.5**

```bash
gh release create v0.0.5 \
  --title "v0.0.5" \
  --target main \
  --notes "## What's Changed

### New Features
- **GET /api/version**: New public endpoint (no authentication required) returning the running server version and current UTC time in RFC 3339 format.

\`\`\`json
{\"version\": \"0.0.5\", \"time\": \"2026-05-18T14:32:01Z\"}
\`\`\`"
```

Expected: a URL like `https://github.com/ftaeger/rustylight-server/releases/tag/v0.0.5`.
