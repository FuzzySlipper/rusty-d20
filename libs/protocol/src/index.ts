export * from './generated/api-types';

import type { RuntimeReadoutDto } from './generated/api-types';

export type ClassifiedError =
  | { readonly kind: 'network'; readonly message: string; readonly retryable: true }
  | { readonly kind: 'unauthorized'; readonly message: string; readonly retryable: false }
  | { readonly kind: 'not-found'; readonly message: string; readonly retryable: false }
  | { readonly kind: 'unknown'; readonly message: string; readonly retryable: false };

export type Result<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: ClassifiedError };

export const unknownError = (message: string): ClassifiedError => ({
  kind: 'unknown',
  message,
  retryable: false,
});

export function decodeRuntimeReadout(value: unknown): Result<RuntimeReadoutDto> {
  if (!isRecord(value)) {
    return { ok: false, error: unknownError('Runtime readout must be an object.') };
  }

  const expectedKeys = ['engineRevision', 'entityCount', 'product', 'status', 'version'];
  const actualKeys = Object.keys(value).sort();
  if (actualKeys.length !== expectedKeys.length || actualKeys.some((key, index) => key !== expectedKeys[index])) {
    return { ok: false, error: unknownError('Runtime readout has an unexpected shape.') };
  }

  const product = value['product'];
  const version = value['version'];
  const engineRevision = value['engineRevision'];
  const status = value['status'];
  const entityCount = value['entityCount'];
  if (
    typeof product !== 'string' ||
    typeof version !== 'string' ||
    typeof engineRevision !== 'string' ||
    status !== 'ready' ||
    typeof entityCount !== 'number' ||
    !Number.isSafeInteger(entityCount) ||
    entityCount < 0
  ) {
    return { ok: false, error: unknownError('Runtime readout contains invalid values.') };
  }

  return {
    ok: true,
    value: { product, version, engineRevision, status, entityCount },
  };
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
