import type {
  GameSnapshotDto,
  Result,
  RuntimeReadoutDto,
} from '@rusty-d20/protocol';
import type { RustyD20Transport } from '@rusty-d20/transport';

export function makeRuntimeReadout(
  overrides: Partial<RuntimeReadoutDto> = {},
): RuntimeReadoutDto {
  return {
    engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
    entityCount: 0,
    product: 'Rusty D20',
    status: 'ready',
    version: '0.1.0',
    ...overrides,
  };
}

export function makeGameSnapshot(
  overrides: Partial<GameSnapshotDto> = {},
): GameSnapshotDto {
  return {
    product: 'Rusty D20',
    version: '0.1.0',
    engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
    rulesetFingerprint: 'starter-core=fingerprint|steel-guard=fingerprint',
    revision: 0,
    saved: false,
    campaign: null,
    encounter: null,
    ...overrides,
  };
}

export function createFakeRustyD20Transport(
  readoutResult: Result<RuntimeReadoutDto> = {
    ok: true,
    value: makeRuntimeReadout(),
  },
  sessionResult: Result<GameSnapshotDto> = {
    ok: true,
    value: makeGameSnapshot(),
  },
): RustyD20Transport {
  return {
    loadReadout: async () => readoutResult,
    loadSession: async () => sessionResult,
    newAdventure: async () => sessionResult,
    enterEncounter: async () => sessionResult,
    previewAction: async () => sessionResult,
    applyReaction: async () => sessionResult,
    applyAction: async () => sessionResult,
    advanceTurn: async () => sessionResult,
    save: async () => sessionResult,
  };
}
