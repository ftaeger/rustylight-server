#!/usr/bin/env bash
# Send a color command to rustylight-server.
#
# Usage:
#   ./set-color.sh [OPTIONS] <r> <g> <b>
#   ./set-color.sh [OPTIONS] off
#
# Options:
#   -h HOST     Server hostname or IP  (default: localhost)
#   -p PORT     Server port            (default: 8443)
#   -k PSK      API key                (default: $RUSTYLIGHT_PSK env var)
#   --blink     Enable blinking
#   --on-ms N   Blink on duration  in ms, 50–10000 (default: 500)
#   --off-ms N  Blink off duration in ms, 50–10000 (default: 500)
#   --r2 N      Secondary blink color red   (0–255)
#   --g2 N      Secondary blink color green (0–255)
#   --b2 N      Secondary blink color blue  (0–255)
#
# Examples:
#   RUSTYLIGHT_PSK=<psk> ./set-color.sh 255 0 0          # solid red
#   RUSTYLIGHT_PSK=<psk> ./set-color.sh off              # turn off
#   RUSTYLIGHT_PSK=<psk> ./set-color.sh --blink 255 0 0  # blinking red
#   RUSTYLIGHT_PSK=<psk> ./set-color.sh --blink --r2 0 --g2 0 --b2 255 255 0 0  # red/blue blink

set -euo pipefail

HOST="localhost"
PORT="8443"
PSK="${RUSTYLIGHT_PSK:-}"
BLINK="false"
ON_MS=""
OFF_MS=""
R2=""
G2=""
B2=""

usage() {
  sed -n '3,28p' "$0" | sed 's/^# \?//'
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h) HOST="$2"; shift 2 ;;
    -p) PORT="$2"; shift 2 ;;
    -k) PSK="$2"; shift 2 ;;
    --blink) BLINK="true"; shift ;;
    --on-ms) ON_MS="$2"; shift 2 ;;
    --off-ms) OFF_MS="$2"; shift 2 ;;
    --r2) R2="$2"; shift 2 ;;
    --g2) G2="$2"; shift 2 ;;
    --b2) B2="$2"; shift 2 ;;
    --) shift; break ;;
    -*) echo "Unknown option: $1" >&2; usage ;;
    *) break ;;
  esac
done

if [[ -z "$PSK" ]]; then
  echo "Error: PSK not set. Use -k <psk> or set RUSTYLIGHT_PSK." >&2
  exit 1
fi

if [[ "${1:-}" == "off" ]]; then
  BODY='{"on":false,"r":0,"g":0,"b":0}'
else
  [[ $# -lt 3 ]] && { echo "Error: expected <r> <g> <b> or 'off'" >&2; usage; }
  R="$1"; G="$2"; B="$3"

  BODY="{\"on\":true,\"r\":${R},\"g\":${G},\"b\":${B},\"blink\":${BLINK}"
  [[ -n "$ON_MS"  ]] && BODY="${BODY},\"blink_on_ms\":${ON_MS}"
  [[ -n "$OFF_MS" ]] && BODY="${BODY},\"blink_off_ms\":${OFF_MS}"
  [[ -n "$R2"     ]] && BODY="${BODY},\"r2\":${R2}"
  [[ -n "$G2"     ]] && BODY="${BODY},\"g2\":${G2}"
  [[ -n "$B2"     ]] && BODY="${BODY},\"b2\":${B2}"
  BODY="${BODY}}"
fi

curl --silent --show-error \
  --insecure \
  --request POST \
  --url "https://${HOST}:${PORT}/api/light" \
  --header "Content-Type: application/json" \
  --header "X-Api-Key: ${PSK}" \
  --data "$BODY"

echo
