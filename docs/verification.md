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
the evidence packet under `test-results/`. The opt-in live scenario covers
empty/new adventure, the durable camp, first-person grid movement, an authored
landmark, trigger-driven encounter entry, automatic Rust-owned action
resolution, optional player reactions, one player/opposition round, save, and
desktop/mobile presentation.

The ordinary browser gate also launches an isolated Rust host, saves through
normal loadout controls, stops that process, starts a fresh process against the
same save, verifies exact camp loadout and encounter continuation, and resolves
the next configured roll automatically. The desktop/mobile smoke covers keyboard and
click activation plus visible capacity rejection without mutation. The
fresh-process case also proves a non-durable reaction prompt rejects saving
without changing either authoritative in-memory state or the last durable save.
A two-page scenario proves a stale optimistic revision is rejected through a
normal action control. Dedicated real-host scenarios complete Warden victory
and defeat plus an Ember victory through ordinary controls. The Ember scenario
starts on mobile, inspects distinct Resolve/Focus/Fire content and attribution,
reopens its pre-encounter camp, traverses the reliquary grid to the Ash Seer,
completes the encounter, reopens the terminal outcome, returns to camp, and
reopens the exactly-once reward. Those
terminal scenarios are part of `verify:browser`, not a non-idempotent
requirement on an arbitrary already-running live save.
The Warden scenario enters through the pass grid, continues from its
exactly-once first reward at the exact trigger cell, claims the sigil-buckler
treasure, activates the Warden refuge checkpoint, returns safely to camp,
reopens the treasure-gated door, completes the Seal Guard and final redoubt,
renders the authored terminal ending, and reopens the exact three-entry
completion history. The focused Rust save regressions also reject forged event
IDs, unmet door prerequisites, contradictory treasure ownership, and premature
terminal state.
A separate real-host scenario displays the exact save identity, proves reset
cancel/confirm behavior, reopens a replacement Ember campaign, then starts from
a deliberately malformed save and recovers through the typed discard path
without page or console errors.

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
