import { describe, expect, it } from 'vitest';
import { projectRuntimeReadout } from './index';

const SYNTHETIC_ENGINE_REVISION = '1111111111111111111111111111111111111111';

describe('projectRuntimeReadout', () => {
  it('projects a concise engine revision without hiding the exact value', () => {
    expect(
      projectRuntimeReadout({
        engineRevision: SYNTHETIC_ENGINE_REVISION,
        entityCount: 0,
        product: 'Rusty D20',
        status: 'ready',
        version: '0.1.0',
      }),
    ).toEqual({
      engineRevision: SYNTHETIC_ENGINE_REVISION,
      engineRevisionShort: '111111111111',
      entityCount: 0,
      product: 'Rusty D20',
      statusLabel: 'Runtime ready',
      version: '0.1.0',
    });
  });
});
