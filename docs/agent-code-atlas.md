# Agent code atlas

| Path | Owner and purpose | Focused proof |
|---|---|---|
| `rust/crates/rusty-d20/src/lib.rs` | Public Rust surface, bootstrap runtime, DTOs, Engine pin readout | `cargo test -p rusty-d20 --locked` |
| `rust/crates/rusty-d20/src/candidate.rs` | Strict versioned d20 candidate types and neutral gameplay-rules envelope admission | `cargo test -p rusty-d20 --test d20r0 candidate --locked` |
| `rust/crates/rusty-d20/src/compiler.rs` | Downstream semantic validation, correlated diagnostics, immutable definitions, mechanics catalog compilation | `cargo test -p rusty-d20 --test d20r0 compiler --locked` |
| `rust/crates/rusty-d20/src/component.rs` | Durable d20 ability, resource, and caller-owned effect-schedule components | `cargo test -p rusty-d20 --test d20r0 --locked` |
| `rust/crates/rusty-d20/src/session.rs` | Atomic preview/reaction/apply, Engine inventory/equipment service adapter, deterministic rolls, explicit turn expiry, complete saves | `cargo test -p rusty-d20 --test d20r0 --locked` |
| `rust/crates/rusty-d20/src/game.rs` | Product campaign phases, fixed camp loadout policy, encounter lifecycle, opaque preview custody, projection, optimistic revisions, strict schema migration, bounded explanations, product save/reopen | `cargo test -p rusty-d20 --lib --locked` |
| `rust/crates/rusty-d20/src/identity.rs` | Bounded stable downstream definition identities | Rust tests and strict candidate decode |
| `rust/crates/rusty-d20/tests/d20r0.rs` | Headless semantic, failure-atomicity, persistence, provenance, and composition evidence | `cargo test -p rusty-d20 --test d20r0 --locked` |
| `rust/crates/rusty-d20/tests/d20a0.rs` | Node-free strict decode, starter composition, generated contract, fingerprint, and diagnostic-correlation proof | `cargo test -p rusty-d20 --test d20a0 --locked` |
| `rust/crates/rusty-d20/src/host.rs` | Same-origin HTTP/static host | Rust host tests and browser smoke |
| `rust/crates/rusty-d20/src/bin/` | Product host and protocol generator entrypoints | `pnpm run verify:rust` |
| `libs/protocol/` | Generated DTOs and strict unknown-JSON decode | `pnpm run protocol:check`; Vitest |
| `libs/platform/` | Browser/host ports and browser adapters | Typecheck and Vitest consumers |
| `libs/transport/` | Typed HTTP operations and failure classification | `http-transport.spec.ts` |
| `libs/domain/` | Pure Rust DTO-to-product-view projection | Domain Vitest |
| `libs/store/` | Angular async state, command orchestration, and stale-response guards | Store Vitest; boundary audit |
| `libs/feature-main-menu/` | Permanent landing, Engine-backed camp loadout/stash, encounter entry, action, status, and receipt feature | Playwright smoke/live evidence |
| `libs/ui-inventory`, `libs/ui-equipment` | Product-neutral accessible inventory/equipment widgets connected by the camp feature | Typecheck, lint, Playwright |
| `libs/components`, `libs/renderer`, remaining `libs/ui-*` | Retained product-neutral presentation building blocks | Typecheck, lint, later live consumers |
| `libs/shell/`, `apps/app/` | Routes and application composition | Build and Playwright |
| `libs/testing-fixtures/` | Explicit fake transport/readout helpers | Must never enter production graph |
| `rules/packages/d20-authoring/` | Isolated build-time d20 builders over Rust-generated types and exact Engine authoring packages | `pnpm --dir rules run verify` |
| `rules/packages/starter-ruleset/` | Multi-file Rusty D20 content and package compositions | Rules tests plus `d20a0` Rust test |
| `rules/artifacts/starter/` | Checked canonical packages and fingerprint manifest consumed by Node-free Rust | `pnpm --dir rules run generate:check` |
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
