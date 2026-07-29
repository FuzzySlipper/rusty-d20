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
import { catalogProbeModule } from './content/adventures/catalog_probe.js';
import { emberCastModule } from './content/adventures/ember_cast.js';
import { emberLoadoutModule } from './content/adventures/ember_loadout.js';
import { embersWakeModule } from './content/adventures/embers_wake.js';
import { wardenCastModule } from './content/adventures/warden_cast.js';
import { wardenLoadoutModule } from './content/adventures/warden_loadout.js';
import { wardensGateModule } from './content/adventures/wardens_gate.js';

export interface StarterArtifacts {
  readonly core: D20CanonicalArtifact;
  readonly steelGuard: D20CanonicalArtifact;
  readonly emberWard: D20CanonicalArtifact;
  readonly embersWake: D20CanonicalArtifact;
  readonly wardensGate: D20CanonicalArtifact;
  readonly catalogProbe: D20CanonicalArtifact;
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
  const steelGuard = authorD20Package({
    domain: 'rusty-d20',
    package: 'steel-guard',
    version: 1,
    dependencies: [exactCore],
    modules: [steelGuardModule],
  });
  const emberWard = authorD20Package({
    domain: 'rusty-d20',
    package: 'ember-ward',
    version: 1,
    dependencies: [exactCore],
    modules: [emberWardModule],
  });
  const wardensGate = authorD20Package({
    domain: 'rusty-d20',
    package: 'wardens-gate',
    version: 1,
    dependencies: [exactDependencyOn(steelGuard)],
    modules: [wardenCastModule, wardenLoadoutModule, wardensGateModule],
  });
  const embersWake = authorD20Package({
    domain: 'rusty-d20',
    package: 'embers-wake',
    version: 1,
    dependencies: [exactDependencyOn(emberWard)],
    modules: [emberCastModule, emberLoadoutModule, embersWakeModule],
  });
  return Object.freeze({
    core,
    steelGuard,
    emberWard,
    embersWake,
    wardensGate,
    catalogProbe: authorD20Package({
      domain: 'rusty-d20',
      package: 'catalog-probe',
      version: 1,
      dependencies: [exactDependencyOn(wardensGate)],
      modules: [catalogProbeModule],
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
          tags: ['attack', 'melee'],
          activationCosts: [{ budget: 'standard-action', amount: 1 }],
          target: {
            kind: 'participant',
            team: 'hostile',
            maximumTargets: 1,
            lineOfEffect: 'required',
          },
          attack: {
            kind: 'fixed',
            ability: 'might',
            defense: 'armor',
            damage: { kind: 'impact', dice: 1, sides: 4, bonus: 0 },
            range: 1,
          },
          effect: null,
          forcedMovement: 1,
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
