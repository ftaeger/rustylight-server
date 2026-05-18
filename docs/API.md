# rustylight-server API Reference

HTTP REST API for controlling a Kuando Busylight USB device over HTTPS with X-Api-Key authentication.

---

## Transport

- **Protocol**: HTTPS only (TLS 1.2+, self-signed ECDSA P-256 certificate)
- **Default port**: `8443`
- **Base URL**: `https://<host>:8443`

TLS certificates are auto-generated on first start and stored at the paths configured in `[tls]`. Clients must either trust the self-signed certificate or disable certificate verification.

---

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

---

## Data Types

### LightState

Represents the desired or current state of the light. Used both as the POST request body and embedded in the GET response.

| Field | Type | Required | Range / Default | Description |
|-------|------|----------|-----------------|-------------|
| `on` | boolean | Yes | — | Power on (`true`) or off (`false`) |
| `r` | integer | Yes | 0–255 | Red channel of primary color |
| `g` | integer | Yes | 0–255 | Green channel of primary color |
| `b` | integer | Yes | 0–255 | Blue channel of primary color |
| `blink` | boolean | No | `false` | Enable blinking |
| `blink_on_ms` | integer | No | 50–10000, default 500 | Duration light is on per blink cycle (ms) |
| `blink_off_ms` | integer | No | 50–10000, default 500 | Duration light is off per blink cycle (ms) |
| `r2` | integer | No | 0–255 | Red channel of secondary blink color |
| `g2` | integer | No | 0–255 | Green channel of secondary blink color |
| `b2` | integer | No | 0–255 | Blue channel of secondary blink color |

**Notes on blink behavior:**
- If `blink` is `false`, `blink_on_ms`, `blink_off_ms`, `r2`, `g2`, `b2` are ignored.
- If `blink` is `true` and `r2`/`g2`/`b2` are set, the light alternates between the primary color (on phase) and the secondary color (off phase). If secondary color is omitted the light turns off during the off phase.
- `blink_on_ms` and `blink_off_ms` are validated only when `blink` is `true`.

### LightResponse

GET `/api/light` returns a `LightState` object plus one extra field:

| Field | Type | Description |
|-------|------|-------------|
| *(all LightState fields)* | — | Current state of the light |
| `connected` | boolean | Whether a Busylight USB device is currently detected |

Optional fields (`blink_on_ms`, `blink_off_ms`, `r2`, `g2`, `b2`) are **omitted** from the response when they are not set (not serialized as `null`).

---

## Endpoints

### GET /api/light

Returns the current light state and device connection status.

**Request headers**: `X-Api-Key` (see Authentication)

**Request body**: none

**Success response — 200 OK**
```json
{
  "on": true,
  "r": 255,
  "g": 0,
  "b": 0,
  "blink": false,
  "connected": true
}
```

```json
{
  "on": true,
  "r": 255,
  "g": 0,
  "b": 0,
  "blink": true,
  "blink_on_ms": 500,
  "blink_off_ms": 500,
  "r2": 0,
  "g2": 0,
  "b2": 255,
  "connected": true
}
```

**Error responses**: see Error Reference below.

---

### POST /api/light

Sets a new light state.

**Request headers**: `X-Api-Key`, `Content-Type: application/json`

**Request body**: a `LightState` JSON object

**Success response — 200 OK**
```json
{"ok": true}
```

**Error responses**: see Error Reference below.

---

### GET /api

Swagger UI — interactive browser-based API explorer. No authentication required.

---

### GET /api/openapi.json

OpenAPI 3.0 specification as JSON. No authentication required. Useful for generating client SDKs.

---

## Request Examples

### Turn the light on — solid red

```http
POST /api/light HTTP/1.1
Host: busylight.local:8443
Content-Type: application/json
X-Api-Key: <your-psk>

{"on": true, "r": 255, "g": 0, "b": 0}
```

### Turn the light off

```http
POST /api/light HTTP/1.1
Host: busylight.local:8443
Content-Type: application/json
X-Api-Key: <your-psk>

{"on": false, "r": 0, "g": 0, "b": 0}
```

Note: `on: false` disables the light regardless of the color values. Sending `r`/`g`/`b` is still required by the schema.

### Blink red/off at 1 Hz

```http
POST /api/light HTTP/1.1
...

{"on": true, "r": 255, "g": 0, "b": 0, "blink": true, "blink_on_ms": 500, "blink_off_ms": 500}
```

### Blink red/blue alternating

```http
POST /api/light HTTP/1.1
...

{
  "on": true,
  "r": 255, "g": 0, "b": 0,
  "blink": true,
  "blink_on_ms": 300,
  "blink_off_ms": 300,
  "r2": 0, "g2": 0, "b2": 255
}
```

### Read current state

```http
GET /api/light HTTP/1.1
Host: busylight.local:8443
X-Api-Key: <your-psk>
```

---

## Error Reference

All error responses use `Content-Type: application/json` and body `{"error": "<message>"}`.

| HTTP Status | Error message | Cause |
|-------------|---------------|-------|
| 400 | `invalid JSON: <detail>` | POST body is not valid JSON or wrong type |
| 400 | `blink_on_ms must be 50–10000, got <n>` | Out of range while `blink` is `true` |
| 400 | `blink_off_ms must be 50–10000, got <n>` | Out of range while `blink` is `true` |
| 401 | `missing header: X-Api-Key` | `X-Api-Key` header absent |
| 401 | `invalid API key` | `X-Api-Key` value does not match PSK |
| 503 | `Busylight not connected` | No compatible USB device detected |

---

## Configuration File

**Default path**: `/etc/rustylight/rustylight.conf`
**Format**: TOML

```toml
[server]
port = 8443                                      # HTTPS listening port

[tls]
cert_file = "/etc/rustylight/tls.crt"           # PEM certificate (auto-generated if missing)
key_file  = "/etc/rustylight/tls.key"           # PEM private key (auto-generated if missing)

[auth]
psk = ""                                         # 64-char hex string; auto-generated on first start

[logging]
level    = "info"                                # One of: trace | debug | info | warn | error
log_file = "/var/log/rustylight/rustylight.log"  # Log file path (created if missing)
```

The PSK is auto-generated on first start if left empty, then written back to the config file. Copy the generated value from the config before building clients.

---

## Supported Hardware

The server auto-detects any of the following Kuando Busylight models by USB VID/PID:

| Model | VID | PID |
|-------|-----|-----|
| Busylight UC Omega | 0x04D8 | 0xF848 |
| Busylight Alpha | 0x27BB | 0x3BCA |
| Busylight Alpha | 0x27BB | 0x3BCB |
| Busylight Omega | 0x27BB | 0x3BCD |
| Busylight UC | 0x04D8 | 0xF8F8 |
| Busylight Lync | 0x04D8 | 0x2013 |
| Busylight Lync | 0x04D8 | 0x2014 |
| Busylight UC2 | 0x27BB | 0x3BC8 |
| Busylight UC2 | 0x27BB | 0x3BC9 |

The USB manager polls every 2 seconds. The `connected` field in the GET response reflects live device availability.

---

## Quick Start Checklist for Client Developers

1. Retrieve the PSK from `/etc/rustylight/rustylight.conf` → `auth.psk`.
2. Configure your HTTP client to accept self-signed TLS certificates, or import the server's certificate.
3. For every request, send `X-Api-Key: <psk>` as a header.
4. If you receive a 503, the USB device is not plugged in — retry after reconnecting.
