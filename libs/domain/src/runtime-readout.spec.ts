import { describe, expect, it } from 'vitest';
import { projectRuntimeReadout } from './index';

describe('projectRuntimeReadout', () => {
  it('projects a concise engine revision without hiding the exact value', () => {
    expect(projectRuntimeReadout({
      engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
      entityCount: 1,
      product: 'Rusty D20',
      status: 'ready',
      version: '0.1.0',
    })).toEqual({
      engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
      engineRevisionShort: 'fb608e323a8b',
      entityCount: 1,
      product: 'Rusty D20',
      statusLabel: 'Runtime ready',
      version: '0.1.0',
    });
  });
});
