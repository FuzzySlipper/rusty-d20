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
| `src/RustyD20.Product/` | `IEngineProduct` orchestration, current input claim fence, UI projection, and Engine surface adaptation. | Product checks |
| `src/RustyD20.NativeProduct/` | NativeAOT product selection and minimal observational development-host assets. | publish and native exercise |
| `src/RustyD20.Core.Checks/` | Focused content, semantic, session, campaign, and persistence proofs. | `dotnet run --project src/RustyD20.Core.Checks/RustyD20.Core.Checks.csproj` |
| `src/RustyD20.Product.Checks/` | Focused Engine-product lifecycle, input, projection, and disposal proofs. | `dotnet run --project src/RustyD20.Product.Checks/RustyD20.Product.Checks.csproj` |
| `src/scripts/exercise-native-product.sh` | Actual Engine-host lifecycle and fresh-process save/load exercise. | `bash src/scripts/exercise-native-product.sh` |
| `docs/csharp-migration-map.md` | Historical source disposition and cutover intent; not an active dependency. | Review with current source |

The adjacent `../rusty-engine` checkout is a provider, not part of this source
tree. Consult its public C# SDK documentation and projects without mutating it.
