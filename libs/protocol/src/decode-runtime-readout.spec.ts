import { describe, expect, it } from 'vitest';
import { decodeRuntimeReadout } from './index';

const validReadout = {
  engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
  entityCount: 1,
  product: 'Rusty D20',
  status: 'ready',
  version: '0.1.0',
};

describe('decodeRuntimeReadout', () => {
  it('accepts the exact Rust-owned shape', () => {
    expect(decodeRuntimeReadout(validReadout)).toEqual({ ok: true, value: validReadout });
  });

  it('rejects unknown fields and unsafe counts', () => {
    expect(decodeRuntimeReadout({ ...validReadout, semanticStatus: 'pretend' })).toMatchObject({ ok: false });
    expect(decodeRuntimeReadout({ ...validReadout, entityCount: Number.MAX_SAFE_INTEGER + 1 })).toMatchObject({ ok: false });
  });
});
