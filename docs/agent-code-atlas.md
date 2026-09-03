# Agent code atlas

| Path | Owner and purpose | Focused proof |
| --- | --- | --- |
| `RustyD20.sln` | Current C# solution boundary. | `dotnet build RustyD20.sln -c Release` |
| `src/RustyD20.Core/Contract/` | Closed schemas, stable IDs, limits, diagnostics, and provenance records. | Core checks |
| `src/RustyD20.Core/Content/D20ContentCatalog.cs` | Inspectable clean-room C# content for both adventures. | Core checks |
| `src/RustyD20.Core/Rules/` | Definition model, semantic admission, and composition fingerprints. | Core checks |
| `src/RustyD20.Core/Session/` | Engine-backed session, roll policy, action/reaction resolution, receipts, and durable session facts. | Core checks |
| `src/RustyD20.Core/Campaign/` | Adventure selection, camp/exploration/outcome policy, strict campaign save state, and Engine spatial gateway. | Core checks |
| `src/RustyD20.Core/Tactical/` | D20 tactical admission and bounded opposition policy over Engine spatial mechanisms. | Core checks |
| `src/RustyD20.Core/Persistence/` | Closed C# save codec and Engine storage adapter. | Core checks |
| `src/RustyD20.Product/` | The single SDK-declared `IEngineProduct` root, input fence, UI projection, and Engine surface adaptation. | Product checks and CoreCLR staging |
| `ui/` and `content/` | Product-owned DOM UI and declared content staged by the SDK. | `rusty dev` |
| `src/RustyD20.Core.Checks/` | Focused content, semantic, session, campaign, and persistence proofs. | `dotnet run --project src/RustyD20.Core.Checks/RustyD20.Core.Checks.csproj` |
| `src/RustyD20.Product.Checks/` | Focused Engine-product lifecycle, input, projection, and disposal proofs. | `dotnet run --project src/RustyD20.Product.Checks/RustyD20.Product.Checks.csproj` |
| `.runtime/` | Ignored exact SDK feed and matched runtime pack provisioned locally. | `rusty dev` and restore |
| `docs/csharp-migration-map.md` | Historical cutover evidence; not an active dependency or launch path. | Review with current source |

`NuGet.Config` resolves the pinned SDK from `.runtime/sdk-feed`. The runtime
pack is selected explicitly by `rusty dev`; no Engine checkout is part of the
ordinary source tree or build path.
