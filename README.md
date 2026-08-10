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
save in camp, enter a first-person dungeon, move and turn on a Rust-owned grid,
inspect landmarks, reveal only visited cells on the automap, and enter combat
only by reaching a hidden authored trigger. Combat shifts to the existing modal
screen, where the player manages Engine-backed inventory/equipment and camp
storage, inspects attributed defenses, chooses authored actions and targets,
receives optional player reaction choices, and sees Rust automatically resolve
source-attributed rolls, damage, and effects under a configured seeded or
static roll source. It faces an explicit opposition turn and plays through victory or
defeat. Warden's Gate is a complete bounded three-encounter expedition with a
four-character party, inspectable landmarks, a claimed sigil treasure, a
treasure-gated door, a durable safe-return checkpoint, and an authored terminal
victory or defeat. Ember's Wake remains a distinct single-encounter path.
Victory transfers the path's
canonical reward into camp storage; defeat applies its authored bounded
recovery. A fresh Rust process continues the exact selected composition,
campaign phase, turn owner, outcome, loadout, and authoritative state without
replay. The shell shows the configured save identity, offers an explicit
destructive reset with identity/revision guards, and remains usable when a
malformed save needs to be discarded.
Characters, loadouts, storage, dungeon topology/placements, encounter
presentation, outcomes, and rewards are defined in multi-file TypeScript
authoring modules and compiled by Rust from checked canonical artifacts; the
running host does not need Node. Fake transport remains available only from
`libs/testing-fixtures`.

## Start the product

```bash
pnpm install --frozen-lockfile
pnpm run serve:local
```

Open the printed `BASE_URL`. With no prior save, choose **The Warden's Gate**
or **Ember's Wake**, inspect its camp and loadout, enter the dungeon, and use
the movement pad or arrow/WASD keys to find its encounters. The browser cannot
name or start an encounter directly. The host writes an explicit save to
`target/rusty-d20/save.json`; after restart, **Continue Adventure** resumes the
exact selected path, camp, dungeon cell and facing, encounter/outcome phase,
and canonical loadout. Choose or decline any pending reaction before saving;
the Rust host rejects that non-durable reaction prompt before changing the
existing save file. **Reset / New
Adventure** names the exact save and live revision before deleting it. A
malformed save starts a typed recovery screen rather than terminating the host.

The default roll source is a seeded scoped PRNG. For exact authored results,
pass `--roll-source path/to/roll-source.json` to `rusty-d20-host`; the JSON is
either `{"kind":"seeded","seed":220209190}` or
`{"kind":"static","rolls":[{"d20":13,"damage":[8]}]}`. The selected source and
position persist in the save and must match when reopening it.

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
[docs/agent-code-atlas.md](docs/agent-code-atlas.md). The adjacent Engine
workspace contract is in
[docs/adjacent-engine-dependency.md](docs/adjacent-engine-dependency.md).

## Current phase boundary

The first bounded Ruleweaver-derived Gold Box adventure is connected end to
end. It is not a sprawling campaign: spellcasting, advancement, content
publication, broader item/content catalogs, multiple dungeon floors, and
branching campaign graphs remain later milestones. See
[docs/known-limitations.md](docs/known-limitations.md).

## Provenance

The UI was copied from `FuzzySlipper/rusty-engine-ui` at exact reviewed commit
`68ddfa5430ec3bc2cf7ca96963982db9511e79ba`. Current builds consume the
adjacent Rusty Engine checkout through one Rust facade path plus two build-time
Rules package links; Engine revision identity is not a D20 runtime or save fact.
The reviewed Ruleweaver and Asha D20 Fantasy references, including the bounded
Crosswind role-shape adaptation used by Warden's Gate, are recorded in
[docs/source-provenance.md](docs/source-provenance.md).
