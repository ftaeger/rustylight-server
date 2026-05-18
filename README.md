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

On first start, a random PSK is generated and written to the config. Copy the value from `auth.psk` and share it with API clients.

## Authentication

Every `/api/light` request must include an `X-Api-Key` header with the PSK value from the config:

```
X-Api-Key: <value of auth.psk from /etc/rustylight/rustylight.conf>
```

### Example with curl

```bash
PSK=$(sudo grep 'psk' /etc/rustylight/rustylight.conf | awk -F'"' '{print $2}')

curl -sk \
  -H "X-Api-Key: $PSK" \
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

Steady color:
```json
{"on": true, "r": 0, "g": 255, "b": 0}
```

Blink between color and off:
```json
{"on": true, "r": 255, "g": 0, "b": 0, "blink": true, "blink_on_ms": 500, "blink_off_ms": 500}
```

Blink between two colors:
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
