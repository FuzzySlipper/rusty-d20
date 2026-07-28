#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

pnpm --dir rules install --frozen-lockfile
pnpm --dir rules run verify
cargo test -p rusty-d20 --test d20a0 --locked
