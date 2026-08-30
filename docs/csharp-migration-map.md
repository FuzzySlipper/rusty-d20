# C# migration map

**Status:** target ownership map for Den task 7598.  This is a dependency-
ordered cutover plan, not a promise that the C# product or every Engine call
already exists.  Rusty D20 becomes one ordinary C# Rusty Engine consumer.  It
does not become a reusable RPG framework, and it does not preserve a second
Rust or browser runtime for compatibility.

## Authority and provider boundary

| Owner | Keeps authority | Does not become |
| --- | --- | --- |
| `RustyD20.Core` and the C# product | D20 vocabulary and meaning, authored content interpretation, semantic admission, campaign/session orchestration, action resolution, save meaning, tuning, receipts, and projections | An Engine wrapper, a generic rules language, or a second host loop |
| `Rusty.Engine` | NativeAOT lifecycle and input delivery, retained rendering/resources, spatial/navigation/collision/perception, content, deterministic random, persistence primitives, UI publication, and managed entity/mechanics/inventory/equipment helpers | D20 rules, adventure policy, encounter AI, or campaign state |
| Optional DOM/accessibility shell | Labels, focus, keyboard/touch translation, accessible controls, and display of published facts | Gameplay, protocol authority, a renderer, save state, or a client-side rules evaluator |

The migration targets the adjacent Engine checkout observed at
`56f857ca789321c987d1caf85b36b137226a0d03`.  This is a survey baseline, not a
gameplay save identity or a reason to mutate or synchronize the sibling
checkout.  C# consumes the generated safe `Rusty.Engine` contracts and never
adds P/Invoke, unsafe ABI declarations, handwritten bindings, or copied Engine
implementation.

The current Rust and TypeScript trees are semantic and content evidence.  The
existing six-package catalog, Warden's Gate, and Ember's Wake remain the named
content to preserve, but the old candidate/artifact pipeline is not a runtime
dependency after cutover.

## Target dependency order

The supported path is:

```text
C# authored content and typed limits
  -> strict D20 definitions, diagnostics, and composition fingerprint
  -> Engine-backed entities, mechanics, roll source, and session
  -> campaign, exploration, tactical encounter, and save policy
  -> typed projections, receipts, Engine presentation/UI
  -> IEngineProduct and NativeAOT composition
```

Implement in this order, keeping each boundary usable before the next one is
started:

1. Establish the solution and generated Engine reference.  Add explicit
   `D20Limits`, typed errors/diagnostics, stable IDs, current content/save
   schema identifiers, and observability records.
2. Port authored content and semantic compilation.  Build Warden's Gate and
   Ember's Wake from inspectable C# modules, validate references and bounds,
   normalize definition order, and calculate a composition fingerprint.
3. Add the Engine-backed session.  Register D20 durable components beside
   Engine mechanics components, seed party/items/equipment, implement seeded
   and static roll sources, and make action/reaction resolution staged and
   failure-atomic.
4. Add campaign and encounter policy.  Implement camp, exploration, outcome,
   ordered trigger admission, hidden topology projection, tactical legality,
   deterministic opposition, and reaction custody over the session.
5. Add strict persistence.  Save the complete product meaning, restore into a
   fresh candidate, re-admit the exact content closure, validate all
   cross-references, and publish only after validation succeeds.
6. Add projections and presentation.  Publish bounded exploration/tactical
   views, action receipts, combat-log detail, character/loadout facts, and
   named tuning/provenance readouts through Engine services.  An optional DOM
   shell remains observational and accessible.
7. Compose `IEngineProduct` and the NativeAOT product.  Lifecycle callbacks
   forward admitted Engine updates to the product; the product owns no clock,
   scheduler, renderer loop, or handwritten interop.
8. Once the vertical path is the only supported route, remove the old Rust,
   Node, Angular/Nx, generated-protocol, browser-runtime, and compatibility
   families listed below.  Replace only their narrow owning gates.

