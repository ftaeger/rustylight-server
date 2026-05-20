# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-05-19

### Added
- `memory_accounting` field in `GET /api/public/healthcheck` response (resident set size in bytes)

## [0.1.0] - 2026-05-19

### Added
- `GET /api/public/healthcheck` endpoint (no authentication required)

### Changed
- Renamed `/api/version` to `/api/public/version` for consistency with other public endpoints

## [0.0.9] - 2026-05-18

### Fixed
- Rewrote HID report builder to use the correct Kuando opcode protocol
- Corrected Busylight Omega VID/PID label (27BB:3BCD was mislabelled as Alpha in docs)

## [0.0.8] - 2026-05-18

### Fixed
- Corrected Busylight step byte layout to `[Repeat, R, G, B, OnHi, OnLo, OffHi, OffLo]`

## [0.0.7] - 2026-05-18

### Fixed
- Rewrote HID report builder with correct Busylight UC protocol and checksum calculation

## [0.0.6] - 2026-05-18

### Fixed
- Removed HID Report ID prefix byte — Linux `hidraw` expects the 64-byte payload directly

## [0.0.5] - 2026-05-18

### Added
- `GET /api/public/version` endpoint (no authentication required), returns server version and UTC time in RFC 3339 format

## [0.0.4] - 2026-05-18

### Fixed
- Prepend HID Report ID byte (`0x00`) to USB write buffer for correct kernel handling

## [0.0.3] - 2026-05-18

### Fixed
- Added udev rules (`99-busylight.rules`) so the `plugdev` group can open the HID device without root

## [0.0.2] - 2026-05-18

### Changed
- Replaced HMAC-SHA256 authentication with simpler `X-Api-Key` header authentication

## [0.0.1] - 2026-05-16

### Added
- Initial release: HTTPS REST API server for Kuando Busylight control on Raspberry Pi
- TLS with auto-generated self-signed ECDSA P-256 certificate
- `GET /api/light` and `POST /api/light` endpoints with PSK authentication
- Background USB polling loop with hot-plug detection and keepalive
- Swagger UI at `/api` and OpenAPI spec at `/api/openapi.json`
- `.deb` and `.rpm` packages for `aarch64`, `armv7`, `x86_64`, `i686`

[unreleased]: https://github.com/ftaeger/rustylight-server/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/ftaeger/rustylight-server/compare/v0.1.0...v1.0.0
[0.1.0]: https://github.com/ftaeger/rustylight-server/compare/v0.0.9...v0.1.0
[0.0.9]: https://github.com/ftaeger/rustylight-server/compare/v0.0.8...v0.0.9
[0.0.8]: https://github.com/ftaeger/rustylight-server/compare/v0.0.7...v0.0.8
[0.0.7]: https://github.com/ftaeger/rustylight-server/compare/v0.0.6...v0.0.7
[0.0.6]: https://github.com/ftaeger/rustylight-server/compare/v0.0.5...v0.0.6
[0.0.5]: https://github.com/ftaeger/rustylight-server/compare/v0.0.4...v0.0.5
[0.0.4]: https://github.com/ftaeger/rustylight-server/compare/v0.0.3...v0.0.4
[0.0.3]: https://github.com/ftaeger/rustylight-server/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/ftaeger/rustylight-server/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/ftaeger/rustylight-server/releases/tag/v0.0.1
