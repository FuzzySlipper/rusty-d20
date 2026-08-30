# Rusty D20

Rusty D20 is a concrete C# d20 RPG and downstream consumer of the adjacent
Rusty Engine public C# SDK. It owns its rules, authored content, campaign
policy, saves, and projections; Rusty Engine supplies reusable lifecycle,
input, spatial, rendering, UI, random, and persistence mechanisms.

The current product contains clean-room C# compositions for **The Warden's
Gate** and **Ember's Wake**. The Engine product accepts explicit commands to
select an adventure, begin exploration, move, inspect, enter ordered
encounters, choose actions/targets/reactions, continue an outcome, and
save/load/reset. Its development host presents a small accessible control
surface and the published UI readout; it does not own gameplay.

## Prerequisite

Keep Rusty D20 beside the Engine checkout it should consume:

```text
workspace/
  rusty-d20/
  rusty-engine/
```

Rusty D20 uses the Engine checkout exactly as present. It never updates,
synchronizes, or copies the provider.

## Verify

```bash
dotnet run --project src/RustyD20.Core.Checks/RustyD20.Core.Checks.csproj
dotnet run --project src/RustyD20.Product.Checks/RustyD20.Product.Checks.csproj
dotnet build RustyD20.sln -c Release
dotnet publish src/RustyD20.NativeProduct/RustyD20.NativeProduct.csproj -c Release -r linux-x64
bash src/scripts/exercise-native-product.sh
```

The final command starts the actual NativeAOT product through the Engine host,
checks lifecycle and current input-binding admission, observes the published
frame/UI projection, and confirms save/load across a fresh process.

Read [docs/design.md](docs/design.md) for authority boundaries,
[docs/agent-code-atlas.md](docs/agent-code-atlas.md) for source ownership,
[docs/source-provenance.md](docs/source-provenance.md) for content provenance,
and [docs/known-limitations.md](docs/known-limitations.md) for deliberate
phase boundaries. [docs/csharp-migration-map.md](docs/csharp-migration-map.md)
is retained historical cutover evidence, not a second runtime.
