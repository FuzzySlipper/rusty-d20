# Verification

All maintained verification is C#-product focused. Rust, Cargo, Node/pnpm,
Nx/Angular, generated-protocol, and browser/E2E workflows are retired.

```bash
dotnet run --project src/RustyD20.Core.Checks/RustyD20.Core.Checks.csproj
dotnet run --project src/RustyD20.Product.Checks/RustyD20.Product.Checks.csproj
dotnet build RustyD20.sln -c Release
./.runtime/runtime-pack-cabba0f/bin/rusty dev --project src/RustyD20.Product/RustyD20.Product.csproj --runtime ./.runtime/runtime-pack-cabba0f
dotnet msbuild src/RustyD20.Product/RustyD20.Product.csproj -t:VerifyRustyEngineAot -p:Configuration=Release
```

The Core checks cover C# content and semantic/session/campaign/persistence
behavior. Product checks cover the Engine-facing product boundary. `rusty dev`
is the ordinary CoreCLR proof: it builds and atomically stages the loose
product, then launches the matching packaged host. The separate MSBuild target
generates and publishes the NativeAOT module below `obj` as a fidelity/release
check.

The ignored local SDK feed and runtime pack are a matched pair. Missing or
mismatched artifacts are provisioning failures, not reasons to compile or host
against a discovered Engine checkout.
