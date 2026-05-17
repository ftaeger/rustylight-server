# rustylight-server API Reference

HTTP REST API for controlling a Kuando Busylight USB device over HTTPS with HMAC-SHA256 authentication.

---

## Transport

- **Protocol**: HTTPS only (TLS 1.2+, self-signed ECDSA P-256 certificate)
- **Default port**: `8443`
- **Base URL**: `https://<host>:8443`

TLS certificates are auto-generated on first start and stored at the paths configured in `[tls]`. Clients must either trust the self-signed certificate or disable certificate verification.

---

## Authentication

Every `/api/light` request requires two headers:

| Header | Value |
|--------|-------|
| `X-Timestamp` | Current Unix timestamp (seconds, UTC) as a decimal string |
| `X-Signature` | Lowercase hex HMAC-SHA256 signature (see below) |

### Signature computation

```
signature = hex(HMAC-SHA256(
    key    = base64url_decode(psk),
    message = timestamp_string + request_body_bytes
))
```

- **`psk`** — the raw value of `auth.psk` from the config file (Base64URL-encoded 32-byte key)
- **`timestamp_string`** — the exact string sent in `X-Timestamp`
- **`request_body_bytes`** — the raw UTF-8 JSON body for POST; **empty string** (`""`) for GET

The server rejects requests where the timestamp differs from the server clock by more than **30 seconds**. When a request is rejected for this reason the response includes an `X-Server-Time` header with the server's current Unix timestamp so the client can correct its clock offset.

### Example (Python)

```python
import hmac, hashlib, base64, time, json

PSK_B64 = "your_base64url_encoded_psk_here"
psk_bytes = base64.urlsafe_b64decode(PSK_B64 + "==")

def sign(timestamp: str, body: bytes = b"") -> str:
    msg = timestamp.encode() + body
    return hmac.new(psk_bytes, msg, hashlib.sha256).hexdigest()

ts = str(int(time.time()))
headers_get = {
    "X-Timestamp": ts,
    "X-Signature": sign(ts),
}

body = json.dumps({"on": True, "r": 255, "g": 0, "b": 0}).encode()
ts2 = str(int(time.time()))
headers_post = {
    "X-Timestamp": ts2,
    "X-Signature": sign(ts2, body),
    "Content-Type": "application/json",
}
```

### Example (JavaScript / Node.js)

```js
import { createHmac } from "node:crypto";

const pskBytes = Buffer.from(process.env.PSK, "base64url");

function sign(timestamp, body = Buffer.alloc(0)) {
  return createHmac("sha256", pskBytes)
    .update(timestamp)
    .update(body)
    .digest("hex");
}

const ts = String(Math.floor(Date.now() / 1000));
const headersGet = { "X-Timestamp": ts, "X-Signature": sign(ts) };

const bodyBuf = Buffer.from(JSON.stringify({ on: true, r: 255, g: 0, b: 0 }));
const ts2 = String(Math.floor(Date.now() / 1000));
const headersPost = {
  "X-Timestamp": ts2,
  "X-Signature": sign(ts2, bodyBuf),
  "Content-Type": "application/json",
};
```

### Example (Rust)

```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

fn sign(psk: &[u8], timestamp: &str, body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(psk).unwrap();
    mac.update(timestamp.as_bytes());
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}
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

**Request headers**: `X-Timestamp`, `X-Signature` (see Authentication)

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

Sets a new light state. The body bytes are incorporated into the HMAC signature.

**Request headers**: `X-Timestamp`, `X-Signature`, `Content-Type: application/json`

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
X-Timestamp: 1747480000
X-Signature: <computed>

{"on": true, "r": 255, "g": 0, "b": 0}
```

### Turn the light off

```http
POST /api/light HTTP/1.1
Host: busylight.local:8443
Content-Type: application/json
X-Timestamp: 1747480000
X-Signature: <computed>

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
X-Timestamp: 1747480000
X-Signature: <computed, body is empty string>
```

---

## Error Reference

All error responses use `Content-Type: application/json` and body `{"error": "<message>"}`.

| HTTP Status | Error message | Cause |
|-------------|---------------|-------|
| 400 | `invalid JSON: <detail>` | POST body is not valid JSON or a field has the wrong type |
| 400 | `blink_on_ms must be 50–10000, got <n>` | `blink_on_ms` out of range while `blink` is `true` |
| 400 | `blink_off_ms must be 50–10000, got <n>` | `blink_off_ms` out of range while `blink` is `true` |
| 401 | `missing header: X-Timestamp` | `X-Timestamp` header absent |
| 401 | `missing header: X-Signature` | `X-Signature` header absent |
| 401 | `X-Timestamp must be a unix timestamp` | `X-Timestamp` value is not a decimal integer |
| 403 | `timestamp outside ±30s window` | Request timestamp differs from server clock by more than 30 s. Response also includes `X-Server-Time: <unix_ts>` header |
| 403 | `invalid signature` | HMAC signature does not match |
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
psk = ""                                         # Base64URL-encoded 32-byte key; auto-generated on first start
                                                 # After first start this will contain the real PSK value

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
| Busylight UC | 0x04D8 | 0xF8F8 |
| Busylight Lync | 0x04D8 | 0x2013 |
| Busylight Lync | 0x04D8 | 0x2014 |
| Busylight UC2 | 0x27BB | 0x3BC8 |
| Busylight UC2 | 0x27BB | 0x3BC9 |

The USB manager polls every 2 seconds. The `connected` field in the GET response reflects live device availability.

---

## Quick Start Checklist for Client Developers

1. Retrieve the PSK from `/etc/rustylight/rustylight.conf` → `auth.psk` (Base64URL string).
2. Configure your HTTP client to accept self-signed TLS certificates, or import the server's certificate.
3. For every request:
   - Take the current Unix timestamp (integer seconds).
   - Compute HMAC-SHA256 over `(timestamp_string + request_body_bytes)` using the decoded PSK as the key.
   - Send `X-Timestamp` and `X-Signature` headers.
4. If you receive a 403 with `timestamp outside ±30s window`, read the `X-Server-Time` response header and adjust your clock offset.
5. If you receive a 503, the USB device is not plugged in — retry after reconnecting the hardware.
