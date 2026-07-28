# Verification

## Aggregate

```bash
./scripts/verify.sh
```

This runs strict Rust, protocol/boundary, TypeScript, production build, and
real-Rust-host browser checks.

## Focused gates

```bash
pnpm run verify:rust
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
the evidence packet under `test-results/`. The bootstrap live claim is limited
to the Rust readout and visible classified failure.

## Standalone clone

Ordinary verification must not require sibling checkouts. A certification clone
uses only the repository plus public Git/package registries:

```bash
git clone https://github.com/FuzzySlipper/rusty-d20.git
cd rusty-d20
pnpm install --frozen-lockfile
./scripts/verify.sh
```
