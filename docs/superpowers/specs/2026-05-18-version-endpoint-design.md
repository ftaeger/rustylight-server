# /api/version Endpoint Design

## Goal

Add a public `GET /api/version` endpoint that returns the running server version and current UTC time. No authentication required.

## Architecture

A single handler function added to `src/api/handlers.rs`. Version is embedded at compile time via `env!("CARGO_PKG_VERSION")`; current time is obtained via `time::OffsetDateTime::now_utc()` and formatted as RFC 3339 using the already-present `time` crate. No new dependencies.

## Response Format

**GET /api/version — 200 OK**
```json
{
  "version": "0.0.5",
  "time": "2026-05-18T14:32:01Z"
}
```

`time` is always UTC, formatted as RFC 3339 (`YYYY-MM-DDTHH:MM:SSZ`).

## Components

### `src/api/handlers.rs`
- New handler `get_version()` — no extractor arguments, no `AuthGuard`.
- Returns `axum::Json` of an inline `VersionResponse` struct (derives `Serialize`, `ToSchema`).
- Annotated with `#[utoipa::path(get, path = "/api/version")]` for OpenAPI.

### `src/api/mod.rs`
- Add `GET /api/version` route to the router without `AuthGuard`, alongside the existing public Swagger routes.

### `src/api/openapi.rs`
- Add `handlers::get_version` to `paths(...)` and `VersionResponse` to `components(schemas(...))` in `ApiDoc`.

## Testing

One integration test in `tests/test_api.rs`:
- `GET /api/version` without `X-Api-Key` header → 200 OK
- Response body contains both `"version"` and `"time"` keys
- `"version"` equals `env!("CARGO_PKG_VERSION")`

## Out of Scope

- No uptime field
- No build timestamp
- No git commit hash
