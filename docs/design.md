# Rusty D20 design

## Authority

Rusty D20 is one concrete d20 game. It owns game meaning and orchestration;
Rusty Engine owns reusable mechanisms. Neither Rusty Engine nor another game
depends on Rusty D20.

The intended durable path is:

```text
TypeScript authoring source
  -> immutable bounded d20 candidate artifact
  -> Rust d20 semantic compiler and admitted definitions
  -> registered entity components plus named Engine services
  -> Rusty D20 orchestration, persistence, and projection
  -> strict same-origin transport
  -> Angular store, features, and presentation
```

The isolated `rules/` workspace and semantic kernel now implement the
authoring-to-Rust portion of this path. TypeScript emits checked canonical
packages from a Rust-generated d20 contract; committed artifacts are decoded
and compiled by headless Rust without Node. The product host still exposes only
its bootstrap readout; connecting gameplay operations and projections is a
later UI milestone.

## Runtime state

Rust owns authoritative state. `D20Session` contains canonical Engine
`EntityState`, one immutable `D20Ruleset`, explicit turn and deterministic roll
positions, and no ambient scheduler or registry. Rusty D20 registers durable
ability-score, action-resource, and scheduled-effect components beside Engine
mechanics components. `GameRuntime` remains the smaller host bootstrap owner
until product operations are connected.

`D20Session` stages heterogeneous action work in an `EntityState` clone and
publishes only after every d20 and Engine service succeeds. Damage, equipment,
effects, stats, tracks, and attributed sources remain Engine mechanisms.
Ability modifiers, candidate meaning, attack checks, reactions, effect
deadlines, turn advancement, and save policy remain Rusty D20 meaning.

Preview records exact relevant component revisions. Applying a reaction changes
resource and effect components, so callers must acquire a fresh action preview.
Unrelated entity changes do not invalidate a preview.

TypeScript does not host live rules, callbacks, runtime sessions, or gameplay
state. The authoring SDK runs ordinary functions, tables, and loops only at
build time, produces immutable candidates, and delegates strict package
admission and all d20 meaning to Rust.

The candidate and compiled-definition contract is documented in
[the d20 rules kernel](d20-rules-kernel.md).

## Rules authoring

`rules/packages/d20-authoring` consumes the two neutral Engine authoring
packages from the exact reviewed Engine Git revision. Its d20 candidate types
and limits are generated from Rust. It provides source-aware definition
builders, module composition, deterministic definition ordering, exact package
dependencies, and canonical artifact emission.

`rules/packages/starter-ruleset` owns concrete content. The checked starter
artifacts contain a shared core plus distinct steel/armor and ember/resolve
compositions. These packages are build-time inputs, not UI or runtime
dependencies. See [rules authoring](rules-authoring.md).

## Transport and protocol

`rusty-d20-host` serves the Angular build and `/api/v1/readout` from one origin.
Rust DTOs generate `libs/protocol/src/generated/api-types.ts`. The protocol
layer strictly decodes unknown JSON; transport classifies HTTP/network failure;
domain projects a view; store owns async UI state; features render it.

The diagnostic readout observes authoritative facts. It is not a second
authority, persistence format, or replay log.

## Dependencies

Rusty Engine packages use exact public Git revisions. There is no ordinary
sibling checkout dependency. Angular libraries follow the retained Nx boundary
graph in `boundaries.json`; production code cannot import testing fixtures.

## Persistence and execution

`D20Session` saves the exact Engine revision, ruleset fingerprint, explicit RNG
seed/roll position, caller-owned turn, and canonical entity snapshot. Compiled
definitions are not copied into live saves. Reopen requires the matching
immutable ruleset, reconstructs registered components, validates mechanics and
d20 references, and reacquires non-durable component revisions.

`advance_turn` is an explicit downstream command that expires recorded effect
instances atomically. There is no clock callback, tick subscription, event bus,
or persisted closure.
