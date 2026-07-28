# Source provenance

## Rusty Engine

All Engine packages are public Git dependencies pinned to reviewed revision
`fb608e323a8b44a55195f5720101224ff37fd5db`:

- `core-ids`
- `entity-state`
- `gameplay-mechanics`
- `gameplay-rules`

`Cargo.lock` records the resolved Git source. There is no sibling path fallback.

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
- Main-menu feature became the permanent Rust runtime bootstrap screen.
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
