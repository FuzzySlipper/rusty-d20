#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

proof_output=$(mktemp -t rusty-d20-native-proof.XXXXXX.log)
cleanup() {
  status=$?
  if ((status != 0)); then
    echo 'native proof log:' >&2
    tail -n 120 "$proof_output" >&2 || true
  fi
  rm -f "$proof_output"
  trap - EXIT
  exit "$status"
}
trap cleanup EXIT

if [[ "$(uname -s)" == "Linux" ]]; then
  cargo build -p rusty-d20 --bin rusty-d20-native --locked
  xvfb-run -a ./scripts/run-native-host-proof-linux.sh "$proof_output"
else
  echo 'verify-native-host requires Linux/X11 input automation' >&2
  exit 1
fi

grep -F \
  'RUSTY_D20_NATIVE_PROOF_OK frame=true views=true camera=true resize=true input_authority=true input_noop=true pick_authority=true pick_miss=true state=true render=true save_round_trip=true lifecycle=disposed' \
  "$proof_output"
