import type { HttpPort } from '@rusty-d20/platform';
import { decodeRuntimeReadout, type Result, type RuntimeReadoutDto } from '@rusty-d20/protocol';

export interface RustyD20Transport {
  readonly loadReadout: () => Promise<Result<RuntimeReadoutDto>>;
}

export function createHttpRustyD20Transport(http: HttpPort): RustyD20Transport {
  return {
    loadReadout: async () => {
      try {
        const response = await http.getJson('/api/v1/readout');
        if (response.status === 401) {
          return { ok: false, error: { kind: 'unauthorized', message: 'Runtime access was denied.', retryable: false } };
        }
        if (response.status === 404) {
          return { ok: false, error: { kind: 'not-found', message: 'Runtime readout was not found.', retryable: false } };
        }
        if (response.status < 200 || response.status >= 300) {
          return { ok: false, error: { kind: 'unknown', message: `Runtime returned HTTP ${response.status}.`, retryable: false } };
        }
        return decodeRuntimeReadout(response.body);
      } catch (error: unknown) {
        return {
          ok: false,
          error: {
            kind: 'network',
            message: error instanceof Error ? error.message : 'Runtime connection failed.',
            retryable: true,
          },
        };
      }
    },
  };
}
