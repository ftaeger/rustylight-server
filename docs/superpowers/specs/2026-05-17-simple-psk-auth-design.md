# Simple PSK Authentication Design

## Goal

Replace the HMAC-SHA256 + timestamp authentication with a plain API key check: the client passes the PSK in an `X-Api-Key` header and the server accepts or rejects it.

## Architecture

Auth is implemented as an Axum `FromRequestParts` extractor (`AuthGuard`) that reads the `X-Api-Key` header and compares it to the stored PSK string. Because it uses `FromRequestParts` (not `FromRequest`), it does not consume the request body, allowing handlers to read the body independently via a `Bytes` extractor.

The PSK is stored as a plain `Arc<String>` in `AppState` and as a 64-character hex string in the config file. Generation and config handling are simplified accordingly.

## Components

### `src/api/auth.rs`

- `AuthGuard` — unit struct, implements `FromRequestParts<S>`
- Reads `X-Api-Key` header; compares value to `app_state.psk` with `==`
- `AuthError` has two variants:

| Variant | Status | Body |
|---------|--------|------|
| `MissingKey` | 401 | `{"error": "missing header: X-Api-Key"}` |
| `InvalidKey` | 401 | `{"error": "invalid API key"}` |

- All HMAC, timestamp, signature, and hex/base64 logic is deleted
- Unit tests: missing header → 401, wrong key → 401

### `src/api/mod.rs`

- `AppState.psk`: `Arc<Vec<u8>>` → `Arc<String>`

### `src/api/handlers.rs`

- `get_light`: no change to signature — `_auth: AuthGuard` stays, `AuthGuard` is now `FromRequestParts` so ordering constraints relax
- `post_light`: `AuthGuard(body_bytes): AuthGuard` → `_auth: AuthGuard`; body read via `body: Bytes` as last parameter; manual JSON parsing and custom error messages preserved
- `utoipa` param annotations: remove `X-Timestamp`/`X-Signature`, add `X-Api-Key`

### `src/config.rs`

- `generate_psk()`: produces 64-char hex string (32 random bytes via `rand`, formatted with `{:02x}`) — no encoding crate needed
- `decode_psk()`: deleted
- `ensure_psk()` and `load_or_create()`: unchanged in behaviour; `psk` field in config stays a plain `String`
- Unit test `generates_non_empty_psk`: drop the `URL_SAFE.decode` assertion; just assert non-empty

### `Cargo.toml`

Remove: `hmac`, `sha2`, `hex`, `base64`
Keep: `rand` (PSK generation)

### `tests/common/mod.rs`

- `test_psk()`: returns `String` (`"test-psk-for-integration-tests!!"`)
- `auth_headers()`: returns `vec![("X-Api-Key", psk)]` — no timestamp, no signature computation

### `tests/test_auth.rs`

Delete: timestamp/signature/window tests
Add:
- missing `X-Api-Key` → 401
- wrong `X-Api-Key` → 401
- correct `X-Api-Key` → 200

### `examples/set-color.sh` and `examples/get-state.sh`

- Remove: PSK hex decoding, `openssl dgst` signing, `od`, timestamp computation
- Replace with: `-H "X-Api-Key: ${PSK}"` on the `curl` call
- Interface unchanged: `-k PSK` flag and `$RUSTYLIGHT_PSK` env var still work

### `docs/API.md`

- Rewrite Authentication section: single `X-Api-Key: <psk>` header
- Replace Python/JS/Rust signing examples with minimal header-setting examples
- Update error reference table (remove 403 timestamp/signature rows)
- Update quick-start checklist

## Error Reference (after change)

| Status | Condition |
|--------|-----------|
| 401 | `X-Api-Key` header absent |
| 401 | `X-Api-Key` value does not match PSK |
| 400 | Invalid JSON or validation failure (unchanged) |
| 503 | Device not connected (unchanged) |
