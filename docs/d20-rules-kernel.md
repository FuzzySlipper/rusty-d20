# D20 rules kernel

## Boundary

The kernel is a concrete downstream d20 implementation, not a generic gameplay
language. TypeScript may compose candidate data, but Rust defines the accepted
vocabulary, validates its meaning, compiles immutable definitions, owns live
state, and executes actions. Rusty Engine supplies neutral package,
component-storage, mechanics, and deterministic-randomness mechanisms.

No candidate field contains a callback, expression tree, scheduler command,
event subscription, or executable TypeScript.

The checked TypeScript contract in
`rules/packages/d20-authoring/src/generated.ts` is emitted by the Rust
`rusty-d20-rules-contract` binary. The authoring SDK is therefore a typed
composition surface over the Rust-owned schema rather than a hand-maintained
twin.

## Package and candidate

`D20RulesCandidate` is the strict `schemaVersion: 6` payload of a
`gameplay-rules` package. It contains bounded lists of:

- abilities with score bounds;
- defenses with a base and one or two governing abilities;
- activation budgets with action/reaction timing and bounded initial amounts;
- damage types;
- action resources;
- sealed presentation features with stable identity, label, and description;
- armor definitions and equipment slots;
- implement definitions with slot, tags, attack ability/defense, damage, and
  range;
- temporary/ongoing effects with explicit turn durations and bounded condition
  clauses;
- resource-spending defensive reactions;
- actions with tags, activation costs, participant/cell target shape, team,
  target count, line-of-effect policy, a fixed or implement-bound attack, and
  an optional effect;
- character templates with level, bounded experience, abilities, resources,
  canonical selected features, actions, reactions, and damage affinities;
- storage and concrete item instances with authored containment/equipment;
- encounters with availability, presentation, terminal consequences, and
  rewards;
- adventures that declare product selectability and select their hero, cast,
  storage, items, and encounters.

Package dependencies, canonical bytes, fingerprints, sources, and provenance
come from `gameplay-rules`. Provenance subjects use stable names such as
`ability:might`, `implement:training-blade`, and `action:strike`. Semantic diagnostics
retain the package plus source/line/column correlation when the matching
subject was authored.

Resolved packages may contribute content-only fragments. Compilation rejects
unsupported schema versions, strict-shape failures, quotas, duplicate
definitions, unknown references, duplicate entity identities, incompatible
ownership or reaction/effect pairs, invalid dice/text/list bounds, exact
dependency failures, and package cycles. Direct Rust
candidate admission and canonical artifact decode converge on the same compiled
ruleset and mechanics catalog fingerprints.

The runtime catalog demonstrates exact package dependencies with six
canonical packages:

- `starter-core` contributes six abilities, four defenses, Standard/Bonus/
  Reaction/Movement budgets, four damage types, and three reaction resources;
- `steel-guard` contributes armor, Training Blade and Field Bow implements, a
  defensive reaction, ongoing bleeding, Held/Unsettled conditions, and four
  physical/control actions;
- `ember-ward` contributes Nerve equipment, a focus reaction, a temporary
  ward, and two energy/resolve actions;
- `wardens-gate` contributes its cast, concrete loadout/storage, ordered
  three-encounter sequence, dungeon treasure/door/checkpoint, terminal copy,
  consequences, exactly-once rewards, and default adventure;
- `embers-wake` contributes its distinct cast, Resolve loadout/storage, Ash
  Seer encounter, consequences, reward, and selectable adventure;
- `catalog-probe` contributes a non-default, non-selectable content-only
  adventure over the exact Warden's Gate closure.

Module and definition order are normalized before canonical emission. Moving
definitions between TypeScript helpers or reordering source modules therefore
does not silently change artifact identity; content changes still do.

## Definitions and live state

`D20Ruleset` contains immutable d20 definitions and one admitted Engine
mechanics catalog. Definitions are not live instances and are not embedded in
complete saves.

Live character facts use one canonical `EntityState`:

- `AbilityScoresComponent` stores bounded authored ability scores;
- `ActionResourcesComponent` stores reaction resources;
- `ScheduledEffectsComponent` records the caller-owned expiry turn for active
  Engine effect instances;
- Engine mechanics components store defense stats, vitality, affinities as
  attributed intrinsic sources, active effects, equipment, and items.

Armor and temporary defense effects compile to attributed mechanics sources.
Implements compile to distinct canonical Engine item definitions rather than
being represented as armor. Equipment references in authored item instances
are a strict tagged union, so armor and implements cannot be confused at
runtime.
Resistance and vulnerability compile to exact one-half and two-times damage
responses. Damage receipts therefore retain the source identities that changed
the result.

## Action transaction

An action preview first resolves its fixed attack or its currently equipped
implement. Implement resolution obtains ability, defense, damage, and range
from the compiled implement and rejects an actor without the matching
canonical Engine item assignment. Rust then reads the actor ability modifier,
applies active condition attack penalties or action-tag prohibitions,
evaluates the target defense through `StatService`, reports currently
affordable reactions, and captures exact relevant component revisions plus the
explicit turn and roll position. Actor equipment and scheduled-condition
revisions are included. Its authoritative fields are private; consumers
receive read-only getters and cannot rewrite the action, roll modifier,
resolved damage, defense evaluation, or reaction list before apply.

A reaction spends its resource and applies/schedules its effect in one staged
transaction. The original preview becomes stale, so Rust acquires a fresh
preview and resolves the action in the same product command. Apply obtains the
d20 and damage dice from the configured roll source: either a scoped PRNG
derived from the saved seed and position or the exact next entry in a bounded
static action-roll tape. It then asks `DamageService` to apply typed damage and
applies/schedules any action effect. The staged `EntityState`
publishes only if every late step succeeds. Because authored temporary effects
compile with Engine Refresh stacking, a repeated effect-bearing action refreshes
the existing Engine instance and atomically reschedules its caller-owned expiry
instead of creating a parallel effect or surfacing a stacking conflict.

`advance_turn` is an explicit caller command. It expires due effects and updates
their schedule components atomically. There is no ambient clock or update
callback.

## Persistence

The semantic session save records:

- save schema and exact Engine revision;
- compiled ruleset fingerprint;
- tagged seeded/static roll-source configuration and next position;
- caller-owned current turn;
- the canonical entity snapshot containing Engine and d20 durable components.

Reopen requires the matching ruleset, validates the Engine catalog and d20
component references, verifies active effects have matching schedules, and
reacquires live component revisions. Position-based continuation is constant
time and does not replay prior random draws or static tape entries.

The product runtime wraps that session with its strict schema, authored
adventure and exact composition fingerprint, active/resolved encounter
identities, ordered completed prefix, optimistic revision, next operation/log
identities, and bounded receipt-explanation log.
Pending preview authority is process-local and deliberately excluded. The
browser never receives the `ActionPreview`; only an opposition action with a
real player reaction choice produces a token plus immutable reaction
projection, while Rust retains and applies the actual preview.
