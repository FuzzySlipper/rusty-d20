# Rusty D20 design

## Authority

Rusty D20 is one concrete C# d20 product. `RustyD20.Core` owns d20 meaning:
definitions, semantic admission, sessions, campaign/exploration/tactical
policy, strict saves, receipts, and bounded projections. `RustyD20.Product`
adapts those facts to the SDK-generated Engine `IEngineProduct` contract. It
is the sole product root; the SDK generates both CoreCLR and NativeAOT
composition below ignored `obj` paths.

Rusty Engine owns reusable mechanisms: product lifecycle and admitted input,
spatial/navigation/collision, retained scene resources and camera scheduling,
deterministic random, UI streams, and storage primitives. Rusty D20 uses only
the public adjacent C# SDK. It creates no second update loop, P/Invoke layer,
unsafe ABI declaration, copied Engine implementation, generic RPG framework,
or browser-side rules evaluator. Ordinary development consumes the pinned
immutable package/runtime pair; Engine source is an explicit contributor
override, never adjacent-checkout discovery.

```text
C# authored content modules
  -> strict D20 semantic compiler and composition fingerprint
  -> Engine-backed session and campaign policy
  -> bounded receipts and UI/surface projections
  -> generated IEngineProduct callbacks
  -> Engine host and observational development page
```

## Product meaning

`D20ContentCatalog` contains inspectable C# modules for the six attributes,
four defenses, Steel Guard and Ember Ward packages, and the Warden's Gate and
Ember's Wake adventures. Each value carries a source-provenance record. The
old source paths are evidence for the clean-room adaptation only; no Rust,
TypeScript, generated catalog, or JSON candidate is an active input.

`D20SemanticCompiler` rejects malformed or non-current content and produces a
stable composition fingerprint. The C# schemas are closed. Legacy Rust
candidate/session/save schemas and unknown schemas are rejected, with no
compatibility migration or defaulting.

`D20Session` owns seeded/static roll policy, action preview fences, staged
resolution, durable component facts, and receipts. `D20CampaignRuntime` owns
camp, exploration, encounter, outcome, and completion policy, including
ordered encounter admission, landmarks, treasures, doors, checkpoints, and
save meaning. `TacticalEncounter` applies D20 target, range, initiative,
movement, reaction, and bounded opposition policy over Engine spatial facts.

## Product integration and observability

`RustyD20Product` accepts only current direct-UI input events that match the
active Engine binding and `gameplay.default` context. It publishes a bounded,
structured UI stream plus a retained Engine surface. The small Engine-hosted
development page can translate controls and keyboard input into those declared
intents and display observations, but it cannot calculate rules, mutate saves,
or render authoritative state.

Save/load uses Engine durable storage while Rusty D20 defines the closed save
schema, content fingerprint, validation, and reset policy. Restore constructs
a fresh product candidate and rejects invalid state before publication.

## Modularity rules

- Keep new game meaning in the smallest owning Core domain; do not add a
  command bus, event bus, service locator, hidden scheduler, or universal AST.
- Keep content values as named C# modules with provenance and limits rather
  than reintroducing an executable authoring pipeline.
- Ask Engine for reusable spatial, rendering, input, random, and persistence
  mechanisms; do not recreate them downstream.
- Keep projections observational and bounded. A projection explains committed
  facts; it never becomes a second command or rules authority.
