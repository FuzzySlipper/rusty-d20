# Extension guide

## Add Rust-owned state

Define durable d20 facts as registered downstream components in the canonical
Engine `EntityState`. Mutate them through exact component revisions and named
Rust services. Keep authored definitions separate from live instances and
complete saves.

## Add a protocol field or operation

1. Change the Rust DTO or add a Rust route.
2. Run `pnpm run protocol:generate`.
3. Tighten the protocol decoder and transport tests.
4. Project the DTO in domain, update store state, then render it in a feature.
5. Run Rust, boundary, TypeScript, build, and real-browser gates.

Do not hand-edit the generated TypeScript file or fetch directly from a feature.

## Add a UI feature

Add a `type:feature`/`scope:feature` Nx library with one public `src/index.ts`.
Compose domain/store/components there, bind its route in `apps/app`, and add an
opt-in live scenario with explicit non-claims. Update `libs/shell` for new route
identities.

## Add authored rules or adventure content

Add source modules under `rules/packages/starter-ruleset/src/content/` using the
package-root `@rusty-d20/rules-authoring` builders. TypeScript may use
functions, loops, tables, and helpers to emit a bounded immutable candidate.
Run `pnpm --dir rules run generate`, inspect the artifact and fingerprint
manifest/catalog, and add Rust assertions when the content proves a new
composition. Character templates, storage, item instances, encounters,
outcomes, rewards, and adventures are ordinary authored definitions; follow
the multi-file example under `content/adventures/`.

An adventure's encounter list is its ordered campaign sequence. The runtime
offers only the next incomplete entry and persists the exact completed prefix.
The same opponent may be referenced again: entry restores only bounded
vitality, while prior resources, effects, equipment, and rewards persist.
Branching, repeatable encounters, or alternate recovery policy would be new
Rust-owned semantics, not fields to smuggle into presentation text.

The runtime resolves an adventure to its exact package dependency closure.
Content-only additions therefore change TypeScript source and regenerated
artifacts, not `game.rs`, `session.rs`, the Rust semantic compiler, or Engine.
If an addition needs new behavior rather than new data, extend the explicit
Rust-owned semantic contract instead of hiding behavior in content.

Rust owns schema admission and semantics. The checked artifact must remain
usable by a Node-free Rust process; no callback or TypeScript evaluator may
enter runtime state.

Use the versioned fields and provenance subjects in
[the d20 rules kernel](d20-rules-kernel.md). Add new fixed semantic vocabulary
to the Rust candidate and compiler together. Do not encode new behavior as an
opaque expression tree just to avoid changing the owning Rust contract.

The full authoring contract and package layout are in
[rules authoring](rules-authoring.md).
