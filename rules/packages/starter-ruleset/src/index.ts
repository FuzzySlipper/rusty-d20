import {
  authorD20Package,
  defineD20Module,
  exactDependencyOn,
  type D20CanonicalArtifact,
} from '@rusty-d20/rules-authoring';

import { abilitiesModule } from './content/abilities.js';
import { emberWardModule } from './content/ember_ward.js';
import { fundamentalsModule } from './content/fundamentals.js';
import { invalidSemanticsModule } from './content/invalid.js';
import { steelGuardModule } from './content/steel_guard.js';

export interface StarterArtifacts {
  readonly core: D20CanonicalArtifact;
  readonly steelGuard: D20CanonicalArtifact;
  readonly emberWard: D20CanonicalArtifact;
  readonly invalidSemantics: D20CanonicalArtifact;
}

export function authorStarterArtifacts(): StarterArtifacts {
  const core = authorD20Package({
    domain: 'rusty-d20',
    package: 'starter-core',
    version: 1,
    modules: [abilitiesModule, fundamentalsModule],
  });
  const exactCore = exactDependencyOn(core);
  return Object.freeze({
    core,
    steelGuard: authorD20Package({
      domain: 'rusty-d20',
      package: 'steel-guard',
      version: 1,
      dependencies: [exactCore],
      modules: [steelGuardModule],
    }),
    emberWard: authorD20Package({
      domain: 'rusty-d20',
      package: 'ember-ward',
      version: 1,
      dependencies: [exactCore],
      modules: [emberWardModule],
    }),
    invalidSemantics: authorD20Package({
      domain: 'rusty-d20',
      package: 'invalid-semantics',
      version: 1,
      dependencies: [exactCore],
      modules: [invalidSemanticsModule],
    }),
  });
}

export function authorReorganizedCore(): D20CanonicalArtifact {
  return authorD20Package({
    domain: 'rusty-d20',
    package: 'starter-core',
    version: 1,
    modules: [fundamentalsModule, abilitiesModule],
  });
}

export function authorContentOnlyExtension(
  core: D20CanonicalArtifact,
): D20CanonicalArtifact {
  const extension = defineD20Module(
    {
      id: 'content-only-extension',
      path: 'rules/packages/starter-ruleset/src/index.ts',
    },
    ({ action }) => ({
      actions: [
        action(65, {
          id: 'shield-bash',
          ability: 'strength',
          defense: 'armor',
          damage: { kind: 'slashing', dice: 1, sides: 4, bonus: 0 },
          effect: null,
        }),
      ],
    }),
  );
  return authorD20Package({
    domain: 'rusty-d20',
    package: 'content-only-extension',
    version: 1,
    dependencies: [exactDependencyOn(core)],
    modules: [extension],
  });
}
