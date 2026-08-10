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
landmark, trigger-driven encounter entry, automatic Rust-owned action and
opposition resolution without a turn-acknowledgement control, optional player
reactions, one player/opposition round, save, and desktop/mobile presentation.

The ordinary real-host smoke additionally requires exactly one
`aui-game-viewport` canvas while the scene mode changes from catalog to camp,
exploration, and encounter. Its artifacts include the full-window catalog,
camp, desktop/mobile exploration party sheets, all-facing corridor, mobile corridor, and desktop/mobile rendered
encounter HUD states. Renderer frame tests separately prove that abstract phase
backdrops have no gameplay source identities, exploration still delegates to
the occlusion-safe dungeon adapter, the public Engine camera sampler is bounded
to accepted adjacent projections with latest-wins interruption, reduced motion
settles synchronously, and resize/disposal preserve the single-frame lifecycle.
The browser records intermediate Engine camera poses for every facing, normal
steps, rapid interruption, narrow layout, rejected collision, and reduced
motion without observing topology changes. Encounter/outcome mapping preserves
stable cell/entity pick handles, route geometry, state markers, lifecycle
replacement, and responsive overhead camera fitting. The browser smoke uses
the public Engine surface picker plus the focusable canvas keyboard cursor to
prove a rendered cell still issues the normal Rust-owned tactical movement
command.

The ordinary browser gate also launches an isolated Rust host, saves through
normal loadout controls, stops that process, starts a fresh process against the
same save, verifies exact camp loadout and encounter continuation, and resolves
the next configured roll automatically. The desktop/mobile smoke moves
equipment to the 24-slot shared inventory and back into an authored slot with
native drag/drop, repeats unequip/equip through keyboard and click activation,
and proves visible capacity rejection without mutation. It opens the
exploration inventory without changing phase, checks its read-only controls and
focus restoration, and directly proves Rust rejects a forged exploration-time
placement without changing the snapshot. The fresh-process case uses the
click/touch-compatible preparation path, reopens that exact loadout, and also
proves a non-durable reaction prompt rejects saving without changing either
authoritative in-memory state or the last durable save.
The same exploration run opens the native modal Party surface, proves it blocks
movement input, switches character tabs with the keyboard, inspects level/XP,
abilities, Engine-attributed defenses, features, actions, reactions, affinities,
effects, and loadout, checks mobile containment, restores trigger focus, and
reopens the same Rust-owned facts after a save and fresh page load.
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

## Renderer-first campaign certification

Run the complete browser certification without accepting any Nx build or test
cache:

```bash
NX_SKIP_NX_CACHE=true E2E_PORT=4392 pnpm run verify:browser
```

The single fresh Rust-host run covers adventure selection, native drag/drop and
keyboard/click preparation, save/fresh-process reopen, exploration inventory,
all-facing Engine-sampled camera movement, trigger-driven encounter entry,
action-first pointer/touch/keyboard target picking, explicit hotbar movement
with keyboard cancellation and two-click route preview/confirmation, reactions,
automatic bounded opposition turns, tactical movement, victory, defeat
recovery, continued expedition, terminal completion, guarded
reset/malformed-save recovery, and classified stale,
transport, and protocol failures. The movement scenario proves an ordinary
board pick does not mutate, the preview retains the exact Rust-projected route
without changing the host session, and only a repeated destination issues the
typed command. It requires one permanent Engine canvas across scene changes and
uses independent saved-process scenarios where persistence or recovery is the
behavior under test.
The combat scenarios assert there is no Begin-turn control or opposition
transport command: a player action or End Activation settles directly to the
next party activation, reaction prompt, or outcome. Isolated-host coverage also
exercises reaction resume, condition-filtered and no-legal-action opposition
progression, rollback on roll-source exhaustion, schema-11 party-boundary
reopen, victory/defeat, and desktop/mobile rendering. The primary desktop and
touch scenarios manually move the rules log away from its end, resolve a new
action, and require it to follow the new stable entry. They then inspect the
same Rust-authored d20, modifier, defense, roll-source, damage, and effect
details through the row's hover/focus/touch disclosure.

Inspect the named Playwright attachments at 1280 by 720 and 390 by 844. The
representative set is `renderer-root-camp-desktop.png`,
`drag-loadout-preparation.png`, `exploration-inventory-overlay.png`,
`engine-dungeon-corridor-mobile.png`,
`renderer-root-encounter-desktop.png`,
`renderer-root-encounter-mobile.png`,
`movement-preview-desktop.png`, `movement-preview-mobile.png`,
`action-first-targeting-mobile-touch.png`, `mobile-defeat.png`,
`warden-adventure-complete.png`, and `malformed-save-recovery.png`. The narrow
encounter assertions additionally prove that the action and log regions stay
at the bottom edge without intersecting each other or the status region,
neither escapes the viewport, combat-log content has no horizontal overflow,
and every narrow action control has at least a 44-pixel hit height. Existing
focus restoration/navigation assertions, document-width checks, pointer
pass-through probes, renderer resize observations, and camera disposal tests
cover focus order, safe gaps, horizontal overflow, resize, and teardown.

## Adjacent checkout prerequisite

Ordinary verification requires Rusty Engine at the adjacent path selected by
`Cargo.toml` and the Rules package links. A fresh workspace provisions both
repositories as siblings; D20 then consumes the Engine checkout exactly as-is:

```bash
mkdir rusty-d20-workspace
cd rusty-d20-workspace
git clone https://github.com/FuzzySlipper/rusty-engine.git
git clone https://github.com/FuzzySlipper/rusty-d20.git
cd rusty-d20
pnpm install --frozen-lockfile
./scripts/verify.sh
```

`verify.sh` installs the separately locked `rules/` workspace before its
focused gate. It does not fetch, update, or mutate the adjacent Engine checkout;
CI provisions that sibling ephemerally before running the same focused gates.
