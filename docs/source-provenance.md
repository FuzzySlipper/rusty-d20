# Source provenance

## Rusty Engine

`Cargo.toml` imports the complete public `rusty-engine` facade from branch
`main`; `engine-source.json` and `Cargo.lock` record its resolved public commit
and full transitive Engine crate closure. The updater described in
[engine-revision-updates.md](engine-revision-updates.md) advances that lock
transactionally. There is no sibling path fallback or downstream crate menu.

The isolated TypeScript authoring workspace pins these public Engine packages
to the same revision and exact repository subpaths:

- `@rusty-engine/gameplay-rules-contracts`
- `@rusty-engine/gameplay-rules-authoring`

`rules/pnpm-lock.yaml` records the codeload revision and subpath identities.
Only those exact git package prepare scripts are allowed by
`rules/pnpm-workspace.yaml`.

The product browser workspace has no Engine package dependency. Rust owns D20
frame construction and uses the facade's native webview adapter; Engine alone
owns the Rust-to-TypeScript renderer boundary, Three scene, camera, input,
render loop, resource lifecycle, and disposal.

## Rusty Engine UI donor

- Repository: `FuzzySlipper/rusty-engine-ui`
- Commit: `68ddfa5430ec3bc2cf7ca96963982db9511e79ba`
- Commit date: `2026-07-24T02:09:01-07:00`
- Copy date: `2026-07-28`
- Method: exact tracked-file archive followed by the donor initializer

The copy owns every retained file.

### Retained

- Angular/Nx workspace, generated boundary discipline, theme, artifact
  collector, live-test gate, and package-root structure.
- Product-neutral components and character-status, combat-log, compass,
  equipment, hotbar, inventory, and minimap widgets.
- Protocol, platform, transport, domain, store, renderer/components, feature,
  shell, theme, and testing-fixture layer identities.

### Adapted

- Package imports changed from `@rusty-engine-ui/*` to `@rusty-d20/*`.
- Transport may depend on the platform HTTP port in addition to protocol.
- Main-menu feature became the permanent Rust-owned landing, camp, and
  encounter screen.
- Playwright and local serving now start the real Rust host.
- CI and aggregate verification now cover both Rust and TypeScript.

### Replaced

- Fake template transport -> same-origin Rust HTTP transport.
- Handwritten template status DTO -> Rust-generated readout DTO.
- Demo configuration and placeholder menu behavior -> Rust-owned readout.
- Angular-only static/dev server product proof -> Rust host serving Angular.
- Donor protocol freshness script -> Rust protocol generator/check.

### Excluded

- Donor `demo-config`, `PlaceholderActions`, and UI-only event log store.
- Fixture-driven game-HUD and inventory feature screens and their demo browser
  scenarios. Their product-neutral widgets remain available for later real
  gameplay features.
- Template README and reinitialization script after bootstrap.

These decisions are recorded in
[ADR 0001](adr/0001-ui-donor-and-real-rust-shell.md) and
`template-manifest.json`.

## Ruleweaver game translation references

The Gold Box expansion was planned against two local, separately owned
repositories:

- `FuzzySlipper/ruleweaver` at
  `04ef26d0eef1ba478a2c39b78cca61fe82b15be5`
  (`2026-05-19T04:04:51-07:00`). The bounded scenario candidates reviewed for
  later translation are `content/scenarios/goblin-ambush.json` and
  `content/scenarios/kobold-warren.json`; the larger class, action, equipment,
  and creature catalogs remain source material, not automatic runtime input.
- `FuzzySlipper/asha-d20-fantasy` at
  `e2bcc32346e70555b59a10034d8621118d53a27c`
  (`2026-07-26T11:25:50-07:00`). The useful prior translation surfaces are
  `rulesets/ruleweaver-tactics`,
  `content-packs/ruleweaver-foundation`, and
  `play-bundles/ruleweaver-foundation.ts`.

