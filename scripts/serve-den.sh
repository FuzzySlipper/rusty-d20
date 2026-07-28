#!/usr/bin/env bash
set -euo pipefail

D20_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
D20_BIND_HOST=""
D20_BIND_PORT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --)
      shift
      ;;
    --host)
      D20_BIND_HOST="${2:-}"
      shift 2
      ;;
    --port)
      D20_BIND_PORT="${2:-}"
      shift 2
      ;;
    *)
      echo "unknown serve-den argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ -z "$D20_BIND_HOST" ]]; then
  echo "--host is required" >&2
  exit 2
fi
if [[ ! "$D20_BIND_PORT" =~ ^[0-9]+$ ]] || (( D20_BIND_PORT < 1 || D20_BIND_PORT > 65535 )); then
  echo "--port must be an integer from 1 through 65535" >&2
  exit 2
fi

cd "$D20_ROOT"
D20_NX="$D20_ROOT/node_modules/.bin/nx"
if [[ ! -x "$D20_NX" ]]; then
  echo "workspace dependencies are missing; run pnpm install --frozen-lockfile" >&2
  exit 1
fi

"$D20_NX" build app
exec cargo run --locked -p rusty-d20 --bin rusty-d20-host -- \
  --address "${D20_BIND_HOST}:${D20_BIND_PORT}"
