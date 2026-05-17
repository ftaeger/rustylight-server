#!/usr/bin/env bash
# Read the current light state from rustylight-server and print it.
#
# Usage:
#   ./get-state.sh [OPTIONS]
#
# Options:
#   -h HOST   Server hostname or IP  (default: localhost)
#   -p PORT   Server port            (default: 8443)
#   -k PSK    API key                (default: $RUSTYLIGHT_PSK env var)
#
# Examples:
#   RUSTYLIGHT_PSK=<psk> ./get-state.sh
#   RUSTYLIGHT_PSK=<psk> ./get-state.sh -h 192.168.1.10 -p 8443

set -euo pipefail

HOST="localhost"
PORT="8443"
PSK="${RUSTYLIGHT_PSK:-}"

usage() {
  sed -n '3,16p' "$0" | sed 's/^# \?//'
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h) HOST="$2"; shift 2 ;;
    -p) PORT="$2"; shift 2 ;;
    -k) PSK="$2"; shift 2 ;;
    --) shift; break ;;
    -*) echo "Unknown option: $1" >&2; usage ;;
    *) break ;;
  esac
done

if [[ -z "$PSK" ]]; then
  echo "Error: PSK not set. Use -k <psk> or set RUSTYLIGHT_PSK." >&2
  exit 1
fi

RESPONSE="$(curl --silent --show-error \
  --insecure \
  --request GET \
  --url "https://${HOST}:${PORT}/api/light" \
  --header "X-Api-Key: ${PSK}")"

if command -v jq &>/dev/null; then
  printf '%s\n' "$RESPONSE" | jq .
else
  printf '%s\n' "$RESPONSE"
fi
