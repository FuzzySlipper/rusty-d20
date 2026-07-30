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

The isolated `rules/` workspace, semantic kernel, and bounded Warden's Gate and
Ember's Wake adventures implement this path end to end. TypeScript emits an
exact package catalog from a Rust-generated d20 contract; committed canonical
artifacts are decoded and compiled by Rust without Node. Rust projects the
selectable catalog entries, validates one optimistic selection before
mutation, and admits only that adventure's immutable dependency closure. The
closure supplies character templates, item instances, storage, encounters,
authored dungeon topology and placements, presentation, availability, outcomes,
and rewards. The product runtime
constructs canonical Engine-backed entities from those definitions and
projects only typed observations and command inputs to the browser. A running
campaign cannot switch compositions.

## Runtime state

Rust owns authoritative state. `D20Session` contains canonical Engine
`EntityState`, one immutable `D20Ruleset`, an explicit turn and roll-source
position, and no ambient scheduler or registry. Its configured roll source is
either a seeded scoped PRNG or a bounded authored static action-roll tape.
`GameRuntime` owns the downstream campaign and encounter lifecycle, optimistic
product revision, opaque reaction prompt, bounded explanatory log, operation
identities, and complete save wrapper. The durable campaign has explicit `camp`, `exploration`,
`encounter`, `outcome`, and `adventure-complete` phases, with an exact dungeon
position and facing, discovered-cell, inspected-landmark, opened-door,
collected-treasure, and active-checkpoint facts, encounter turn owner, and typed
terminal result. It is product state for this adventure, not a generic Engine
campaign mechanism. Rusty D20 registers durable ability-score,
action-resource, and scheduled-effect components beside Engine mechanics
components.
The campaign retains the active and last resolved encounter identity plus the
ordered completed-encounter prefix. Only the next incomplete encounter in the
adventure's authored list is offered. Reusing an opponent in a later encounter
restores only its bounded vitality through the Engine track service; resources,
effects, equipment, rewards, and other prior facts remain authoritative. This
is explicit Rusty D20 campaign policy, not a generic quest graph.

The authored dungeon is a bounded, enclosed ASCII grid compiled by Rust.
Semantic admission rejects malformed or excessive topology, blocked or
overlapping placements, invalid starts/checkpoints/door edges, unreachable
content or circular treasure-door prerequisites, duplicate event identities,
and an encounter placement sequence that disagrees with the adventure. Rust
alone resolves turns, steps, collisions, landmarks, treasure transfer, door
opening, discoveries, safe checkpoint return, and the encounter trigger at the
reached cell. The browser receives a bounded three-depth first-person wall
projection, movement availability, the current inspectable event, the door
directly ahead, and visited cells; it does not receive the complete wall grid
or trigger coordinates. Compass and minimap are presentation over this
projection, not a second navigation authority. Completed encounter identities consume their
authored trigger cells, so later traversal can cross them while the next
unconsumed trigger still follows ordered admission. The fixed three-depth view
emits neutral all-wall records after the first opaque front wall; the strict
browser decoder rejects any non-neutral topology behind that occluder.

`D20Session` stages heterogeneous action work in an `EntityState` clone and
publishes only after every d20 and Engine service succeeds. Damage, equipment,
effects, stats, tracks, and attributed sources remain Engine mechanisms.
Ability modifiers, candidate meaning, attack checks, reactions, effect
deadlines, turn advancement, and save policy remain Rusty D20 meaning.
The admitted Ruleweaver foundation defines six attributes and four defenses;
Wits and Nerve select the better modifier from their two authored governing
attributes. Actions carry bounded tags, activation costs, target shape, range,
line-of-effect policy, and either a fixed attack or an implement binding.
Fixed attacks compile all roll facts directly. Implement-bound attacks resolve
ability, defense, damage, and range from the required implement definition and
require a matching canonical equipped Engine item at preview time.

Scheduled effects may contribute bounded downstream condition clauses. The
tactical encounter enforces action-tag prohibitions, attack penalties, and
movement prohibition in Rust. TypeScript does not execute a condition
predicate.

Each encounter authors one bounded ASCII tactical board and a unique starting
placement for every admitted participant. Rust adapts that board into direct
Engine volume, spatial, pathfinding, and collision services. It owns canonical
positions and occupancy, per-activation movement budgets, legal destination
routes, range and line-of-effect admission, deterministic opposition movement,
and bounded forced movement. The browser receives only the immutable board,
participant coordinates, legal routes, targets, and receipts needed to render
the overhead view and issue typed commands.

The bounded opposition policy selects from the active encounter participant's
legal admitted actions using the configured Rust-owned roll source. Party
actions and opposition actions without player reactions resolve atomically in
the command that selected them. When an opposition action offers a player
reaction, Rust exposes only that gameplay decision; choosing or declining it
immediately resolves the action and roll. After the opposition resolves, Rusty
D20 advances the caller-owned round and expires due
Engine effect instances before publishing the next player turn. Authoritative
vitality selects victory or defeat exactly once. Victory unequips and
transfers the active encounter's authored reward through Engine equipment and
inventory services; defeat leaves inventory untouched and returning to camp
restores its authored bounded vitality amount through the Engine track
service. None of this policy is promoted into an Engine scheduler, AI graph,
or event bus.

