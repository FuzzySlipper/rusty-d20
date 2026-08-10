import { describe, expect, it } from 'vitest';
import { projectRuntimeReadout } from './index';

describe('projectRuntimeReadout', () => {
  it('projects a concise engine revision without hiding the exact value', () => {
    expect(
      projectRuntimeReadout({
        entityCount: 0,
        product: 'Rusty D20',
        status: 'ready',
        version: '0.1.0',
      }),
    ).toEqual({
      entityCount: 0,
      product: 'Rusty D20',
      statusLabel: 'Runtime ready',
      version: '0.1.0',
    });
  });
});
