# Rusty D20

Rusty D20 is a concrete C# d20 RPG and downstream consumer of the pinned
`Rusty.Engine` SDK package. It owns its rules, authored content, campaign
policy, saves, and projections; Rusty Engine supplies reusable lifecycle,
input, spatial, rendering, UI, random, and persistence mechanisms.

The current product contains clean-room C# compositions for **The Warden's
Gate** and **Ember's Wake**. The Engine product accepts explicit commands to
select an adventure, begin exploration, move, inspect, enter ordered
encounters, choose actions/targets/reactions, continue an outcome, and
save/load/reset. Its product-owned DOM UI presents a small accessible control
surface and the published UI readout; it does not own gameplay.

## Engine pair

`Directory.Build.props` pins the immutable Engine SDK/runtime pair. Get the
Engine's `rusty` command once with
`curl -fsSL https://raw.githubusercontent.com/FuzzySlipper/rusty-engine/main/scripts/install-rusty.sh | bash`,
then `rusty install` puts the pair in the shared cache; `rusty update` moves the
pin and lists the release notes to read. The package and runtime come from the
same Engine release and are not tracked product inputs. An Engine contributor
may use the explicit `rusty dev --engine-source /absolute/rusty-engine`
override; ordinary product development never discovers an adjacent checkout.

## Verify

```bash
export $(rusty env)
dotnet run --project src/RustyD20.Core.Checks/RustyD20.Core.Checks.csproj
dotnet run --project src/RustyD20.Product.Checks/RustyD20.Product.Checks.csproj
dotnet build RustyD20.sln -c Release
rusty dev --project src/RustyD20.Product/RustyD20.Product.csproj
dotnet msbuild src/RustyD20.Product/RustyD20.Product.csproj -t:VerifyRustyEngineAot -p:Configuration=Release
```

`rusty dev` is the normal CoreCLR host/staging command. The final command is
the separate NativeAOT fidelity/release check; the immutable SDK generates its
composition below `obj`.

The tracked `.den-serve.json` uses that same explicit CoreCLR command for
broker-owned local sessions. It requires the provisioned `.runtime` pair and
never falls back to an adjacent Engine checkout or a product-built host.

Read [docs/design.md](docs/design.md) for authority boundaries,
[docs/agent-code-atlas.md](docs/agent-code-atlas.md) for source ownership,
[docs/source-provenance.md](docs/source-provenance.md) for content provenance,
and [docs/known-limitations.md](docs/known-limitations.md) for deliberate
phase boundaries. [docs/csharp-migration-map.md](docs/csharp-migration-map.md)
is retained historical cutover evidence, not a second runtime.
