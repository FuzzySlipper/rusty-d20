import type { RuntimeReadoutDto } from '@rusty-d20/protocol';

export interface RuntimeReadoutView {
  readonly product: string;
  readonly version: string;
  readonly engineRevision: string;
  readonly engineRevisionShort: string;
  readonly statusLabel: string;
  readonly entityCount: number;
}

export function projectRuntimeReadout(readout: RuntimeReadoutDto): RuntimeReadoutView {
  return {
    product: readout.product,
    version: readout.version,
    engineRevision: readout.engineRevision,
    engineRevisionShort: readout.engineRevision.slice(0, 12),
    statusLabel: readout.status === 'ready' ? 'Runtime ready' : 'Runtime unavailable',
    entityCount: readout.entityCount,
  };
}
