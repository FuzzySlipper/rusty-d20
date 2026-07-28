# Rusty D20

Rusty D20 is a concrete d20 RPG and interactive reference consumer for
[Rusty Engine](https://github.com/FuzzySlipper/rusty-engine). It owns d20 rules,
game orchestration, complete saves, transport, and presentation. Rusty Engine
remains optional reusable infrastructure; other games never need to import this
repository.

The current product is a deliberately bounded but real encounter. An Angular/Nx
shell is served by a Rust process and reaches Rust-owned state through a strict
generated same-origin contract. The browser can start or reopen the Steel Guard
starter encounter, choose an authored action and target, apply an optional
reaction, resolve deterministic source-attributed damage/effects, advance the
caller-owned turn, and save. A fresh Rust process continues from the same
authoritative state without replay. The checked starter catalog also includes
the separate ember/resolve composition for headless and authoring proof. Fake
transport remains available only from `libs/testing-fixtures`.

## Start the product

```bash
pnpm install --frozen-lockfile
pnpm run serve:local
```

Open the printed `BASE_URL`. With no prior save, start **The Warden's Gate**.
The host writes an explicit save to `target/rusty-d20/save.json`; stopping and
starting the command again reopens it.

## Verify

```bash
./scripts/verify.sh
```

Focused commands and live-evidence instructions are in
[docs/verification.md](docs/verification.md). Architecture and source routing
start at [docs/design.md](docs/design.md) and
[docs/agent-code-atlas.md](docs/agent-code-atlas.md).

## Current phase boundary

The GM7 reference slice is connected end to end. It is not yet a broad d20
product: initiative, movement, spellcasting, advancement, content publication,
save migration, and additional encounter/UI work remain later milestones. See
[docs/known-limitations.md](docs/known-limitations.md).

## Provenance

The UI was copied from `FuzzySlipper/rusty-engine-ui` at exact reviewed commit
`68ddfa5430ec3bc2cf7ca96963982db9511e79ba`. Rusty Engine crates are pinned to
exact reviewed commit `fb608e323a8b44a55195f5720101224ff37fd5db` with public
Git dependencies. See [docs/source-provenance.md](docs/source-provenance.md).
