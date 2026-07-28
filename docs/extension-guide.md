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

## Add authored rules

TypeScript authoring may use functions, loops, tables, and helpers to emit a
bounded immutable candidate. Rust owns schema admission and semantics. The
checked artifact must remain usable by a Node-free Rust process; no callback or
TypeScript evaluator may enter runtime state.
