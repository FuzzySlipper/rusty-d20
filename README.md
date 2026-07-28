# Rusty D20

Rusty D20 is a concrete d20 RPG and interactive reference consumer for
[Rusty Engine](https://github.com/FuzzySlipper/rusty-engine). It owns d20 rules,
game orchestration, complete saves, transport, and presentation. Rusty Engine
remains optional reusable infrastructure; other games never need to import this
repository.

The current product bootstrap is deliberately small but real: an Angular/Nx
shell is served by a Rust process, calls a same-origin typed endpoint, strictly
decodes a Rust-generated contract, and renders a readout derived from canonical
`entity-state` state. Separately, the headless Rust kernel compiles bounded d20
candidates and runs deterministic, source-attributed action/reaction/effect
lifecycles with complete saves. Fake transport is available only from
`libs/testing-fixtures`.

## Start the product

```bash
pnpm install --frozen-lockfile
pnpm run serve:local
```

Open the printed `BASE_URL`. The page must report `Runtime ready`, one canonical
entity, and the pinned Rusty Engine revision.

## Verify

```bash
./scripts/verify.sh
```

Focused commands and live-evidence instructions are in
[docs/verification.md](docs/verification.md). Architecture and source routing
start at [docs/design.md](docs/design.md) and
[docs/agent-code-atlas.md](docs/agent-code-atlas.md).

## Current phase boundary

The semantic kernel is implemented, but it is not yet connected to the product
host or Angular UI and no TypeScript authoring SDK exists. Encounter play and
substantial d20 content remain later milestones. See
[docs/known-limitations.md](docs/known-limitations.md).

## Provenance

The UI was copied from `FuzzySlipper/rusty-engine-ui` at exact reviewed commit
`68ddfa5430ec3bc2cf7ca96963982db9511e79ba`. Rusty Engine crates are pinned to
exact reviewed commit `fb608e323a8b44a55195f5720101224ff37fd5db` with public
Git dependencies. See [docs/source-provenance.md](docs/source-provenance.md).
