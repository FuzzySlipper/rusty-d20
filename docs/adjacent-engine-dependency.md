# Adjacent Rusty Engine dependency

Rusty D20 consumes one local Rusty Engine checkout placed beside this
repository. `Cargo.toml` declares the complete Rust facade at
`../rusty-engine/rust/crates/rusty-engine`; the isolated `rules/` workspace
links only Engine's semantic-neutral `gameplay-rules-contracts` and
`gameplay-rules-authoring` packages from that same checkout.

There is no Engine pin manifest, revision synchronizer, freshness comparison,
or update command in this repository. D20 scripts must not fetch, pull, reset,
or otherwise mutate the adjacent checkout. The operator chooses which Engine
checkout is present, and D20 compiles against its current files exactly as they
stand.

## Fresh workspace

Place both repositories under one parent directory:

```text
workspace/
  rusty-d20/
  rusty-engine/
```

CI creates this shape ephemerally before installing or compiling D20. For local
work, provision or update Rusty Engine separately, then run only the focused
D20 gate that owns the changed surface:

```bash
cargo test -p rusty-d20 --locked
./scripts/verify-rules.sh
```

Use `./scripts/verify.sh` when the whole product surface needs certification.
`Cargo.lock` and `rules/pnpm-lock.yaml` prove path resolution and third-party
package versions; neither lockfile is an Engine Git-revision carrier.

## Studio boundary

D20 does not embed or configure the Engine Studio or renderer workspaces. If a
future project adapter is needed, the Engine-hosted Studio boundary remains the
owner: `.rusty-studio.json`, project data, and a Rust adapter are the integration
surface. Do not add a downstream Studio shell, renderer TypeScript/Three
package, private bridge, or child HTML document.

## Ownership

Rusty Engine owns reusable mechanisms and the neutral authoring packages. D20
owns d20 semantics, content, orchestration, complete saves, generated
protocols, and presentation. Engine revision identity does not cross the
runtime protocol or persistence boundary. If the adjacent checkout exposes an
incompatibility, adapt D20 or route a reusable missing mechanism upstream; do
not copy Engine code or add a synchronization layer here.
