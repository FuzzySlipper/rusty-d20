# Rusty D20

Rusty D20 is a concrete d20 RPG and interactive reference consumer for
[Rusty Engine](https://github.com/FuzzySlipper/rusty-engine). It owns d20 rules,
game orchestration, complete saves, transport, and presentation. Rusty Engine
remains optional reusable infrastructure; other games never need to import this
repository.

The current product is a deliberately bounded but real adventure shell. An
Angular/Nx application is served by a Rust process and reaches Rust-owned state
through a strict generated same-origin contract. The browser can choose either
the Steel-oriented **The Warden's Gate** or Ember-oriented **Ember's Wake**,
save in camp, enter authored encounters, manage Engine-backed
inventory/equipment and camp storage, inspect every attributed defense, choose
authored actions and targets, apply optional reactions, resolve deterministic
source-attributed damage/effects, face an explicit deterministic opposition
turn, and play through victory or defeat. Warden's Gate is an ordered
two-encounter campaign; Ember's Wake is a distinct single-encounter path.
Victory transfers the path's
canonical reward into camp storage; defeat applies its authored bounded
recovery. A fresh Rust process continues the exact selected composition,
campaign phase, turn owner, outcome, loadout, and authoritative state without
replay. The shell shows the configured save identity, offers an explicit
destructive reset with identity/revision guards, and remains usable when a
malformed save needs to be discarded.
Characters, loadouts, storage, encounter presentation, outcomes, and rewards
are defined in multi-file TypeScript authoring modules and compiled by Rust
from checked canonical artifacts; the running host does not need Node. Fake
transport remains available only from `libs/testing-fixtures`.

## Start the product

```bash
pnpm install --frozen-lockfile
pnpm run serve:local
```

Open the printed `BASE_URL`. With no prior save, choose **The Warden's Gate**
or **Ember's Wake**, inspect its camp and loadout, then enter the offered
encounter. The host writes an explicit save to
`target/rusty-d20/save.json`; after restart, **Continue Adventure** resumes the
exact selected path, camp or encounter phase, and canonical loadout. Resolve
any pending action before saving; the Rust host rejects pending saves before
changing the existing save file. **Reset / New Adventure** names the exact
save and live revision before deleting it. A malformed save starts a typed
recovery screen rather than terminating the host.

For a managed LAN-visible instance, use:

```bash
den-serve up rusty-d20 -repo /absolute/path/to/rusty-d20
```

## Verify

```bash
./scripts/verify.sh
```

Focused commands and live-evidence instructions are in
[docs/verification.md](docs/verification.md). Architecture and source routing
start at [docs/design.md](docs/design.md) and
[docs/agent-code-atlas.md](docs/agent-code-atlas.md).

## Current phase boundary

The GM7 reference slice and D20G1 campaign shell are connected end to end. It
is not yet a broad d20 product: initiative order beyond each
bounded two-combatant encounter, movement, spellcasting, advancement, content
publication, broader item/content catalogs, and branching campaign graphs
remain later milestones. See
[docs/known-limitations.md](docs/known-limitations.md).

## Provenance

The UI was copied from `FuzzySlipper/rusty-engine-ui` at exact reviewed commit
`68ddfa5430ec3bc2cf7ca96963982db9511e79ba`. Rusty Engine crates are pinned to
exact reviewed commit `fb608e323a8b44a55195f5720101224ff37fd5db` with public
Git dependencies. See [docs/source-provenance.md](docs/source-provenance.md).
