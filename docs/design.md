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

The isolated `rules/` workspace, semantic kernel, and bounded Warden's Gate
adventure implement this path end to end. TypeScript emits checked canonical
packages from a Rust-generated d20 contract; committed artifacts are decoded
and compiled by Rust without Node. The product runtime constructs canonical
Engine-backed entities from those definitions and projects only typed
observations and command inputs to the browser.

## Runtime state

Rust owns authoritative state. `D20Session` contains canonical Engine
`EntityState`, one immutable `D20Ruleset`, explicit turn and deterministic roll
positions, and no ambient scheduler or registry. `GameRuntime` owns the
downstream campaign and encounter lifecycle, optimistic product revision,
opaque pending preview, bounded explanatory log, operation identities, and
complete save wrapper. The durable campaign has explicit `camp`, `encounter`,
and `outcome` phases, with an exact encounter turn owner and typed terminal
result. It is product state for this adventure, not a generic Engine campaign
mechanism. Rusty D20 registers durable ability-score, action-resource, and
scheduled-effect components beside Engine mechanics components.

`D20Session` stages heterogeneous action work in an `EntityState` clone and
publishes only after every d20 and Engine service succeeds. Damage, equipment,
effects, stats, tracks, and attributed sources remain Engine mechanisms.
Ability modifiers, candidate meaning, attack checks, reactions, effect
deadlines, turn advancement, and save policy remain Rusty D20 meaning.

The bounded Iron Warden policy selects from admitted authored actions with the
Rust-owned deterministic session seed, then uses the same opaque
preview/reaction/apply path as a player action. After the opposition resolves,
Rusty D20 advances the caller-owned round and expires due Engine effect
instances before publishing the next player turn. Authoritative vitality
selects victory or defeat exactly once. Victory unequips and transfers the
authored reward through Engine equipment and inventory services; defeat leaves
inventory untouched and returning to camp restores a bounded vitality amount
through the Engine track service. None of this policy is promoted into an
Engine scheduler, AI graph, or event bus.

The camp loadout uses the same canonical `EntityState`: characters and the camp
stash carry `InventoryComponent`, unique armor entities use containment plus
`ItemComponent`, and `EquipmentComponent` is the only assignment authority.
Rusty D20 owns the fixed starter items, slot meaning, camp-only command policy,
inventory presentation order, and product revision. Engine services own
capacity, containment, equipment, prospective track validation, and attributed
stat-source mutation. The browser receives immutable loadout and defense
readouts and never maintains a shadow inventory.

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

`rusty-d20-host` serves the Angular build plus read-only session projection and
typed new-adventure, loadout equip/unequip/transfer, enter-encounter, preview,
reaction, action, begin-opposition, return-to-camp, and save commands from one
origin. Rust DTOs generate `libs/protocol/src/generated/api-types.ts`. The
protocol layer strictly decodes unknown JSON with collection bounds; transport
preserves typed HTTP rejection; domain projects a view; store owns async UI
state and rejects late responses by request generation; features render it.
Continue is a local presentation choice over an already loaded Rust
projection; it does not mutate or duplicate campaign authority.

The combat log is a bounded receipt explanation. It observes committed facts
and is not a second authority, persistence replay mechanism, or command source.

## Dependencies

Rusty Engine packages use exact public Git revisions. There is no ordinary
sibling checkout dependency. Angular libraries follow the retained Nx boundary
graph in `boundaries.json`; production code cannot import testing fixtures.

## Persistence and execution

`D20Session` saves the exact Engine revision, ruleset fingerprint, explicit RNG
seed/roll position, caller-owned turn, and canonical entity snapshot. Product
session save schema 2 includes the catalog-v2 inventory/equipment state.
Product save schema 4 wraps it with strict campaign identity, phase, encounter
turn owner, terminal outcome, product revision, next operation/log identities,
and the bounded explanatory log. Product schemas 1 through 3 migrate
deterministically: the old mechanics catalog is upgraded, the fixed starter
loadout is installed without replay where required, and older active
encounters resume at the player decision boundary. Schema 1 establishes the
active Iron Warden encounter. Unknown schemas, partial loadouts, and
inconsistent phase/turn/outcome pairs reject rather than defaulting or
discarding state.
Opaque previews are intentionally not durable, so save rejects before file
mutation while an action is pending; the user must resolve it first. This
includes a pending action whose reaction has already committed resource and
effect changes. Compiled definitions are not copied into live saves. Reopen
requires the matching immutable ruleset, reconstructs registered components,
validates mechanics, d20 references, product loadout and reward identities,
reacquires non-durable component revisions, and continues the exact camp,
encounter, or outcome phase, turn owner, loadout, and deterministic rolls
without replay.

Round advancement is an explicit downstream consequence of resolving the
opposition action and expires recorded effect instances atomically before the
next player decision. There is no clock callback, tick subscription, event
bus, or persisted closure.
