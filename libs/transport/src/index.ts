import type { HttpPort, HttpResponse } from '@rusty-d20/platform';
import {
  decodeApiError,
  decodeGameSnapshot,
  decodeRuntimeReadout,
  type ApplyActionRequestDto,
  type ApplyReactionRequestDto,
  type ClassifiedError,
  type EnterEncounterRequestDto,
  type GameSnapshotDto,
  type PreviewActionRequestDto,
  type Result,
  type RuntimeReadoutDto,
} from '@rusty-d20/protocol';

export interface RustyD20Transport {
  readonly loadReadout: () => Promise<Result<RuntimeReadoutDto>>;
  readonly loadSession: () => Promise<Result<GameSnapshotDto>>;
  readonly newAdventure: (expectedRevision: number) => Promise<Result<GameSnapshotDto>>;
  readonly enterEncounter: (
    request: EnterEncounterRequestDto,
  ) => Promise<Result<GameSnapshotDto>>;
  readonly previewAction: (request: PreviewActionRequestDto) => Promise<Result<GameSnapshotDto>>;
  readonly applyReaction: (request: ApplyReactionRequestDto) => Promise<Result<GameSnapshotDto>>;
  readonly applyAction: (request: ApplyActionRequestDto) => Promise<Result<GameSnapshotDto>>;
  readonly advanceTurn: (expectedRevision: number) => Promise<Result<GameSnapshotDto>>;
  readonly save: (expectedRevision: number) => Promise<Result<GameSnapshotDto>>;
}

export function createHttpRustyD20Transport(http: HttpPort): RustyD20Transport {
  const get = async <T>(
    path: string,
    decode: (value: unknown) => Result<T>,
  ): Promise<Result<T>> => request(() => http.getJson(path), decode);
  const post = async <T>(
    path: string,
    body: unknown,
    decode: (value: unknown) => Result<T>,
  ): Promise<Result<T>> => request(() => http.postJson(path, body), decode);

  return {
    loadReadout: () => get('/api/v1/readout', decodeRuntimeReadout),
    loadSession: () => get('/api/v1/session', decodeGameSnapshot),
    newAdventure: (expectedRevision) =>
      post('/api/v1/session/new', { expectedRevision }, decodeGameSnapshot),
    enterEncounter: (body) => post('/api/v1/session/encounter', body, decodeGameSnapshot),
    previewAction: (body) => post('/api/v1/session/preview', body, decodeGameSnapshot),
    applyReaction: (body) => post('/api/v1/session/reaction', body, decodeGameSnapshot),
    applyAction: (body) => post('/api/v1/session/action', body, decodeGameSnapshot),
    advanceTurn: (expectedRevision) =>
      post('/api/v1/session/turn', { expectedRevision }, decodeGameSnapshot),
    save: (expectedRevision) =>
      post('/api/v1/session/save', { expectedRevision }, decodeGameSnapshot),
  };
}

async function request<T>(
  send: () => Promise<HttpResponse>,
  decode: (value: unknown) => Result<T>,
): Promise<Result<T>> {
  try {
    const response = await send();
    if (response.status === 401) {
      return {
        ok: false,
        error: {
          kind: 'unauthorized',
          message: 'Runtime access was denied.',
          retryable: false,
        },
      };
    }
    if (response.status >= 200 && response.status < 300) {
      return decode(response.body);
    }
    const error = decodeApiError(response.body);
    if (error !== undefined) {
      return { ok: false, error };
    }
    return {
      ok: false,
      error: {
        kind: response.status === 404 ? 'not-found' : 'unknown',
        message: `Runtime returned HTTP ${response.status}.`,
        retryable: false,
      },
    };
  } catch (error: unknown) {
    const classified: ClassifiedError = {
      kind: 'network',
      message: error instanceof Error ? error.message : 'Runtime connection failed.',
      retryable: true,
    };
    return { ok: false, error: classified };
  }
}