## Target C# layout

These are product ownership seams, not a framework or mandatory registration
API:

```text
src/
  RustyD20.Core/
    Contract/       D20 IDs, schemas, limits, DTOs, diagnostics
    Content/        Warden's Gate, Ember's Wake, and source/provenance facts
    Rules/          definitions, composition, semantic compiler, fingerprints
    Components/     D20-owned durable EntityWorld component values
    Session/        turns, roll sources, previews, atomic resolution, receipts
    Campaign/       phases, admission, exploration, outcomes, progression
    Tactical/       boards, targets, initiative, opposition, reactions
    Persistence/    current save codec and strict restore validation
    Projection/     bounded readouts, logs, and presentation facts
  RustyD20.Product/
    D20Product.cs   IEngineProduct-facing orchestration and service wiring
  RustyD20.NativeProduct/
    NativeProduct.cs  one EngineProduct selection and NativeAOT composition
  RustyD20.Product.Checks/
    focused semantic and lifecycle/readout checks
```

`Core` may reference the public Engine assembly for managed mechanisms and
generated values, but it must not mirror the Engine ABI.  Domain methods should
remain ordinary C# and follow a readable Read -> Decide -> Apply -> Publish
shape where a multi-service command needs it.  A command bus, ambient event
bus, service locator, universal gameplay AST, or hidden scheduler is not part
of this map.

## Semantic contract to preserve

The following behavior is product meaning and must survive translation.  Keep
the values in named records/constants or authored definitions so they are
visible and tunable; do not scatter magic numbers through resolution code.

### Strict contracts and bounds

- IDs accept only lowercase ASCII `a-z`, digits, `.`, `_`, and `-`, with at
  most 64 UTF-8 bytes.  Unknown fields, malformed tags, duplicate identities,
  unknown references, unsupported schema versions, and noncanonical lists
  reject with correlated diagnostics.
- Preserve the current explicit quotas: 64 definitions per kind, 16
  adventures per package, 64 adventure entries, 512 authored-text bytes,
  dungeon dimensions of 24 by 24, tactical boards of 16 by 16, 32 damage dice,
  1,000 die sides, 10,000 effect-duration turns, 1,000,000,000 experience,
  16 action/implement tags, 4 activation costs, 8 condition clauses, tactical
  range 32, forced movement 6, 12 action targets, 4 party members, and 12
  encounter participants.  Static action-roll tapes remain bounded at 4,096
  entries.
- Keep candidate schema 6's vocabulary as behavioral evidence while moving
  authored definitions into C#.  The C# product must declare one current
  content/session/save schema and reject the old Rust candidate/session/product
  schemas and every unknown schema.  There is no compatibility migration or
  defaulting of missing facts.
- Preserve source path, subject identity, package/adventure identity, exact
  composition fingerprint, and diagnostic detail in authored definitions and
  compile failures.  Fingerprints change when content changes, not merely when
  C# helper modules are reordered.

### Rules and resolution

- Preserve the six attributes and four defenses, including Wits/Nerve's
  better-of-two governing-ability decision.  Ability modifiers use mathematical
  floor division for negative scores; C# integer truncation is not equivalent.
- Preserve action tags, activation costs, target shape/team/count, range,
  line-of-effect policy, fixed versus implement-bound attacks, damage types,
  affinity responses, resource costs, and bounded condition clauses.  An
  implement-bound action resolves its ability, defense, damage, and range from
  the currently equipped canonical Engine item at preview time.
- A roll source is explicitly either a saved seeded stream or a bounded static
  action-roll tape.  Consume draws in the authored order; a target-selection
  `choice_index` does not consume an action roll.  Save the complete tagged
  source configuration and next position.
- Preview captures the explicit turn, roll position, actor/equipment state,
  scheduled-condition state, and every relevant component revision.  A stale
  campaign/encounter/actor/action/reaction/equipment/condition revision is
  rejected before mutation.  A reaction spends its resource and schedules its
  effect in the same staged transaction, then requires a fresh action preview.
