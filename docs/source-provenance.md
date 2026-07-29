# Source provenance

## Rusty Engine

All Engine packages are public Git dependencies pinned to reviewed revision
`fb608e323a8b44a55195f5720101224ff37fd5db`:

- `core-ids`
- `entity-state`
- `gameplay-mechanics`
- `gameplay-rules`
- `svc-rng`

`Cargo.lock` records the resolved Git source. There is no sibling path fallback.

The isolated TypeScript authoring workspace pins these public Engine packages
to the same revision and exact repository subpaths:

- `@rusty-engine/gameplay-rules-contracts`
- `@rusty-engine/gameplay-rules-authoring`

`rules/pnpm-lock.yaml` records the codeload revision and subpath identities.
Only those exact git package prepare scripts are allowed by
`rules/pnpm-workspace.yaml`.

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

This composition continues to use Rusty Engine revision
`fb608e323a8b44a55195f5720101224ff37fd5db` and the UI donor revision
`68ddfa5430ec3bc2cf7ca96963982db9511e79ba`; D20G1D and D20G1E do not change
either pin.

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
hotbar, and combat-log surfaces for explicit player/opposition turns and a
terminal outcome without importing donor gameplay authority. Rusty D20 owns
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
consumers; compass and minimap remain explicitly unconnected until navigation
facts exist.
