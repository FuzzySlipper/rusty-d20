# C# migration record

**Status:** historical cutover evidence, updated for the packaged SDK workflow.
The executable authority is the current C# source and the concise documents
linked from the repository README. This record is not a second launch path.

## Retained product authority

`RustyD20.Core` owns d20 vocabulary, authored-content interpretation,
semantic admission, session/campaign/tactical policy, action resolution, save
meaning, tuning, receipts, and bounded projections. `RustyD20.Product` is the
one ordinary `IEngineProduct` root and translates admitted input into those
product decisions.

Rusty Engine owns reusable lifecycle, input, renderer resources, retained
presentation, spatial/navigation/collision, deterministic random, content,
UI publication, and persistence primitives. Rusty D20 does not add a second
loop, P/Invoke layer, unsafe ABI declaration, browser renderer, generic RPG
framework, or local replacement for an Engine mechanism.

The clean-room C# content preserves the foundation, Steel Guard, Ember Ward,
The Warden's Gate, and Ember's Wake as named product facts with provenance.
The prior Rust and TypeScript paths remain historical research evidence only;
they are not active inputs or compatibility routes.

## Current downstream contract

The normal product contract is intentionally small and exact:

```text
Rusty.Engine 0.1.0-dev.cbf35130d06c from .runtime/sdk-feed
  + runtime-pack-cbf35130d06c from .runtime/runtime-pack-cbf35130d06c
  -> rusty dev CoreCLR build, atomic staging, and packaged host
  -> optional VerifyRustyEngineAot fidelity/release module
```

The package and runtime pack share the `cbf35130d06c` Engine release identity.
They are ignored local provisioning artifacts; `NuGet.Config` selects the SDK
feed and the product project pins the package version. The runtime pack stays
explicit on every `rusty dev` invocation, so ordinary builds never discover a
sibling checkout or assemble a host themselves.

An Engine contributor can select source deliberately with:

```bash
rusty dev --engine-source /absolute/rusty-engine ...
```

That SDK-owned override conditionally replaces package compile/runtime assets.
It is not ordinary downstream setup and must not be encoded as a local default.

## Cutover disposition

The following superseded families were removed once the C# product owned the
vertical path:

| Retired family | Current owner/path |
| --- | --- |
| Handwritten NativeAOT product leaf and generator references | The package generates CoreCLR and NativeAOT composition under `obj`. |
| Product-built browser host, copied Engine browser assets, and runtime adapter | The runtime pack owns the browser shell and renderer; `ui/main.js` is product-owned DOM UI only. |
| Product Cargo host build and native exercise runner | `rusty dev` stages/runs CoreCLR; `VerifyRustyEngineAot` is the separate fidelity check. |
| Direct Engine project references and checkout-selected dependency model | Pinned `PackageReference` plus `NuGet.Config` local feed. |
| Retired Rust/Node/Nx/Angular/protocol execution paths | Inspectable C# Core/Product source and focused C# checks. |

The current product UI and `content/product-spine.json` were retained as
product-owned staged inputs. Product checks remain focused lifecycle/input/UI
projection proof; Core checks retain content, semantic, campaign, tactical,
and persistence proof. The active-tactical fresh-process restore limitation is
recorded in [known limitations](known-limitations.md), rather than hidden by
the host migration.

## Current focused commands

```bash
dotnet run --project src/RustyD20.Core.Checks/RustyD20.Core.Checks.csproj
dotnet run --project src/RustyD20.Product.Checks/RustyD20.Product.Checks.csproj
dotnet build RustyD20.sln -c Release
./.runtime/runtime-pack-cbf35130d06c/bin/rusty dev --project src/RustyD20.Product/RustyD20.Product.csproj --runtime ./.runtime/runtime-pack-cbf35130d06c
dotnet msbuild src/RustyD20.Product/RustyD20.Product.csproj -t:VerifyRustyEngineAot -p:Configuration=Release
```

The first four commands establish ordinary CoreCLR development and staging;
the last one is intentionally separate NativeAOT fidelity/release proof.
