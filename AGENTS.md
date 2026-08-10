# Rusty D20 agent guidance

## Repository role

Rusty D20 is a concrete downstream d20 game and reference consumer. It owns d20
vocabulary, semantic compilation, orchestration, complete saves, transport,
projection, controls, and presentation. It must never become an umbrella RPG
dependency or a facade that other games must import.

Rusty Engine owns reusable host-neutral mechanisms. Consume its complete Rust
facade through the one unconditional adjacent path in `Cargo.toml`, and consume
its two neutral Rules authoring packages through the checked adjacent links in
`rules/`. Use that checkout exactly as it stands: no D20 script may pull,
synchronize, or mutate it. Do not copy Engine implementations downstream.
Route a genuinely reusable missing mechanism to an Engine task.

## Den Guidance Bootstrap

- Project ID: `rusty-d20`
- Resolve live guidance with the Den MCP `get_agent_guidance` tool before
  substantial work.
- Treat the resolved Den guidance packet and its referenced Den documents as
  the source of truth.
- If Den is unreachable, stop and tell the user which Den tool or command
  failed and what you were about to do. Do not reconstruct Den state from local files.

## Architecture

Read [docs/design.md](docs/design.md) before changing authority, dependency
direction, persistence, protocol generation, or the rules pipeline. Use
[docs/agent-code-atlas.md](docs/agent-code-atlas.md) for path-level ownership.

- Rust is the only semantic and authoritative gameplay runtime.
- TypeScript owns authoring-time composition and browser presentation. It never
  evaluates gameplay rules or owns live authoritative state.
- All persistent entity facts use registered components in Engine's canonical
  `entity-state` store. Do not add shadow entity maps or a mechanics aggregate.
- Call named Engine services directly. Do not add an ambient event bus,
  universal gameplay AST, scheduler, callbacks, or service locator.
- Protocol TypeScript is generated from the Rust owner and strictly decoded at
  the browser boundary. Do not hand-maintain a twin.

## UI boundaries

Preserve the package-root layers: protocol, platform, transport, domain, store,
renderer/components, features, shell, theme, and testing fixtures.

- Do not deep-import another library's `src/` tree.
- Browser APIs go through platform ports.
- Backend calls go through transport; application mutation goes through store.
- Components are presentational; features compose behavior; shell owns routes.
- Classified failures remain visible as `AsyncState<T>`.
- Fake transport, fake content, and placeholder actions are test fixtures only
  and may not enter `apps/app`, `libs/store`, or production features.

## Work and verification

Treat a dirty worktree as shared state. Preserve unrelated changes. Commit and
push each reviewable milestone directly to the current branch; record exact
SHAs in Den when the work is task-managed.

Run the narrowest check first, then the owning gate. The aggregate gate is:

```bash
./scripts/verify.sh
```

User-visible work requires a real Rust-served browser scenario and inspected
artifacts. Synthetic tests do not prove product integration. Update
[docs/source-provenance.md](docs/source-provenance.md) when donor provenance or
the adjacent Engine boundary changes, and
[docs/known-limitations.md](docs/known-limitations.md) whenever an intentional
phase boundary remains.