Task 6388 reviewed and translated the following bounded inventory. The
Ruleweaver checkout has no top-level license or copying notice, so it is design
evidence only: no prose, record, stat block, or catalog bytes were copied. Asha
`SOURCES.md` identifies its Ruleweaver Tactics foundation and Crosswind content
as clean-room original work released under CC BY 4.0. Rusty D20 attributes that
review here and adapts only selected mechanical vocabulary and independently
authored values through its own schema.

| Disposition | Reviewed source                                                                                                                                               | Rusty D20 result                                                                                                                                                                                                                      |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Adopted     | Ruleweaver `design-doc.md`                                                                                                                                    | Six attributes: Might, Finesse, Acuity, Intellect, Conviction, Spirit; Armor, Grit, Wits, and Nerve defense derivation.                                                                                                               |
| Adopted     | Ruleweaver `docs/design/action-economy-conditions-targeting.md` and Asha `rulesets/ruleweaver-tactics/src/ruleset.ts`                                         | Standard, Bonus, Reaction, and Movement budgets with initial amounts 1/1/1/6; bounded participant/cell, team, target-count, range, and line-of-effect action shapes.                                                                  |
| Adapted     | Ruleweaver `docs/design/equipment-system.md`, `docs/design/implement-system.md`; Asha `content-packs/ruleweaver-foundation/src/items.ts` and `src/actions.ts` | Training Blade and Field Bow become distinct Rust-compiled implement definitions and canonical Engine items. Implement-bound actions resolve ability, defense, damage, and range only from the currently equipped authored implement. |
| Adapted     | Ruleweaver condition design; Asha `content-packs/ruleweaver-foundation/src/effects.ts`                                                                        | Held and Unsettled become bounded Rust condition clauses: movement prohibition, action-tag prohibition, and attack penalty. Existing Warden and Ember content uses the same clause system.                                            |
| Adapted     | Asha Ruleweaver foundation action examples                                                                                                                    | Existing playable actions now carry explicit tags, activation cost, target shape, range, and fixed or implement-bound attack identity. Pin in Place and Disrupt exercise selected control conditions in the default encounter.        |
| Replaced    | Previous Rusty D20 Strength/Dexterity/Wisdom/Constitution and Armor/Resolve/Fortitude vocabulary                                                              | Replaced by the six-attribute/four-defense foundation. Previous direct weapon-shaped actions now use the explicit attack union and equipment binding.                                                                                 |
| Replaced    | Asha TypeScript predicate/effect execution and package shape                                                                                                  | Replaced by Rust semantic compilation, registered Engine entity facts, named Engine services, complete Rust saves, Rust-generated browser protocol, and strict decoding.                                                              |
| Excluded    | Ruleweaver `docs/dnd4e-content/**`, `content/**/imported/**`, class/talent catalogs, creature catalogs, broad action/equipment catalogs                       | No bulk import, transformed dataset, branded creature, class system, or universal rules AST.                                                                                                                                          |
| Excluded    | Ruleweaver `content/scenarios/goblin-ambush.json`, `content/scenarios/kobold-warren.json`; Asha Crosswind Outpost content                                     | Reviewed only as scale/shape witnesses. Task 6388 does not copy those encounters or claim the later bounded adventure milestone.                                                                                                      |

Task 6391 completes that later bounded milestone without changing either donor
pin. Warden's Gate adapts the reviewed Asha Crosswind Outpost party/opposition
role shape into Ward Anchor, Pathfinder, Signal Guide, Field Shaper, Line
Sentry, Warden Pathfinder, Field Adept, and Gate Sentinel roles. Rusty D20 owns
the new entity identities, values, equipment assignments, three encounter
boards, Warden pass topology, sigil treasure, door, checkpoint, outcomes, and
all presentation prose. No Ruleweaver scenario bytes, branded catalog entries,
or TypeScript runtime behavior were copied. The Ruleweaver scenarios remain
scale witnesses; Asha's CC BY 4.0 clean-room foundation/Crosswind content is the
attributed adaptation source.

Task 6451 keeps both donor pins unchanged. Its bounded experience values and
feature labels/descriptions are new Rusty D20-authored presentation content for
the already attributed party-role adaptation; no Ruleweaver class/talent data,
feature prose, progression table, or executable rule behavior was copied. Rust
schema 6 seals those definitions and selected identities before the browser can
inspect them.