- Apply an action, reaction, damage, resource/equipment/effect updates, turn
  changes, roll position, and receipts in a clone or equivalent prepared
  state.  If any late validation or Engine operation fails, the live product,
  Engine state, roll position, revision, and log remain unchanged.

### Campaign, exploration, and tactical policy

- Use explicit `Camp`, `Exploration`, `Encounter`, `Outcome`, and
  `AdventureComplete` phases.  The authored encounter list is ordered; only
  the next incomplete trigger is admitted.  Completed encounter identities,
  dungeon discoveries, opened doors, collected treasures, and safe-return
  checkpoints are durable facts.
- Exploration owns the party pose, facing, collision admission, landmark and
  treasure interaction, door prerequisites, checkpoint return, and trigger
  transition.  The observer receives only an occlusion-safe first-person view
  bounded to three depths, not the complete topology or hidden trigger cells.
  Ask Engine spatial/navigation/perception services for reusable mechanism
  facts; do not port local BFS, A*, ray casting, shadow casting, or a parallel
  occupancy map.
- Tactical legality remains C# policy over Engine spatial facts: Chebyshev
  range, line of effect, target team/shape, living-participant admission,
  movement budget, forced movement, and condition prohibitions.  Initiative is
  descending with stable entity-identity tie breaks and dead actors are
  skipped.
- Opposition is deterministic and bounded by the admitted participant limit.
  A player command settles consecutive opposition activations in the same
  product transition.  If an opposition action offers a player reaction, C#
  retains the opaque decision and immediately resumes after choose/decline;
  there is no idle opposition acknowledgement command or durable pending
  reaction.
- Victory, defeat, exactly-once reward transfer, bounded returning-opponent
  vitality, camp recovery, terminal copy, and continuation are D20 campaign
  policy.  Engine owns tracks, effects, inventory, containment, equipment, and
  attributed stat sources; it does not decide the outcome or reward meaning.

### Persistence and observability

The current C# save contains the schema and content fingerprint, complete
authored composition identity, roll-source configuration/position, phase,
campaign and exploration progress, encounter identities and turn owner,
canonical D20 and Engine entity/component state, loadout, outcome, revision,
operation/log identities, and bounded receipt log.  It does not contain copied
compiled definitions or process-local previews.

Restore must resolve the exact authored content closure, reconstruct a fresh
Engine-backed entity world, validate component identity/revision relationships,
phase/turn/outcome/vitality consistency, ordered encounter history, doors and
treasures, loadout/equipment, and the requirement for a living party where
applicable.  Reject unknown or legacy schemas, partial loadouts, impossible
budgets, unknown event identities, unreachable discoveries, idle opposition,
pending reactions, and mismatched fingerprints without changing the live
state or durable file.

Every accepted command should expose a stable operation identity, changed
revision, relevant source identities, roll/modifier/defense/damage facts,
effect/resource changes, and rejection reason where applicable.  The
projection and combat log explain committed facts; they never recalculate
rules, replay random draws, or become a second command source.  Keep tuning
records for limits, roll policy, opposition policy, exploration view depth,
and presentation choices alongside the domains that use them.

## Engine C# mapping

Use the current public SDK documented in the adjacent `csharp-sdk.md` and
`csharp-capabilities.md`:

| Rusty D20 need | Engine surface | C# product responsibility |
| --- | --- | --- |
| Product lifecycle/input | Generated `IEngineProduct`, `ProductCreateContext`, `ProductUpdate`, copied input events, and lifecycle callbacks | Translate admitted inputs to typed D20 commands; never create a second loop or retain borrowed update data |
| D20 entity facts | Managed `EntityWorld`, `ComponentType<T>`, snapshots/batches | Register D20 ability, action-resource, activation-budget, encounter-participation/position, and scheduled-effect components; own their meaning and codecs |
| Stats, vitality, effects, damage, inventory, equipment | `ExactStatEvaluator`, track/effect mechanisms, `InventoryWorld`, `EquipmentState`, and managed `Resolution` helpers where useful | Compile D20 definitions, choose policy, validate prospective operations, and stage product transitions |
| Randomness | Generated `Random` service | Select and persist the D20 seeded/static policy; do not implement an untracked RNG in UI or Engine |
| Dungeon/tactical spatial facts | `Spatial`/`SpatialSession`, collision, navigation, perception, voxel picking, and supported content replacement | Supply authored bounded content and policy; retain canonical positions/routes only as D20 facts |
| Retained game surface | `Appearance`, `Presentation`, `CameraView`, `VoxelScenePresentation`, animation/audio as needed | Adapt exploration/tactical projections and tune named presentation facts; dispose leases and resources at their owning scope |
| Product readouts and UI | Generated `Ui` service and optional DOM/accessibility shell | Publish bounded observability; preserve ARIA live log, keyboard/touch alternatives, focus restoration, disabled/selected states, and 44px touch targets without making them authority |
| Save/load | `Persistence`, `ProductStateStore<T>` or entity-world persistence helpers | Define the closed D20 save schema, identity, strict restore, and file/reset policy |

The Engine owns collision/navigation/path queries, camera/render scheduling,
surface picking, retained renderer resources, input normalization, content
admission, persistence primitives, and lease lifetime.  D20 must not recreate
those mechanisms in C#, TypeScript, a JSON bridge, or a browser canvas.  Each
Engine call has its own validation/failure behavior; an exception does not
implicitly roll back earlier mutable spatial or voxel calls, so product policy
must validate first and retain receipts/revisions explicitly.

## Content and provenance disposition

Re-author these current concrete compositions as C# content modules with
stable names, source paths, labels, and inspectable values:

- the shared six-attribute/four-defense foundation and Standard/Bonus/Reaction/
  Movement budgets;
- the steel/armor package with Training Blade, Field Bow, control effects,
  physical actions, and reaction resources;
- the ember/Nerve package with its distinct equipment, focus/ward behavior,
  and energy/resolve actions;
- Warden's Gate with its four-person cast, loadout/stash, ordered three-
  encounter expedition, dungeon landmarks, sigil treasure, gated door,
  checkpoint, rewards, and authored terminal outcomes; and
- Ember's Wake with its distinct cast/loadout, Ash Seer encounter, reward,
  terminal outcome, and authored exploration facts.

These are product-owned values and clean-room adaptations recorded by
`docs/source-provenance.md`; do not copy Ruleweaver/Asha runtime code, broad
catalogs, branded records, or executable TypeScript.  If a content artifact is
used for Engine `Content`/`AuthoredContent`, it is an immutable admitted input
with explicit provenance, not a second D20 rules runtime.  Old TypeScript
authoring packages and generated JSON catalog bytes may be consulted while
porting, then cease to be active inputs.

## Exact legacy deletion families

After the C# vertical slice owns the behaviors above, delete the old families
as a hard cut.  Do not leave wrappers, adapters, fallback binaries, or a
dual-host compatibility mode:

