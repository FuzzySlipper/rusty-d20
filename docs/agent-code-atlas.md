# Agent code atlas

| Path | Owner and purpose | Focused proof |
|---|---|---|
| `rust/crates/rusty-d20/src/lib.rs` | Public Rust surface, bootstrap runtime, DTOs, Engine pin readout | `cargo test -p rusty-d20 --locked` |
| `rust/crates/rusty-d20/src/candidate.rs` | Strict versioned d20 candidate types and neutral gameplay-rules envelope admission | `cargo test -p rusty-d20 --test d20r0 candidate --locked` |
| `rust/crates/rusty-d20/src/compiler.rs` | Downstream semantic validation, correlated diagnostics, immutable definitions, mechanics catalog compilation | `cargo test -p rusty-d20 --test d20r0 compiler --locked` |
| `rust/crates/rusty-d20/src/component.rs` | Durable d20 ability, resource, and caller-owned effect-schedule components | `cargo test -p rusty-d20 --test d20r0 --locked` |
| `rust/crates/rusty-d20/src/session.rs` | Atomic preview/reaction/apply, deterministic rolls, explicit turn expiry, complete saves | `cargo test -p rusty-d20 --test d20r0 --locked` |
| `rust/crates/rusty-d20/src/identity.rs` | Bounded stable downstream definition identities | Rust tests and strict candidate decode |
| `rust/crates/rusty-d20/tests/d20r0.rs` | Headless semantic, failure-atomicity, persistence, provenance, and composition evidence | `cargo test -p rusty-d20 --test d20r0 --locked` |
| `rust/crates/rusty-d20/src/host.rs` | Same-origin HTTP/static host | Rust host tests and browser smoke |
| `rust/crates/rusty-d20/src/bin/` | Product host and protocol generator entrypoints | `pnpm run verify:rust` |
| `libs/protocol/` | Generated DTOs and strict unknown-JSON decode | `pnpm run protocol:check`; Vitest |
| `libs/platform/` | Browser/host ports and browser adapters | Typecheck and Vitest consumers |
| `libs/transport/` | Typed HTTP operations and failure classification | `http-transport.spec.ts` |
| `libs/domain/` | Pure readout-to-view projection | Domain Vitest |
| `libs/store/` | Angular async state and production providers | Store Vitest; boundary audit |
| `libs/feature-main-menu/` | Permanent bootstrap/product entry feature | Playwright smoke/live evidence |
| `libs/components`, `libs/renderer`, `libs/ui-*` | Retained product-neutral presentation building blocks | Typecheck, lint, later live consumers |
| `libs/shell/`, `apps/app/` | Routes and application composition | Build and Playwright |
| `libs/testing-fixtures/` | Explicit fake transport/readout helpers | Must never enter production graph |
| `apps/app-e2e/` | Real-host browser smoke and opt-in evidence collector | `pnpm run verify:browser` |
| `tools/scripts/` | Generated boundaries, product audit, live broker | `pnpm run verify:boundaries` |
| `docs/` | Architecture, provenance, limitations, extension, verification | `pnpm run check:docs` |

Before adding a new library, update `boundaries.json`, regenerate
`eslint.config.mjs`, expose one package-root barrel, and add the narrow owning
test. Do not work from this atlas instead of the owning design or executable
contract.
