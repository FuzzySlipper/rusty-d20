#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
engine=${RUSTY_ENGINE_ROOT:-"$root/../rusty-engine"}
project="$root/src/RustyD20.NativeProduct/RustyD20.NativeProduct.csproj"
host_root="$root/src/RustyD20.NativeProduct/DevelopmentHost"
output="$root/src/RustyD20.NativeProduct/bin/Release/net10.0/linux-x64/publish"
run_dir=$(mktemp -d); persistence_root="$run_dir/persistence"; log=; sse_log=; pid=; sse_pid=; origin=
cleanup() {
  stop_host || true
  [[ "${D20_EXERCISE_KEEP_RUN_DIR:-}" == "1" ]] || rm -rf "$run_dir"
}; trap cleanup EXIT
if [[ "${D20_EXERCISE_SKIP_BUILD:-}" != "1" ]]; then
  dotnet publish "$project" -c Release -r linux-x64 >/dev/null
  cargo build --quiet --manifest-path "$engine/rust/crates/csharp-product-runtime/Cargo.toml" --bin csharp-product-runtime --locked
  node "$root/src/scripts/generate-development-host-browser.mjs"
fi
args=(); for intent in d20.select.warden d20.select.ember d20.begin d20.forward d20.back d20.left d20.right d20.interact d20.party.next d20.action.next d20.target.next d20.action.commit d20.reaction.choose d20.reaction.decline d20.outcome.continue d20.save d20.load d20.reset; do args+=(--direct-intent "$intent=digital"); done
start_host() {
  local label=$1; log="$run_dir/$label.host.log"; sse_log="$run_dir/$label.outputs.sse"; origin=
  "$engine/target/debug/csharp-product-runtime" --library "$output/RustyD20.NativeProduct.so" --bundle-dir "$host_root/generated-browser" --content-dir "$host_root/content" --persistence-root "$persistence_root" --mode demand --port 0 "${args[@]}" >"$log" 2>&1 & pid=$!
  for _ in {1..300}; do origin=$(sed -n 's/^C# NativeAOT product host listening at //p' "$log" | tail -1); [[ -n "$origin" ]] && break; kill -0 "$pid" || { cat "$log"; return 1; }; sleep .05; done
  [[ -n "$origin" ]] || { cat "$log"; return 1; }
  curl --fail --silent --no-buffer -H 'Accept: text/event-stream' "$origin/__rusty/product/runtime/outputs" >"$sse_log" & sse_pid=$!
  sleep .1; kill -0 "$sse_pid"
}
stop_host() {
  if [[ -n "$sse_pid" ]] && kill -0 "$sse_pid" 2>/dev/null; then kill "$sse_pid"; wait "$sse_pid" || true; fi; sse_pid=
  if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then kill "$pid"; wait "$pid" || true; fi; pid=
}
lifecycle() { curl --fail --silent -H 'Content-Type: application/json' --data "{\"runtime\":$2}" "$origin/__rusty/product/runtime/lifecycle/$1"; }
send() { jq -cn --argjson runtime "$1" --arg intent "$2" --arg sequence "$3" '{batch:[{runtime:$runtime,sequence:$sequence,context:"gameplay.default",intent:$intent,value:{kind:"digital",active:true}}]}' | curl --fail --silent -H 'Content-Type: application/json' --data @- "$origin/__rusty/product/runtime/input"; }
step() { curl --fail --silent -H 'Content-Type: application/json' --data '{}' "$origin/__rusty/product/runtime/admit-demand-step"; }
events() { awk '/^data: / { sub(/^data: /, ""); print }' "$sse_log"; }
assert_projection() { for _ in {1..200}; do events | jq -es 'any(.kind == "frame") and any(.kind == "ui-projection" and .envelope.stream == "rusty-d20" and .envelope.contract == "rusty-d20.workbench.v1" and (.envelope.value | type == "object") and ((.envelope.value | tostring | length) <= 8192) and .envelope.value["status.lifecycle"] == "running" and .envelope.value["campaign.phase"] == "Camp" and (.envelope.value["content.source"] | type == "string") and (.envelope.value["content.fingerprint"] | type == "string") and (.envelope.value["presentation.source"] | type == "string") and (.envelope.value["presentation.adventure"] == "wardens-gate") and (.envelope.value["presentation.tuning"] | type == "string") and (.envelope.value["tuning.viewDepth"] == 3))' >/dev/null && return; sleep .05; done; return 1; }
grep -Fq 'Rusty D20 NativeAOT Engine workbench' "$host_root/browser/index.html"
start_host first
started=$(lifecycle start null); jq -e '.accepted == true and .readout.state == "running"' <<<"$started" >/dev/null; runtime=$(jq -c '.binding' <<<"$started")
assert_projection
send "$runtime" d20.begin 1 | jq -e '.accepted == true' >/dev/null; step | jq -e '.accepted == true' >/dev/null
for _ in {1..100}; do events | jq -es 'any(.kind == "ui-projection" and .envelope.value["campaign.phase"] == "Exploration")' >/dev/null && break; sleep .05; done
events | jq -es 'any(.kind == "ui-projection" and .envelope.value["campaign.phase"] == "Exploration")' >/dev/null
paused=$(lifecycle pause "$runtime"); jq -e '.accepted == true and .readout.state == "paused"' <<<"$paused" >/dev/null
inactive=$(send "$(jq -c '.binding' <<<"$paused")" d20.forward 2); jq -e '.accepted == false or .count == 0' <<<"$inactive" >/dev/null
resumed=$(lifecycle resume "$(jq -c '.binding' <<<"$paused")"); jq -e '.accepted == true and .readout.state == "running"' <<<"$resumed" >/dev/null; rebound=$(jq -c '.binding' <<<"$resumed"); [[ "$rebound" != "$runtime" ]]
stale=$(send "$runtime" d20.forward 1); jq -e '.accepted == false or .count == 0' <<<"$stale" >/dev/null
send "$rebound" d20.left 1 | jq -e '.accepted == true and .count == 1' >/dev/null; step | jq -e '.accepted == true' >/dev/null
send "$rebound" d20.save 2 | jq -e '.accepted == true' >/dev/null; step | jq -e '.accepted == true' >/dev/null
shutdown=$(lifecycle shutdown "$rebound"); jq -e '.accepted == true' <<<"$shutdown" >/dev/null; stop_host
start_host second
started=$(lifecycle start null); jq -e '.accepted == true and .readout.state == "running"' <<<"$started" >/dev/null; runtime=$(jq -c '.binding' <<<"$started")
send "$runtime" d20.load 1 | jq -e '.accepted == true' >/dev/null; step | jq -e '.accepted == true' >/dev/null
for _ in {1..100}; do events | jq -es 'any(.kind == "ui-projection" and .envelope.value["campaign.phase"] == "Exploration" and (.envelope.value["content.source"] | type == "string") and (.envelope.value["content.fingerprint"] | type == "string"))' >/dev/null && break; sleep .05; done
events | jq -es 'any(.kind == "ui-projection" and .envelope.value["campaign.phase"] == "Exploration" and (.envelope.value["content.source"] | type == "string") and (.envelope.value["content.fingerprint"] | type == "string"))' >/dev/null
shutdown=$(lifecycle shutdown "$runtime"); jq -e '.accepted == true' <<<"$shutdown" >/dev/null; stop_host
echo 'NativeAOT Engine lifecycle, binding fence, UI/frame, and fresh-process save/load exercise passed'
