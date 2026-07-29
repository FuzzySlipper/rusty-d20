# Agent code atlas

| Path | Owner and purpose | Focused proof |
|---|---|---|
| `rust/crates/rusty-d20/src/lib.rs` | Public Rust surface, bootstrap runtime, DTOs, Engine pin readout | `cargo test -p rusty-d20 --locked` |
| `rust/crates/rusty-d20/src/candidate.rs` | Strict versioned d20 candidate types, including authored dungeon topology/placements, and neutral gameplay-rules envelope admission | `cargo test -p rusty-d20 --test d20r0 candidate --locked` |
| `rust/crates/rusty-d20/src/compiler.rs` | Downstream semantic validation, bounded dungeon reachability/placement checks, correlated diagnostics, immutable definitions, mechanics catalog compilation | `cargo test -p rusty-d20 --test d20r0 compiler --locked` |
| `rust/crates/rusty-d20/src/adventure.rs` | Strict embedded authored catalog, selectable projection, default identity, and exact dependency-closure compilation | `cargo test -p rusty-d20 --lib adventure --locked` |
| `rust/crates/rusty-d20/src/component.rs` | Durable d20 ability, resource, encounter participation/position, activation-budget, and caller-owned effect-schedule components | `cargo test -p rusty-d20 --test d20r0 --locked` |
| `rust/crates/rusty-d20/src/session.rs` | Atomic preview/reaction/apply, Engine inventory/equipment service adapter, deterministic rolls, explicit turn expiry, complete saves | `cargo test -p rusty-d20 --test d20r0 --locked` |
| `rust/crates/rusty-d20/src/game.rs` | Product command orchestration: atomic adventure selection, ordered encounter progression, campaign phases, deterministic opposition/turn policy, outcomes, opaque preview custody, and optimistic revisions | `cargo test -p rusty-d20 --lib --locked` |
| `rust/crates/rusty-d20/src/game/exploration.rs` | Authoritative grid turns/steps/collisions, discovery, landmarks, encounter triggers, and bounded first-person projection | `cargo test -p rusty-d20 --lib dungeon_exploration --locked` |
| `rust/crates/rusty-d20/src/game/tactical.rs` | Direct Engine volume/spatial/pathfinding/collision adapter for tactical routes, range, line of effect, and forced movement | `cargo test -p rusty-d20 --lib tactical --locked` |
| `rust/crates/rusty-d20/src/game/dto.rs` | Generated-protocol DTO ownership | `pnpm run protocol:check` |
| `rust/crates/rusty-d20/src/game/content.rs` | Admitted content-to-session seeds, loadout/reward adaptation, and product-state validation | `cargo test -p rusty-d20 --lib --locked` |
| `rust/crates/rusty-d20/src/game/persistence.rs` | Strict product-schema-9/session-schema-4 save/reopen with exploration and tactical state plus exact composition/history binding | `cargo test -p rusty-d20 --lib --locked` |
| `rust/crates/rusty-d20/src/game/projection.rs` | Immutable campaign, exploration, encounter, character, loadout, and receipt projections | `cargo test -p rusty-d20 --lib --locked` |
| `rust/crates/rusty-d20/src/game/tests.rs` | Product orchestration, persistence, atomicity, and catalog regressions | `cargo test -p rusty-d20 --lib --locked` |
| `rust/crates/rusty-d20/src/identity.rs` | Bounded stable downstream definition identities | Rust tests and strict candidate decode |
| `rust/crates/rusty-d20/tests/d20r0.rs` | Headless semantic, failure-atomicity, persistence, provenance, and composition evidence | `cargo test -p rusty-d20 --test d20r0 --locked` |
| `rust/crates/rusty-d20/tests/d20a0.rs` | Node-free strict decode, starter composition, generated contract, fingerprint, and diagnostic-correlation proof | `cargo test -p rusty-d20 --test d20a0 --locked` |
| `rust/crates/rusty-d20/src/host.rs` | Same-origin HTTP/static host, save identity/status, malformed-save recovery, atomic persistence, and guarded reset | Rust host tests and browser smoke |
| `rust/crates/rusty-d20/src/bin/` | Product host and protocol generator entrypoints | `pnpm run verify:rust` |
| `libs/protocol/` | Generated DTOs and strict unknown-JSON decode | `pnpm run protocol:check`; Vitest |
| `libs/platform/` | Browser/host ports and browser adapters | Typecheck and Vitest consumers |
| `libs/transport/` | Typed HTTP operations and failure classification | `http-transport.spec.ts` |
| `libs/domain/` | Pure Rust DTO-to-product-view projection | Domain Vitest |
| `libs/store/` | Angular async state, command orchestration, and stale-response guards | Store Vitest; boundary audit |
| `libs/feature-main-menu/` | Permanent landing, save recovery/reset, Engine-backed camp, first-person exploration/movement, campaign progress, modal encounter, player/opposition turns, outcome, status, and receipt feature | Playwright smoke/live evidence |
| `libs/ui-inventory`, `libs/ui-equipment` | Product-neutral accessible inventory/equipment widgets connected by the camp feature | Typecheck, lint, Playwright |
| `libs/components`, `libs/renderer`, `libs/ui-compass`, `libs/ui-minimap` | Product-neutral presentation building blocks; renderer owns the first-person corridor and overhead tactical board while compass/minimap consume only Rust-projected navigation facts | Typecheck, lint, Playwright |
| `libs/shell/`, `apps/app/` | Routes and application composition | Build and Playwright |
| `libs/testing-fixtures/` | Explicit fake transport/readout helpers | Must never enter production graph |
| `rules/packages/d20-authoring/` | Isolated build-time d20 builders over Rust-generated types and exact Engine authoring packages | `pnpm --dir rules run verify` |
| `rules/packages/starter-ruleset/` | Multi-file Rusty D20 content and package compositions | Rules tests plus `d20a0` Rust test |
| `rules/artifacts/starter/` | Checked canonical packages, fingerprint manifest, and runtime catalog consumed by Node-free Rust | `pnpm --dir rules run generate:check` |
| `rules/scripts/` | Artifact generation and authoring/runtime/browser isolation audit | `pnpm --dir rules run boundary` |
| `apps/app-e2e/` | Real-host browser smoke and opt-in evidence collector | `pnpm run verify:browser` |
| `tools/scripts/` | Generated boundaries, product audit, live broker | `pnpm run verify:boundaries` |
| `docs/` | Architecture, provenance, limitations, extension, verification | `pnpm run check:docs` |

Before adding a new library, update `boundaries.json`, regenerate
`eslint.config.mjs`, expose one package-root barrel, and add the narrow owning
test. Do not work from this atlas instead of the owning design or executable
contract.

The isolated `rules/` workspace is not an Angular/Nx library graph. Add its
packages through `rules/pnpm-workspace.yaml`, keep package-root imports, and
run `./scripts/verify-rules.sh`.