Neither donor repository is a build/runtime dependency or sibling fallback.
The committed canonical artifacts contain only the selected Rusty D20-owned
translation and their exact local source provenance.

## GM7 product compositions

The interactive reference slice resolves its selected adventure from checked
`catalog.json`. Warden's Gate compiles the exact `starter-core`,
`steel-guard`, and `wardens-gate` closure; Ember's Wake compiles the exact
`starter-core`, `ember-ward`, and `embers-wake` closure. Canonical package
fingerprints and exact source paths remain recorded by
`rules/artifacts/starter/manifest.json` and the artifacts themselves. The
browser receives the compiled ruleset fingerprint and immutable selectable
catalog projection, not authored TypeScript or an executable rules
representation.

This composition did not change the then-selected Engine revision or the UI
donor revision `68ddfa5430ec3bc2cf7ca96963982db9511e79ba`; D20G1D and D20G1E did not
change either pin.

## D20G1 campaign-shell disposition

D20G1A keeps both reviewed pins unchanged while turning the permanent shell
into the start of one durable Rust-owned adventure. Character status, hotbar,
and combat log remain real encounter surfaces. Character status is also reused
for the real camp hero projection.

D20G1B connects the retained inventory and equipment widgets to strict
Rust-owned projections and named loadout commands. The widgets remain
presentational; Engine inventory, containment, equipment, capacity, and source
facts stay authoritative. Compass and minimap remain excluded until Rusty D20
owns real navigation facts, and settings remain deferred until the product has
meaningful settings to expose. No fixture-driven donor screen or second UI
authority was reintroduced.

D20G1C keeps both reviewed pins unchanged. It adapts the retained status,
hotbar, and combat-log surfaces for explicit player decisions, opponent turns,
and a terminal outcome without importing donor gameplay authority. Rusty D20 owns
the deterministic opponent choice, outcome, reward, recovery, and save policy;
the reward transfer and recovery use the already pinned Engine equipment,
inventory, and track services. No reusable gap or pin change was required.

D20G1D also keeps both pins unchanged. The selectable Ember's Wake composition
was added through the existing public authoring pipeline and uses the same
Rust semantic compiler, campaign/session orchestration, Engine mechanics, host,
and UI surfaces as Warden's Gate. The generic downstream additions are bounded
adventure selection and multi-defense projection; no Engine change, sibling
checkout, Node gameplay path, or alternate UI authority was introduced.

D20G1E keeps the same boundaries while certifying a coherent playable slice.
Warden's Gate now authors an ordered second encounter through the existing
TypeScript content pipeline; Rusty D20 owns next-encounter admission,
completed-prefix persistence, bounded returning-opponent recovery, and
exactly-once reward consequences. Save identity, malformed-save recovery, and
guarded reset are host/product policy. No Engine mechanism, generic campaign
graph, fixture-driven production path, or second presentation authority was
added. Character status, hotbar, combat log, inventory, and equipment are real
consumers.

## Gold Box exploration disposition

The exploration foundation initially kept the Engine and UI donor pins
unchanged.
Warden's Gate and Ember's Wake now include original bounded dungeon topology,
encounter placements, and landmarks in their existing authored packages. Rust
owns semantic admission, navigation state, collision, discovery, interaction,
trigger activation, persistence, and projection. The retained compass and
minimap are connected only to those Rust-projected facts.

Task 6418 replaces the original CSS corridor illusion with the public Engine
renderer packages already present at the reviewed Engine revision. Rusty D20
maps its bounded relative view to retained floor, ceiling, side-wall, and
front-wall cuboids; Engine owns neutral projection, Three/WebGL realization,
camera, resize, render scheduling, fail-atomic frame application, and disposal.
No Engine pin change, new Engine mechanism, Ruleweaver bulk import, Asha
runtime, fixture transport, browser rules evaluator, or direct browser
encounter command was introduced.

