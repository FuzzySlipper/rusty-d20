# Rusty D20 agent guidance

## Repository role

Rusty D20 is one concrete C# d20 game and a downstream Rusty Engine consumer.
It owns d20 vocabulary, authored-content interpretation, semantic admission,
session and campaign policy, action resolution, complete-save meaning, controls,
and observational projections. It is not a reusable RPG framework and must
never become a dependency of another game.

Rusty Engine owns reusable host-neutral mechanisms. Consume the adjacent
`../rusty-engine` checkout exactly as it stands through its public C# SDK and
generated product contract. Do not pull, synchronize, mutate, or copy Engine
implementation from this repository; route reusable gaps upstream instead.

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
- `RustyD20Product` is an Engine-owned `IEngineProduct`: it reacts to admitted
  updates and never creates a second loop or handwritten interop boundary.
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
`.agent-teams/`. Commit and push each reviewable milestone directly to the
current branch and record its exact SHA in Den.

Run focused maintained checks:

```bash
dotnet run --project src/RustyD20.Core.Checks/RustyD20.Core.Checks.csproj
dotnet run --project src/RustyD20.Product.Checks/RustyD20.Product.Checks.csproj
dotnet build RustyD20.sln -c Release
dotnet publish src/RustyD20.NativeProduct/RustyD20.NativeProduct.csproj -c Release -r linux-x64
bash src/scripts/exercise-native-product.sh
```

The exercise launches the actual Engine C# product runtime and proves a short
lifecycle, input-binding, projection, and fresh-process save/load scenario.
Do not restore Cargo/Rust, Node/pnpm/Nx/Angular, generated-protocol, old-host,
or broad browser/E2E workflows. Add a focused C# proof only for new C# behavior.

Update [docs/source-provenance.md](docs/source-provenance.md) when Engine or
content source selection changes, and
[docs/known-limitations.md](docs/known-limitations.md) when an intentional C#
phase boundary remains.
