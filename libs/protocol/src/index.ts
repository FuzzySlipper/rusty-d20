export * from './generated/api-types';

import type {
  ActionDto,
  ApiErrorDto,
  ApiErrorKindDto,
  CharacterDto,
  EncounterDto,
  GameLogEntryDto,
  GameLogKindDto,
  GameSnapshotDto,
  PendingActionDto,
  ReactionDto,
  ResourceDto,
  RuntimeReadoutDto,
} from './generated/api-types';

export type ClassifiedError =
  | { readonly kind: 'network'; readonly message: string; readonly retryable: true }
  | { readonly kind: 'unauthorized'; readonly message: string; readonly retryable: false }
  | { readonly kind: ApiErrorKindDto | 'unknown'; readonly message: string; readonly retryable: boolean };

export type Result<T> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly error: ClassifiedError };

export const unknownError = (message: string): ClassifiedError => ({
  kind: 'unknown',
  message,
  retryable: false,
});

export function decodeRuntimeReadout(value: unknown): Result<RuntimeReadoutDto> {
  if (!hasExactKeys(value, ['engineRevision', 'entityCount', 'product', 'status', 'version'])) {
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
    !isSafeNonNegativeInteger(entityCount)
  ) {
    return { ok: false, error: unknownError('Runtime readout contains invalid values.') };
  }
  return { ok: true, value: { product, version, engineRevision, status, entityCount } };
}

export function decodeGameSnapshot(value: unknown): Result<GameSnapshotDto> {
  const decoded = gameSnapshot(value);
  return decoded === undefined
    ? { ok: false, error: unknownError('Game snapshot has an unexpected or invalid shape.') }
    : { ok: true, value: decoded };
}

export function decodeApiError(value: unknown): ApiErrorDto | undefined {
  if (!hasExactKeys(value, ['kind', 'message', 'retryable'])) {
    return undefined;
  }
  const kind = value['kind'];
  const message = value['message'];
  const retryable = value['retryable'];
  return isApiErrorKind(kind) && typeof message === 'string' && typeof retryable === 'boolean'
    ? { kind, message, retryable }
    : undefined;
}

function gameSnapshot(value: unknown): GameSnapshotDto | undefined {
  if (
    !hasExactKeys(value, [
      'encounter',
      'engineRevision',
      'product',
      'revision',
      'rulesetFingerprint',
      'saved',
      'version',
    ])
  ) {
    return undefined;
  }
  const encounterValue = value['encounter'];
  const encounter = encounterValue === null ? null : decodeEncounter(encounterValue);
  if (
    typeof value['product'] !== 'string' ||
    typeof value['version'] !== 'string' ||
    typeof value['engineRevision'] !== 'string' ||
    typeof value['rulesetFingerprint'] !== 'string' ||
    !isSafeNonNegativeInteger(value['revision']) ||
    typeof value['saved'] !== 'boolean' ||
    encounter === undefined
  ) {
    return undefined;
  }
  return {
    product: value['product'],
    version: value['version'],
    engineRevision: value['engineRevision'],
    rulesetFingerprint: value['rulesetFingerprint'],
    revision: value['revision'],
    saved: value['saved'],
    encounter,
  };
}

function decodeEncounter(value: unknown): EncounterDto | undefined {
  if (
    !hasExactKeys(value, [
      'actions',
      'characters',
      'log',
      'nextRoll',
      'pendingAction',
      'playerId',
      'turn',
    ])
  ) {
    return undefined;
  }
  const characters = decodeArray(value['characters'], 16, decodeCharacter);
  const actions = decodeArray(value['actions'], 64, decodeAction);
  const log = decodeArray(value['log'], 64, decodeLogEntry);
  const pendingValue = value['pendingAction'];
  const pendingAction = pendingValue === null ? null : decodePending(pendingValue);
  if (
    !isSafeNonNegativeInteger(value['turn']) ||
    !isSafeNonNegativeInteger(value['nextRoll']) ||
    !isSafePositiveInteger(value['playerId']) ||
    characters === undefined ||
    actions === undefined ||
    pendingAction === undefined ||
    log === undefined
  ) {
    return undefined;
  }
  return {
    turn: value['turn'],
    nextRoll: value['nextRoll'],
    playerId: value['playerId'],
    characters,
    actions,
    pendingAction,
    log,
  };
}

function decodeCharacter(value: unknown): CharacterDto | undefined {
  if (
    !hasExactKeys(value, [
      'effects',
      'healthCurrent',
      'healthMaximum',
      'id',
      'level',
      'name',
      'resources',
      'title',
    ])
  ) {
    return undefined;
  }
  const resources = decodeArray(value['resources'], 64, decodeResource);
  const effects = decodeStrings(value['effects'], 64);
  if (
    !isSafePositiveInteger(value['id']) ||
    typeof value['name'] !== 'string' ||
    typeof value['title'] !== 'string' ||
    !isSafeNonNegativeInteger(value['level']) ||
    !isSafeInteger(value['healthCurrent']) ||
    !isSafeNonNegativeInteger(value['healthMaximum']) ||
    resources === undefined ||
    effects === undefined
  ) {
    return undefined;
  }
  return {
    id: value['id'],
    name: value['name'],
    title: value['title'],
    level: value['level'],
    healthCurrent: value['healthCurrent'],
    healthMaximum: value['healthMaximum'],
    resources,
    effects,
  };
}

