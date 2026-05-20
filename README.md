# rustylight-server

A Rust Linux daemon that controls a [Kuando Busylight](https://www.kuando.com/busylight/) USB device via a TLS-encrypted REST API.

## Supported Hardware

The server supports the following Kuando Busylight models, detected automatically by USB VID/PID at startup and on reconnection.

| Vendor | Product | USB ID | Verified |
|--------|---------|--------|:--------:|
| Kuando | Busylight UC Omega | `04d8:f848` | — |
| Kuando | Busylight UC | `04d8:f8f8` | — |
| Kuando | Busylight Lync | `04d8:2013` | — |
| Kuando | Busylight Lync | `04d8:2014` | — |
| Kuando | Busylight Alpha | `27bb:3bca` | — |
| Kuando | Busylight Alpha | `27bb:3bcb` | — |
| Kuando | Busylight Omega | `27bb:3bcd` | ✅ |
| Kuando | Busylight UC2 | `27bb:3bc8` | — |
| Kuando | Busylight UC2 | `27bb:3bc9` | — |

## Installation

### Package Repository (Recommended)

#### Debian / Ubuntu — Modern (.sources, Debian 13+ / Ubuntu 24.04+)

Optional: verify the key fingerprint before importing (this instance's fingerprint: `80A3 D1FA 02E2 97F5 5F73 B8C2 AB16 B30A 66F4 1FFB`):

```bash
curl -fsSL https://reprox.dev/ftaeger/rustylight-server/public.key | gpg --show-keys
```

Import the signing key:

```bash
curl -fsSL https://reprox.dev/ftaeger/rustylight-server/public.key | \
  sudo gpg --dearmor -o /etc/apt/keyrings/rustylight-server.gpg
```

Add the repository. To include pre-release versions, replace the URL with `https://reprox.dev/ftaeger/rustylight-server/prerelease`:

```bash
sudo tee /etc/apt/sources.list.d/rustylight-server.sources << EOF
Types: deb
URIs: https://reprox.dev/ftaeger/rustylight-server
Suites: stable
Components: main
Signed-By: /etc/apt/keyrings/rustylight-server.gpg
EOF
```

Install:

```bash
sudo apt update && sudo apt install rustylight-server
```

#### Debian / Ubuntu — Legacy (.list format)

Optional: verify the key fingerprint before importing (this instance's fingerprint: `80A3 D1FA 02E2 97F5 5F73 B8C2 AB16 B30A 66F4 1FFB`):

```bash
curl -fsSL https://reprox.dev/ftaeger/rustylight-server/public.key | gpg --show-keys
```

Import the signing key:

```bash
curl -fsSL https://reprox.dev/ftaeger/rustylight-server/public.key | \
  sudo gpg --dearmor -o /etc/apt/keyrings/rustylight-server.gpg
```

Add the repository. To include pre-release versions, replace the URL with `https://reprox.dev/ftaeger/rustylight-server/prerelease`:

```bash
echo "deb [signed-by=/etc/apt/keyrings/rustylight-server.gpg] https://reprox.dev/ftaeger/rustylight-server stable main" | \
  sudo tee /etc/apt/sources.list.d/rustylight-server.list
```

Install:

```bash
sudo apt update && sudo apt install rustylight-server
```

#### Fedora / RHEL / CentOS

Add the repository. To include pre-release versions, replace the `baseurl` with `https://reprox.dev/ftaeger/rustylight-server/prerelease`:

```bash
sudo tee /etc/yum.repos.d/rustylight-server.repo << EOF
[rustylight-server]
name=rustylight-server from GitHub via Reprox
baseurl=https://reprox.dev/ftaeger/rustylight-server
enabled=1
gpgcheck=0
repo_gpgcheck=1
gpgkey=https://reprox.dev/ftaeger/rustylight-server/public.key
EOF
```

Install (the GPG key fingerprint `80A3 D1FA 02E2 97F5 5F73 B8C2 AB16 B30A 66F4 1FFB` will be shown for verification on first install):

```bash
sudo dnf install rustylight-server
```

### Manual Download

#### Debian / Ubuntu / Raspberry Pi

Download the `.deb` for your architecture from the [latest release](../../releases/latest):

```bash
sudo dpkg -i rustylight-server_<version>_<arch>.deb
```

Supported architectures: `amd64`, `i386`, `arm64` (Pi 3/4/Zero 2W 64-bit), `armhf` (Pi 3/4/Zero 2W 32-bit).

#### RHEL / Rocky Linux / AlmaLinux 8, 9, 10

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
{"on": false, "r": 0, "g": 0, "b": 0}
```

### `GET /api/public/healthcheck`

Reports service health — no authentication required:

```json
{"status": "ok", "busylight_connected": true, "log_writable": true}
```

Returns 200 when all checks pass, 503 with `"status": "degraded"` when any check fails.

### `GET /api/public/version`

Returns server version and current UTC time — no authentication required.

## Service Management

```bash
sudo systemctl start rustylight
sudo systemctl stop rustylight
sudo systemctl status rustylight
sudo journalctl -u rustylight -f
```

### Memory Usage

```bash
systemctl show --property=MemoryCurrent rustylight
```

On Raspberry Pi OS this requires enabling the memory cgroup controller. Add `cgroup_enable=memory cgroup_memory=1` to `/boot/firmware/cmdline.txt` (bookworm) or `/boot/cmdline.txt` (bullseye) on a single line, then reboot. See `docs/API.md` for details.

## Logs

`/var/log/rustylight/rustylight.log` — rotated daily, compressed after 2 days, deleted after 30 days.
