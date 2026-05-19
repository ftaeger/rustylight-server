# Healthcheck Endpoint Design

## Goal

Add a public `GET /api/public/healthcheck` endpoint that reports whether the service is operating correctly. Returns 200 when all checks pass, 503 when any check fails. No authentication required.

Also move the existing `GET /api/version` endpoint to `GET /api/public/version` to consolidate all public (no-auth) endpoints under the `/api/public/` prefix.

## Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/public/healthcheck` | none | Service health status |
| GET | `/api/public/version` | none | Server version and current UTC time (moved from `/api/version`) |

## Checks

| Field | Source | How |
|-------|--------|-----|
| `busylight_connected` | `SharedState.connected` | Read from the shared mutex — already maintained by the USB manager |
| `log_writable` | log file path from config | Open the configured log file in append mode, close immediately; report `false` on any OS error |

"Service is running" is implied by the endpoint responding at all — no explicit field needed.

## Response Shape

**200 — all checks pass:**
```json
{"status": "ok", "busylight_connected": true, "log_writable": true}
```

**503 — one or more checks fail:**
```json
{"status": "degraded", "busylight_connected": false, "log_writable": false}
```

`status` is always present. Each check is a boolean field. A `false` value on any check means that subsystem is not functioning correctly. No separate error strings — the field names are self-explanatory.

## Architecture

### New / changed files

**`src/api/handlers.rs`**
- New `HealthResponse` struct (derives `Serialize`, `utoipa::ToSchema`) with fields `status: &'static str`, `busylight_connected: bool`, `log_writable: bool`
- New `get_healthcheck(State(state): State<AppState>) -> impl IntoResponse` handler:
  1. Lock `SharedState`, read `connected`
  2. Attempt `OpenOptions::new().write(true).create(true).append(true).open(&state.log_file)` — `log_writable = result.is_ok()`. Using `create(true)` means the check works even before the first log line is written (tracing-appender creates the file lazily); the only side effect is creating a zero-byte file if it didn't already exist, which is acceptable.
  3. If both true → 200 + `{"status":"ok",...}`; otherwise → 503 + `{"status":"degraded",...}`
- `utoipa::path` annotation for OpenAPI

**`src/api/mod.rs`**
- Add `log_file: Arc<String>` to `AppState`
- Register route: `GET /api/public/healthcheck` → `handlers::get_healthcheck`, no auth middleware
- Move `GET /api/version` → `GET /api/public/version` (rename existing route, no handler changes needed)

**`src/main.rs`**
- Pass `config.logging.log_file` into `AppState` when building the router

**`src/api/openapi.rs`**
- Register `get_healthcheck` in the OpenAPI spec paths

**`Cargo.toml`**
- Bump version `0.0.9` → `0.1.0`

**`docs/API.md`**
- Document the new `/api/public/healthcheck` endpoint, response fields, and status codes
- Update `/api/version` references to `/api/public/version`

### No new dependencies required

## Testing

**Integration tests (`tests/test_api.rs`)**
- `GET /api/public/healthcheck` with `connected = true` and a writable temp log file → 200, `status = "ok"`, both booleans `true`
- `GET /api/public/healthcheck` with `connected = false` → 503, `status = "degraded"`, `busylight_connected = false`
- `GET /api/public/healthcheck` with a non-existent/non-writable log path → 503, `status = "degraded"`, `log_writable = false`
- Confirm endpoint requires no `X-Api-Key` header (request without header → 200 when healthy, not 401)
- `GET /api/public/version` returns 200 without auth (renamed from `/api/version`)
- `GET /api/version` no longer exists (would 404)

## Version

Bump to `0.1.0` — first minor version, marks the first complete user-facing feature addition beyond the initial release.