function decodeResource(value: unknown): ResourceDto | undefined {
  if (!hasExactKeys(value, ['current', 'id', 'label', 'maximum'])) {
    return undefined;
  }
  return typeof value['id'] === 'string' &&
    typeof value['label'] === 'string' &&
    isSafeNonNegativeInteger(value['current']) &&
    isSafeNonNegativeInteger(value['maximum'])
    ? {
        id: value['id'],
        label: value['label'],
        current: value['current'],
        maximum: value['maximum'],
      }
    : undefined;
}

function decodeAction(value: unknown): ActionDto | undefined {
  if (!hasExactKeys(value, ['ability', 'damage', 'defense', 'effect', 'id', 'label'])) {
    return undefined;
  }
  const effect = value['effect'];
  return typeof value['id'] === 'string' &&
    typeof value['label'] === 'string' &&
    typeof value['ability'] === 'string' &&
    typeof value['defense'] === 'string' &&
    typeof value['damage'] === 'string' &&
    (typeof effect === 'string' || effect === null)
    ? {
        id: value['id'],
        label: value['label'],
        ability: value['ability'],
        defense: value['defense'],
        damage: value['damage'],
        effect,
      }
    : undefined;
}

function decodeReaction(value: unknown): ReactionDto | undefined {
  if (!hasExactKeys(value, ['available', 'bonus', 'cost', 'effect', 'id', 'label', 'resource'])) {
    return undefined;
  }
  return typeof value['id'] === 'string' &&
    typeof value['label'] === 'string' &&
    typeof value['resource'] === 'string' &&
    isSafeNonNegativeInteger(value['cost']) &&
    isSafeNonNegativeInteger(value['available']) &&
    isSafeInteger(value['bonus']) &&
    typeof value['effect'] === 'string'
    ? {
        id: value['id'],
        label: value['label'],
        resource: value['resource'],
        cost: value['cost'],
        available: value['available'],
        bonus: value['bonus'],
        effect: value['effect'],
      }
    : undefined;
}

function decodePending(value: unknown): PendingActionDto | undefined {
  if (
    !hasExactKeys(value, [
      'abilityModifier',
      'abilityScore',
      'actionId',
      'actionLabel',
      'actorId',
      'defense',
      'defenseSources',
      'reactions',
      'targetId',
      'token',
    ])
  ) {
    return undefined;
  }
  const defenseSources = decodeStrings(value['defenseSources'], 256);
  const reactions = decodeArray(value['reactions'], 64, decodeReaction);
  if (
    typeof value['token'] !== 'string' ||
    !isSafePositiveInteger(value['actorId']) ||
    !isSafePositiveInteger(value['targetId']) ||
    typeof value['actionId'] !== 'string' ||
    typeof value['actionLabel'] !== 'string' ||
    !isSafeInteger(value['abilityScore']) ||
    !isSafeInteger(value['abilityModifier']) ||
    !isSafeInteger(value['defense']) ||
    defenseSources === undefined ||
    reactions === undefined
  ) {
    return undefined;
  }
  return {
    token: value['token'],
    actorId: value['actorId'],
    targetId: value['targetId'],
    actionId: value['actionId'],
    actionLabel: value['actionLabel'],
    abilityScore: value['abilityScore'],
    abilityModifier: value['abilityModifier'],
    defense: value['defense'],
    defenseSources,
    reactions,
  };
}

function decodeLogEntry(value: unknown): GameLogEntryDto | undefined {
  if (!hasExactKeys(value, ['details', 'id', 'kind', 'source', 'text', 'turn'])) {
    return undefined;
  }
  const details = decodeStrings(value['details'], 32);
  return isSafePositiveInteger(value['id']) &&
    isSafeNonNegativeInteger(value['turn']) &&
    isLogKind(value['kind']) &&
    typeof value['source'] === 'string' &&
    typeof value['text'] === 'string' &&
    details !== undefined
    ? {
        id: value['id'],
        turn: value['turn'],
        kind: value['kind'],
        source: value['source'],
        text: value['text'],
        details,
      }
    : undefined;
}

function decodeArray<T>(
  value: unknown,
  maximum: number,
  decode: (entry: unknown) => T | undefined,
): T[] | undefined {
  if (!Array.isArray(value) || value.length > maximum) {
    return undefined;
  }
  const decoded: T[] = [];
  for (const entry of value) {
    const item = decode(entry);
    if (item === undefined) {
      return undefined;
    }
    decoded.push(item);
  }
  return decoded;
}

function decodeStrings(value: unknown, maximum: number): string[] | undefined {
  return Array.isArray(value) &&
    value.length <= maximum &&
    value.every((entry) => typeof entry === 'string' && entry.length <= 512)
    ? value
    : undefined;
}

function hasExactKeys(
  value: unknown,
  expected: readonly string[],
): value is Readonly<Record<string, unknown>> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    return false;
  }
  const actual = Object.keys(value).sort();
  const keys = [...expected].sort();
  return actual.length === keys.length && actual.every((key, index) => key === keys[index]);
}

function isSafeInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value);
}

function isSafeNonNegativeInteger(value: unknown): value is number {
  return isSafeInteger(value) && value >= 0;
}

function isSafePositiveInteger(value: unknown): value is number {
  return isSafeInteger(value) && value > 0;
}

function isApiErrorKind(value: unknown): value is ApiErrorKindDto {
  return (
    value === 'stale' ||
    value === 'invalid' ||
    value === 'not-found' ||
    value === 'persistence' ||
    value === 'internal'
  );
}

function isLogKind(value: unknown): value is GameLogKindDto {
  return value === 'system' || value === 'reaction' || value === 'hit' || value === 'miss' || value === 'turn';
}
