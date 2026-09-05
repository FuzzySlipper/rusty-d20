# Rusty D20 agent guidance

## Repository role

Rusty D20 is one concrete C# d20 game and a downstream Rusty Engine consumer.
It owns d20 vocabulary, authored-content interpretation, semantic admission,
session and campaign policy, action resolution, complete-save meaning, controls,
and observational projections. It is not a reusable RPG framework and must
never become a dependency of another game.

Rusty Engine owns reusable host-neutral mechanisms. Ordinary development
consumes the pinned immutable `Rusty.Engine` package from the ignored local
`.runtime/sdk-feed` and uses its exactly matched `.runtime/runtime-pack-cbf35130d06c`.
Do not discover, synchronize, mutate, or copy an Engine checkout from this
repository; route reusable gaps upstream instead.

## Den guidance bootstrap

- Project ID: `rusty-d20`
- Resolve live guidance with Den's `get_agent_guidance` before substantial work.
- Treat the resolved packet and its referenced Den documents as the source of
  truth.
- If Den is unreachable, stop and report the failed operation rather than
  reconstructing Den state from local files.

## Architecture

Read [docs/design.md](docs/design.md) before changing authority, dependency
direction, persistence, floor admission, or the turn model. Use
[docs/agent-code-atlas.md](docs/agent-code-atlas.md) for path ownership and
[docs/csharp-migration-map.md](docs/csharp-migration-map.md) for the retained
historical cutover disposition.

- C# is the sole authoritative gameplay runtime.
- `RustyD20Product` is the one SDK-declared `IEngineProduct`: it reacts to
  admitted updates and never creates a second loop or handwritten interop
  boundary. The SDK generates CoreCLR and NativeAOT composition beneath `obj`.
- Rusty D20 owns d20 policy and the meanings it stores. Rusty Engine owns
  lifecycle/input, renderer resources, spatial/navigation/collision,
  deterministic random, content, UI streams, and durable storage primitives.
- The development host's minimal HTML page and UI stream are observational;
  neither is gameplay authority or a renderer.
- Keep C# content modules modular and inspectable. Source provenance is a
  record of clean-room adaptation, not an active TypeScript, Rust, or JSON
  content pipeline.

## Work and verification

Treat a dirty worktree as shared state. Preserve unrelated changes, especially
`.agent-teams/`. Commit, push, and Den transitions require task authorization.

Run focused maintained checks:

```bash
dotnet run --project src/RustyD20.Core.Checks/RustyD20.Core.Checks.csproj
dotnet run --project src/RustyD20.Product.Checks/RustyD20.Product.Checks.csproj
dotnet build RustyD20.sln -c Release
./.runtime/runtime-pack-cbf35130d06c/bin/rusty dev --project src/RustyD20.Product/RustyD20.Product.csproj --runtime ./.runtime/runtime-pack-cbf35130d06c
dotnet msbuild src/RustyD20.Product/RustyD20.Product.csproj -t:VerifyRustyEngineAot -p:Configuration=Release
```

`rusty dev` is the normal CoreCLR edit-run path and stages the loose product
atomically. NativeAOT is an explicit fidelity/release check, not an edit-run
loop. An Engine contributor may add `--engine-source /absolute/rusty-engine`
to that command; it is the only source-use override. Do not restore Cargo/Rust,
Node/pnpm/Nx/Angular, generated-protocol, old-host, or broad browser/E2E
workflows. Add a focused C# proof only for new C# behavior.

Update [docs/source-provenance.md](docs/source-provenance.md) when Engine or
content source selection changes, and
[docs/known-limitations.md](docs/known-limitations.md) when an intentional C#
phase boundary remains.
