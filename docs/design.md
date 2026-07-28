# Rusty D20 design

## Authority

Rusty D20 is one concrete d20 game. It owns game meaning and orchestration;
Rusty Engine owns reusable mechanisms. Neither Rusty Engine nor another game
depends on Rusty D20.

The intended durable path is:

```text
TypeScript authoring source
  -> immutable bounded d20 candidate artifact
  -> Rust d20 semantic compiler and admitted definitions
  -> registered entity components plus named Engine services
  -> Rusty D20 orchestration, persistence, and projection
  -> strict same-origin transport
  -> Angular store, features, and presentation
```

Only the final transport/UI portion exists in the bootstrap milestone. That
path is permanent rather than a GM7-only alternative.

## Runtime state

Rust owns authoritative state. `GameRuntime` currently contains canonical
Engine `EntityState`; future d20 facts attach as downstream registered
components. Mechanics such as stats, tracks, effects, inventory, equipment,
damage, and restoration are applied through direct Rusty Engine services.

TypeScript does not host live rules, callbacks, runtime sessions, or gameplay
state. Authored TypeScript will emit immutable candidates that Rust validates
and compiles before publication.

## Transport and protocol

`rusty-d20-host` serves the Angular build and `/api/v1/readout` from one origin.
Rust DTOs generate `libs/protocol/src/generated/api-types.ts`. The protocol
layer strictly decodes unknown JSON; transport classifies HTTP/network failure;
domain projects a view; store owns async UI state; features render it.

The diagnostic readout observes authoritative facts. It is not a second
authority, persistence format, or replay log.

## Dependencies

Rusty Engine packages use exact public Git revisions. There is no ordinary
sibling checkout dependency. Angular libraries follow the retained Nx boundary
graph in `boundaries.json`; production code cannot import testing fixtures.

## Persistence and execution

Complete-save ownership, authored definition persistence, action execution,
turn progression, and effect expiry remain Rusty D20 concerns. They will be
added without moving game meaning into Engine or browser code.
