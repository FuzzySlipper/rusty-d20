# Verification

All maintained verification is C#-product focused. Rust, Cargo, Node/pnpm,
Nx/Angular, generated-protocol, and browser/E2E workflows are retired.

```bash
dotnet run --project src/RustyD20.Core.Checks/RustyD20.Core.Checks.csproj
dotnet run --project src/RustyD20.Product.Checks/RustyD20.Product.Checks.csproj
dotnet build RustyD20.sln -c Release
dotnet publish src/RustyD20.NativeProduct/RustyD20.NativeProduct.csproj -c Release -r linux-x64
bash src/scripts/exercise-native-product.sh
```

The Core checks cover C# content and semantic/session/campaign/persistence
behavior. Product checks cover the Engine-facing product boundary. Release
build and NativeAOT publish prove the supported deliverable. The native
exercise starts the real Engine C# product runtime, checks lifecycle and input
binding fences, observes the published frame/UI state, saves, restarts, and
loads the same product state.

The adjacent Engine checkout is a prerequisite and is never modified by these
commands. The exercise may prepare its own disposable run directory and removes
it unless `D20_EXERCISE_KEEP_RUN_DIR=1` is set for diagnosis.
