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
- `rules/artifacts/starter/` contains canonical JSON packages, the fingerprint
  manifest, and `catalog.json`. The catalog embeds the canonical runtime
  packages; it intentionally excludes negative-test artifacts.

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

## Add an adventure

Adventure content uses the same source-aware builders. Keep cohesive authored
facts in separate modules, as the current example does:

```text
content/adventures/warden_cast.ts       character templates
content/adventures/warden_loadout.ts    storage and item instances
content/adventures/wardens_gate.ts      encounter, outcomes, reward, adventure
content/adventures/ember_cast.ts        alternate character templates
content/adventures/ember_loadout.ts     alternate storage and item instances
content/adventures/embers_wake.ts       alternate encounter, reward, adventure
content/adventures/catalog_probe.ts     content-only composition proof
```

Register the modules in a concrete package in
`starter-ruleset/src/index.ts`, use `exactDependencyOn` for its prerequisite
rules package, add the artifact to `generate-artifacts.mjs`, and regenerate.
Rust discovers the adventure owner from the catalog, exposes only entries
authored as selectable, and compiles only the requested exact dependency
closure before campaign publication. Adding another adventure with the
existing vocabulary, or changing names, character scores, inventory,
equipment, explanations, availability, or outcome content, does not require
edits to `game.rs`, `session.rs`, the semantic compiler, or Rusty Engine. New
semantic behavior still belongs in the Rust candidate and compiler.

The encounter identities in an adventure are an authored ordered sequence.
Warden's Gate demonstrates two entries and a repeated opponent without adding
runtime TypeScript: Rust admits only the next incomplete encounter, persists
the completed prefix, restores bounded returning vitality, and preserves prior
resources, effects, loadout, and reward state.

The focused gate checks generated-contract freshness, package isolation,
deterministic goldens, TypeScript tests, strict Rust canonical decode, package
and catalog fingerprints, Node-free adventure compilation, and correlated
invalid-content diagnostics.
