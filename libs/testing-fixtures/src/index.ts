import type {
  GameSnapshotDto,
  Result,
  RuntimeReadoutDto,
  SaveStatusDto,
} from '@rusty-d20/protocol';
import type { RustyD20Transport } from '@rusty-d20/transport';

export function makeRuntimeReadout(overrides: Partial<RuntimeReadoutDto> = {}): RuntimeReadoutDto {
  return {
    engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
    entityCount: 0,
    product: 'Rusty D20',
    status: 'ready',
    version: '0.1.0',
    ...overrides,
  };
}

export function makeGameSnapshot(overrides: Partial<GameSnapshotDto> = {}): GameSnapshotDto {
  return {
    product: 'Rusty D20',
    version: '0.1.0',
    engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
    rulesetFingerprint: 'starter-core=fingerprint|steel-guard=fingerprint',
    revision: 0,
    saved: false,
    availableAdventures: [
      {
        id: 'wardens-gate',
        title: "The Warden's Gate",
        summary: "Mara Venn prepares at the Warden's Gate camp.",
        details: ['Starter Core + Steel Guard authored packages compiled by Rust.'],
      },
      {
        id: 'embers-wake',
        title: "Ember's Wake",
        summary: 'Sera Vale prepares beside the ember reliquary.',
        details: ['Starter Core + Ember Ward authored packages compiled by Rust.'],
      },
    ],
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
  const saveStatus: SaveStatusDto = {
    saveIdentity: '/fixture/rusty-d20.json',
    state: 'empty',
    campaignId: null,
    campaignTitle: null,
    revision: 0,
    persistenceError: null,
  };
  return {
    loadReadout: async () => readoutResult,
    loadSession: async () => sessionResult,
    loadSaveStatus: async () => ({ ok: true, value: saveStatus }),
    resetSession: async () => sessionResult,
    newAdventure: async () => sessionResult,
    enterEncounter: async () => sessionResult,
    equipItem: async () => sessionResult,
    unequipItem: async () => sessionResult,
    transferItem: async () => sessionResult,
    previewAction: async () => sessionResult,
    applyReaction: async () => sessionResult,
    applyAction: async () => sessionResult,
    beginOppositionTurn: async () => sessionResult,
    returnToCamp: async () => sessionResult,
    save: async () => sessionResult,
  };
}
