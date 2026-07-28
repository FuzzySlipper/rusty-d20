# D20 rules kernel

## Boundary

The kernel is a concrete downstream d20 implementation, not a generic gameplay
language. TypeScript may compose candidate data, but Rust defines the accepted
vocabulary, validates its meaning, compiles immutable definitions, owns live
state, and executes actions. Rusty Engine supplies neutral package,
component-storage, mechanics, and deterministic-randomness mechanisms.

No candidate field contains a callback, expression tree, scheduler command,
event subscription, or executable TypeScript.

## Package and candidate

`D20RulesCandidate` is the strict `schemaVersion: 1` payload of a
`gameplay-rules` package. It contains bounded lists of:

- abilities with score bounds;
- defenses with a base and ability-derived modifier;
- damage types;
- action resources;
- armor definitions and equipment slots;
- temporary/ongoing effects with explicit turn durations;
- resource-spending defensive reactions;
- attack/check actions with fixed dice damage and an optional effect.

Package dependencies, canonical bytes, fingerprints, sources, and provenance
come from `gameplay-rules`. Provenance subjects use stable names such as
`ability:strength`, `armor:chain`, and `action:strike`. Semantic diagnostics
retain the package plus source/line/column correlation when the matching
subject was authored.

Resolved packages may contribute content-only fragments. Compilation rejects
unsupported schema versions, strict-shape failures, quotas, duplicate
definitions, unknown references, incompatible reaction/effect pairs, invalid
dice or bounds, exact dependency failures, and package cycles. Direct Rust
candidate admission and canonical artifact decode converge on the same compiled
ruleset and mechanics catalog fingerprints.

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
Resistance and vulnerability compile to exact one-half and two-times damage
responses. Damage receipts therefore retain the source identities that changed
the result.

## Action transaction

An action preview reads the actor ability modifier, evaluates the target
defense through `StatService`, reports currently affordable reactions, and
captures exact relevant component revisions plus the explicit turn and roll
position. Its authoritative fields are private; consumers receive read-only
getters and cannot rewrite the action, roll modifier, defense evaluation, or
reaction list before apply.

A reaction spends its resource and applies/schedules its effect in one staged
transaction. The original preview becomes stale; the caller must preview
again. A fresh apply derives a deterministic scoped stream from the saved seed
and roll index, resolves the d20 check, asks `DamageService` to apply typed
damage, and applies/schedules any action effect. The staged `EntityState`
publishes only if every late step succeeds.

`advance_turn` is an explicit caller command. It expires due effects and updates
their schedule components atomically. There is no ambient clock or update
callback.

## Persistence

The complete save records:

- save schema and exact Engine revision;
- compiled ruleset fingerprint;
- deterministic seed and next roll index;
- caller-owned current turn;
- the canonical entity snapshot containing Engine and d20 durable components.

Reopen requires the matching ruleset, validates the Engine catalog and d20
component references, verifies active effects have matching schedules, and
reacquires live component revisions. Roll-index scoping makes continuation
constant-time rather than replaying every prior random draw.
