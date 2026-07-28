import type { Result, RuntimeReadoutDto } from '@rusty-d20/protocol';
import type { RustyD20Transport } from '@rusty-d20/transport';

export function makeRuntimeReadout(overrides: Partial<RuntimeReadoutDto> = {}): RuntimeReadoutDto {
  return {
    engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
    entityCount: 1,
    product: 'Rusty D20',
    status: 'ready',
    version: '0.1.0',
    ...overrides,
  };
}

export function createFakeRustyD20Transport(
  result: Result<RuntimeReadoutDto> = { ok: true, value: makeRuntimeReadout() },
): RustyD20Transport {
  return { loadReadout: async () => result };
}