The camp loadout uses the same canonical `EntityState`: characters and the camp
stash carry `InventoryComponent`, unique armor and implement entities use
containment plus `ItemComponent`, and `EquipmentComponent` is the only
assignment authority.
Rusty D20 owns the authored item selection, slot meaning, camp-only command
policy, inventory presentation order, and product revision. Engine services own
capacity, containment, equipment, prospective track validation, and attributed
stat-source mutation. The browser receives immutable loadout and defense
readouts and never maintains a shadow inventory.

Preview records exact relevant component revisions, including actor equipment
and scheduled conditions. Applying a reaction changes resource and effect
components, so callers must acquire a fresh action preview. Unequipping the
required implement or changing an active actor condition likewise makes an
existing preview stale. Unrelated entity changes do not invalidate a preview.

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
artifacts contain a shared Ruleweaver foundation, distinct steel/armor and
ember/Nerve rule
packages, the multi-file Warden's Gate and Ember's Wake adventures, and a
non-selectable content-only catalog probe. `catalog.json` embeds canonical
package bytes; Rust selects only the exact dependency closure that owns the
requested adventure. Every adventure authors a bounded dungeon, start
checkpoint, ordered encounter triggers, landmarks, doors, treasures,
safe-return checkpoints, and terminal copy beside its existing cast and
encounters. These packages are build-time inputs, not UI or runtime
dependencies. See
[rules authoring](rules-authoring.md).

## Transport and protocol

`rusty-d20-host` serves the Angular build plus read-only session projection and
typed adventure selection, loadout equip/unequip/transfer, begin-exploration,
exploration-command, tactical-move, action, reaction/decline-reaction,
begin-opposition, continue-after-outcome, and save commands from one origin.
There is no
browser-facing command that names an encounter; reaching an authored dungeon
trigger is the only product transport path into combat. The host also exposes
a Rust-generated save-status contract and an
identity/revision/adventure-guarded destructive reset. A malformed save keeps
the host alive in a recovery-only state; ordinary session commands fail closed
until the exact save is discarded. Rust DTOs generate
`libs/protocol/src/generated/api-types.ts`. The protocol layer strictly decodes
unknown JSON with collection bounds; transport preserves typed HTTP rejection;
domain projects a view; store owns async UI state and rejects late responses
by request generation; features render it. Continue is a local presentation
choice over an already loaded Rust projection; it does not mutate or duplicate
campaign authority.

The combat log is a bounded receipt explanation. It observes committed facts
and is not a second authority, persistence replay mechanism, or command source.

## Dependencies

`engine-source.json` selects one exact public Rusty Engine Git revision.
`scripts/engine-revision` transactionally keeps every Rust crate, rules
package, build policy, and lockfile at that revision; runtime provenance and
boundary checks derive from the manifest. There is no ordinary sibling
checkout dependency. Angular libraries follow the retained Nx boundary graph
in `boundaries.json`; production code cannot import testing fixtures.

## Persistence and execution

`D20Session` saves the exact Engine revision, ruleset fingerprint, complete
tagged roll-source configuration and position, caller-owned turn, and canonical
entity snapshot. Session save schema 5 includes the catalog-v2 inventory/equipment state together with
the registered party roster, encounter participation facts, and per-character
activation budgets and canonical tactical positions. Product save schema 10
wraps it with the authored adventure identity, exact composition fingerprint,
phase, dungeon position/facing/discovery/inspection state, opened doors,
collected treasures, active checkpoint, active and resolved encounter
identities, ordered completed-encounter history, encounter turn owner, terminal
adventure result, product revision, next operation/log identities, and the
bounded explanatory log.

Product schemas 1 through 9 and session schemas before 5 are rejected rather
than migrated. Unknown schemas, partial loadouts, missing or extra registered
party/participation/budget facts, unknown budget identities, above-initial
budgets, inconsistent phase/turn/outcome pairs, unreachable discoveries,
unknown event IDs, unmet door prerequisites, and treasure ownership that
contradicts the collected-event set also reject rather than defaulting or
discarding state. New saves never infer a missing adventure, composition,
roster, action economy, or exploration event.
Opaque reaction prompts are intentionally not durable, so save rejects before
file mutation while a player reaction decision is pending. Choosing or
declining the reaction resolves the roll atomically, leaving no reacted-pending
state. Compiled definitions are not copied into live saves. Reopen
resolves the saved adventure from the embedded catalog, requires its matching
immutable package closure and fingerprint, reconstructs registered components,
validates mechanics, d20 references, product loadout and reward identities,
requires the exact authored budget identity set and canonical participation
roster, cross-checks encounter phase and outcome against the vitality of that
encounter's party and opposition participants, and separately requires at
least one living whole-party member in camp or exploration. It reacquires
non-durable component revisions and continues the exact camp, exploration,
encounter, outcome, or terminal adventure phase, exact dungeon progress, turn
owner, loadout, roll-source configuration, and position without replay.

File layout and storage policy remain host-owned. The browser observes the
configured save identity but never chooses an arbitrary path. Reset validates
that identity together with the current campaign and revision before deleting
the file or replacing live state; stale requests and file failures leave both
unchanged. The host accepts an optional `--roll-source` JSON file. Its tagged
configuration must match an existing save, and guarded reset retains the
configured source for the next adventure.

Round advancement is an explicit downstream consequence of resolving the
opposition action and expires recorded effect instances atomically before the
next player decision. There is no clock callback, tick subscription, event
bus, or persisted closure.