| Delete | Reason |
| --- | --- |
| `rust/`, root `Cargo.toml`, and `Cargo.lock` | Rust candidate/compiler/session/game/host/presentation runtime and Rust test binaries are replaced by C# |
| `rules/` and its package/artifact lockfiles | TypeScript D20 authoring, generated contract, canonical package catalog, and rules test pipeline are no longer runtime inputs |
| `apps/`, `libs/`, `apps/app-e2e/`, and generated `libs/protocol` DTOs | Angular/Nx browser gameplay, HTTP transport/store, handwritten renderer/picker, and broad Playwright runtime are retired; recreate only an observational shell if needed |
| `package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, `nx.json`, `tsconfig*.json`, `vitest.config.ts`, `eslint.config.mjs`, and `boundaries.json` | Node/Nx/TypeScript workspace machinery is bound to the retired architecture |
| Old verification scripts and `tools/` generators/audits | Rust/Node/browser aggregate gates and protocol generators cannot remain compatibility obligations |
| `.github/workflows/ci.yml`, `product-playtest.scenario.json`, `.den-serve.json`, `.playwright-service.json`, and `template-manifest.json` when their old commands are removed | Old CI and browser-service metadata describe the retired host and workflows |
| Rust, TypeScript authoring, Vitest, and Playwright suites tied to the old schemas/protocol | Broad legacy coverage is not a migration requirement; retain only focused C# checks that prove current behavior |

Rewrite rather than blindly delete `docs/design.md`,
`docs/agent-code-atlas.md`, and `docs/verification.md` so they describe the
C# owner and current gates.  Update `docs/source-provenance.md` for the C#
content and Engine SDK boundary, and update `docs/known-limitations.md` for
any intentional host/UI phase boundary.  Historical source references may
remain as provenance prose, never as build dependencies.

## Replacement gates and cutover checkpoints

Keep validation deliberately focused:

1. **Contract/content gate:** build C# definitions, reject malformed/unknown
   content and all stated bounds, preserve source diagnostics, and prove both
   named adventures compile to stable fingerprints.
2. **Headless semantic gate:** run one short check project covering floor
   division, seeded/static draw order, choice-index behavior, staged rollback,
   stale preview/revision fences, action/reaction effects, ordered encounters,
   bounded opposition, and strict current-schema save rejection/reopen.
3. **Engine product gate:** build Release, publish the NativeAOT composition,
   start the actual Engine product, exercise lifecycle/input/presentation,
   dispose resources, and confirm terminal shutdown does not call product
   services after disposal.
4. **Small visible scenario:** select Warden's Gate, prepare the camp,
   explore to a landmark and trigger, resolve one tactical action plus an
   opposition/reaction branch, save, stop, reopen, and inspect the same
   observable facts.  Keep the log/receipts and named tuning values visible.
5. **Reuse audit:** after implementation, inspect the exact downstream SHA
   against the current Engine checkout with `rusty-engine-reuse-audit`.  The
   audit is specifically for duplicated Engine mechanisms, missed safe SDK
   calls, and provisional wrappers; it is not a generic style or coverage
   review.

The intended commands are the narrow C# equivalents of the current product
checks:

```bash
dotnet run --project src/RustyD20.Product.Checks/RustyD20.Product.Checks.csproj
dotnet run --project src/RustyD20.Core.Checks/RustyD20.Core.Checks.csproj
dotnet build RustyD20.sln -c Release
dotnet publish src/RustyD20.NativeProduct/RustyD20.NativeProduct.csproj -c Release -r linux-x64
bash src/scripts/exercise-native-product.sh
```

Do not recreate an aggregate legacy workflow or an exhaustive browser matrix.
Commit and push each reviewable milestone on the current branch and record its
exact SHA in Den as required by the repository guidance.

## Genuine gaps and stopping rule

The surveys found no definite Engine capability gap for D20's semantic,
mechanics, inventory/equipment, random, persistence, spatial/navigation, or
retained presentation needs.  Exact generated method names must still be read
from the current Engine checkout at implementation time; this map is not an
API promise.

If a required operation cannot be expressed through the generated public
surface, record the concrete request/fact shape and lifecycle point, file or
link the narrow upstream Engine task when authorized, and stop that slice.
Do not hide an Engine gap behind a C# port of Engine code, a Rust sidecar,
handwritten interop, a browser renderer, a local pathfinder/raycast, a JSON
command bus, or a provisional compatibility layer.

The only optional product choice is whether to keep a minimal DOM shell for
accessibility and visible inspection.  Either way, C# remains the sole D20
authority and Engine remains the sole reusable host/render/spatial/persistence
authority.