Task 6426 promotes that same public surface into the permanent full-window game
viewport. It reuses the exact package-root APIs and reviewed Engine revision:
no dependency or donor pin changed. Rusty D20 adds only downstream scene
adaptation and abstract phase backdrops; Engine continues to own the surface,
camera, retained application, render loop, resize, and disposal. The catalog,
camp, encounter, outcome, terminal, loading, and failure backdrops intentionally
carry no Rust or browser-authored gameplay identities.

Task 6428 reuses the same reviewed Engine surface without a pin or donor
change. Rusty D20 maps only its immutable tactical projection into retained
cell, obstacle, participant, marker, and route nodes. Public renderer handles,
metadata source identities, viewport picking, camera control, resize, and
disposal remain Engine processes. Angular arranges initiative, status, action,
log, reaction, outcome, and failure overlays and translates a typed picked
cell back into the pre-existing Rust command path; it does not acquire
movement, occupancy, targeting, or outcome authority.

Task 6429 also keeps the reviewed Engine and donor pins unchanged. Rusty D20
derives a bounded step/quarter-turn presentation offset only after two
successive Rust projections have been accepted, then delegates interpolation
to the public Engine `sampleCameraTransition` process and submits its sampled
pose through `RendererSurface.setCameraPose`. No dungeon topology, collision,
discovery, trigger, save fact, input queue, private Three object, or CSS canvas
transform participates in the tween.

Task 6431 reconciles the renderer-first campaign without changing any source
pin: renderer root task 6426 is approved at
`9cc5695f7c614b3e63abd0c312d299975a40afea`, loadout task 6427 at
`cba4918f96fe6a58a8e3e3682800a39ecaeaf9ca`, tactical renderer task 6428 at
`9638de2b53942ebc690aa8d3ed15819f4311db49`, action-first targeting task 6430
at `7e110ff61b0232a910e570c58c3b221638b3f90b`, and camera tween task 6429 at
`5df74a4c9295dcbbca661946831bd4ff92277ab6`. The overlay pointer-pass-through
finding and busy loadout false-success finding are verified fixed in those
registered descendant revisions; the remaining descendants had no findings.
The combined proof consumes only public Engine package roots at
`fb608e323a8b44a55195f5720101224ff37fd5db`; no sibling checkout, private Three
object, copied renderer mechanism, donor update, or new content source enters
the certification.

## Ruleweaver foundation translation disposition

The foundation translation keeps both reviewed Engine/UI pins unchanged.
TypeScript authoring gained immutable builders for activation budgets,
implements, condition clauses, shaped targets, attack variants, and typed
equipment references. Rust remains the only semantic owner: it enforces
bounded lists and references, compiles armor and implements into distinct
Engine item definitions, derives Wits/Nerve from the better of their two
governing attributes, resolves implement-bound attacks from canonical equipped
items, applies active condition penalties/prohibitions, and captures equipment
and condition revisions in action previews.

Canonical artifacts were regenerated from the Rust-owned contract. The normal
Warden's Gate encounter equips both selected implements and exposes
implement-bound attacks plus Held/Unsettled control actions. The browser
receives only resolved presentation facts, target/range/cost metadata, and
strict command inputs; it does not evaluate the translated rules. Live
per-actor activation-budget consumption/reset, Engine-routed tactical movement,
spatial range/line-of-effect enforcement, and forced movement are now owned by
the Rust gameplay runtime and projected as strict presentation facts.

## First bounded Ruleweaver adventure disposition

The first complete adventure keeps the reviewed Engine/UI and
Ruleweaver/Asha pins unchanged. Warden's Gate now owns a compact three-encounter
expedition with a four-character party, two ordinary encounters, a final
redoubt, inspectable landmarks, a canonical sigil-buckler treasure, a
treasure-gated door, a durable safe-return checkpoint, and authored terminal
victory/defeat copy. TypeScript authors immutable content only. Rust compiles
the door-aware route, transfers the treasure through Engine inventory,
persists exploration events, admits encounters only at their cells, and owns
the terminal campaign transition. The strict browser protocol and UI only
project and command those Rust facts.
