# Rules authoring

## Boundary

The `rules/` workspace is an isolated build-time toolchain. It is not part of
the Angular application graph and it is not needed by a headless Rust runtime.
It may execute ordinary TypeScript functions while producing artifacts, but no
function, callback, evaluator, mutable session, browser API, or transport is
persisted.

Rust owns the candidate schema, exact d20 limits, strict decode, semantic
diagnostics, compilation, and runtime. The generated file
`rules/packages/d20-authoring/src/generated.ts` must only be changed through:

```bash
pnpm --dir rules run generate:contract
```

The generic envelope, canonical writer, fingerprinting, sources, and
provenance come from Rusty Engine packages pinned to the reviewed Engine
revision.

## Package layout

- `@rusty-d20/rules-authoring` supplies source-aware typed builders, module
  composition, deterministic ordering, exact dependencies, diagnostic source
  mapping, and canonical emission.
- `@rusty-d20/starter-ruleset` owns concrete Rusty D20 content. It imports the
  SDK only through its package root.
- `rules/artifacts/starter/` contains canonical JSON packages and the
  fingerprint manifest consumed by Rust tests and later product loading.

Definition arrays are sorted by stable identity before emission. Engine
admission then canonicalizes dependencies, sources, provenance, object keys,
and bytes. Reordering modules or moving helper composition does not change a
package fingerprint; changing content or source provenance does.

## Add content

Define one source module and give each definition its authored line:

```ts
import { defineD20Module } from '@rusty-d20/rules-authoring';

export const example = defineD20Module(
  { id: 'example-content', path: 'rules/content/example.ts' },
  ({ action }) => ({
    actions: [
      action(7, {
        id: 'example-strike',
        ability: 'strength',
        defense: 'armor',
        damage: { kind: 'slashing', dice: 1, sides: 6, bonus: 0 },
        effect: null,
      }),
    ],
  }),
);
```

The callback executes immediately during authoring and is not stored. D20
identity syntax errors fail at the supplied source location. Valid identities
with invalid references, bounds, dice, durations, or cross-definition meaning
reach the Rust compiler and retain package/subject/source correlation.

Compose modules with `authorD20Package`. Use `exactDependencyOn` for fragments
that require another package's exact fingerprint. Regenerate and inspect:

```bash
pnpm --dir rules run generate
./scripts/verify-rules.sh
```

The focused gate checks generated-contract freshness, package isolation,
deterministic goldens, TypeScript tests, strict Rust canonical decode, both
starter compositions, fingerprints, and correlated invalid-content
diagnostics.
