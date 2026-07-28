# Verification

## Aggregate

```bash
./scripts/verify.sh
```

This runs strict Rust, protocol/boundary, TypeScript, production build, and
real-Rust-host browser checks, including the isolated authored-rules gate.

## Focused gates

```bash
pnpm run verify:rust
pnpm run verify:rules
pnpm run verify:boundaries
pnpm run verify:ui
pnpm run verify:build
pnpm run verify:browser
```

The headless d20 semantic slice can be iterated without Node:

```bash
cargo test -p rusty-d20 --test d20r0 --locked
cargo clippy -p rusty-d20 --all-targets --locked -- -D warnings
```

The isolated authoring workspace can be checked directly:

```bash
./scripts/verify-rules.sh
pnpm --dir rules run generate:check
cargo test -p rusty-d20 --test d20a0 --locked
```

Regenerate the Rust-owned d20 TypeScript contract and canonical starter
artifacts with:

```bash
pnpm --dir rules run generate
```

The checked artifacts remain usable by
`cargo test -p rusty-d20 --test d20a0 --locked` without installing or running
Node.

Regenerate the Rust-owned TypeScript DTOs with:

```bash
pnpm run protocol:generate
pnpm run protocol:check
```

## Opt-in live evidence

Start the built product through Rust:

```bash
pnpm run serve:local
```

Then use the printed URL in another shell:

```bash
BASE_URL=http://127.0.0.1:PORT LIVE_RUN=1 pnpm run e2e:live
```

Inspect milestone screenshots, visible text, console output, page errors, and
the evidence packet under `test-results/`. The live scenario covers empty/start,
authored preview attribution, optional reaction, deterministic action receipt,
turn advancement, save, classified failure, and desktop/mobile presentation.

The ordinary browser gate also launches an isolated Rust host, saves through
normal controls, stops that process, starts a fresh process against the same
save, verifies the exact projected continuation, and resolves the next
deterministic roll. It also proves preview-only and reacted-pending saves reject
without changing either authoritative in-memory state or the last durable save.
A two-page scenario proves a stale optimistic revision is rejected through a
normal action control.

## Standalone clone

Ordinary verification must not require sibling checkouts. A certification clone
uses only the repository plus public Git/package registries:

```bash
git clone https://github.com/FuzzySlipper/rusty-d20.git
cd rusty-d20
pnpm install --frozen-lockfile
./scripts/verify.sh
```

`verify.sh` installs the separately locked `rules/` workspace before its
focused gate. No sibling Engine checkout is consulted.
