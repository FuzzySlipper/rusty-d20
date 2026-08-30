# Adjacent Rusty Engine dependency

Rusty D20 consumes the local `../rusty-engine` checkout through public C#
project references to `csharp/Rusty.Engine` and the Engine product generator.
The checkout must sit beside this repository:

```text
workspace/
  rusty-d20/
  rusty-engine/
```

The operator selects the Engine revision. Rusty D20 compiles against that
checkout exactly as it stands and has no pin manifest, synchronizer, update
script, copied provider implementation, handwritten interop, or runtime
fallback. Do not pull, reset, or otherwise mutate the Engine checkout from
this repository.

Core consumes Engine managed mechanisms where appropriate; Product consumes
the generated `IEngineProduct` contract and public services; NativeProduct
adds the Engine generator analyzer for NativeAOT composition. Engine revision
identity is not a D20 save field.

Use the maintained C# checks, build, publish, and native exercise listed in
[verification.md](verification.md). If the adjacent SDK lacks a reusable
mechanism, adapt D20 policy or route the reusable gap upstream rather than
adding a local substitute.
