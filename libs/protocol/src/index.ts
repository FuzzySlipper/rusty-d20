export * from './generated/api-types';

import type {
  ActionDto,
  AdventureChoiceDto,
  ApiErrorDto,
  ApiErrorKindDto,
  CampaignDto,
  CampaignOutcomeDto,
  CharacterDto,
  DefenseReadoutDto,
  EncounterChoiceDto,
  EncounterDto,
  EncounterOutcomeKindDto,
  EncounterTurnOwnerDto,
  EquipmentSlotDto,
  GameLogEntryDto,
  GameLogKindDto,
  GameSnapshotDto,
  LoadoutCapacityDto,
  LoadoutDto,
  LoadoutItemDto,
  LoadoutRarityDto,
  PendingActionDto,
  ReactionDto,
  ResourceDto,
  RuntimeReadoutDto,
} from './generated/api-types';

export type ClassifiedError =
  | {
      readonly kind: 'network';
      readonly message: string;
      readonly retryable: true;
    }
  | {
      readonly kind: 'unauthorized';
      readonly message: string;
      readonly retryable: false;
    }
  | {
      readonly kind: ApiErrorKindDto | 'unknown';
      readonly message: string;
      readonly retryable: boolean;
    };

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
    return {
      ok: false,
      error: unknownError('Runtime readout has an unexpected shape.'),
    };
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
    return {
      ok: false,
      error: unknownError('Runtime readout contains invalid values.'),
    };
  }
  return {
    ok: true,
    value: { product, version, engineRevision, status, entityCount },
  };
}

export function decodeGameSnapshot(value: unknown): Result<GameSnapshotDto> {
  const decoded = gameSnapshot(value);
  return decoded === undefined
    ? {
        ok: false,
        error: unknownError('Game snapshot has an unexpected or invalid shape.'),
      }
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
      'campaign',
      'availableAdventures',
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
  const campaignValue = value['campaign'];
  const campaign = campaignValue === null ? null : decodeCampaign(campaignValue);
  const availableAdventures = decodeArray(value['availableAdventures'], 16, decodeAdventureChoice);
  const encounterValue = value['encounter'];
  const encounter = encounterValue === null ? null : decodeEncounter(encounterValue);
  if (
    typeof value['product'] !== 'string' ||
    typeof value['version'] !== 'string' ||
    typeof value['engineRevision'] !== 'string' ||
    typeof value['rulesetFingerprint'] !== 'string' ||
    !isSafeNonNegativeInteger(value['revision']) ||
    typeof value['saved'] !== 'boolean' ||
    availableAdventures === undefined ||
    availableAdventures.length === 0 ||
    new Set(availableAdventures.map((choice) => choice.id)).size !== availableAdventures.length ||
    campaign === undefined ||
    encounter === undefined ||
    (campaign === null && encounter !== null) ||
    (campaign?.phase === 'camp' && encounter !== null) ||
    ((campaign?.phase === 'encounter' || campaign?.phase === 'outcome') && encounter === null) ||
    (campaign?.phase === 'encounter' &&
      (encounter?.turnOwner === null || campaign.latestOutcome !== null)) ||
    (campaign?.phase === 'outcome' &&
      (encounter?.turnOwner !== null || campaign.latestOutcome === null))
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
    availableAdventures,
    campaign,
    encounter,
  };
}

function decodeCampaign(value: unknown): CampaignDto | undefined {
  if (
    !hasExactKeys(value, [
      'activeEncounterId',
      'availableEncounters',
      'hero',
      'id',
      'loadout',
      'latestOutcome',
      'phase',
      'title',
    ])
  ) {
    return undefined;
  }
  const hero = decodeCharacter(value['hero']);
  const loadout = decodeLoadout(value['loadout']);
  const encounters = decodeArray(value['availableEncounters'], 16, decodeEncounterChoice);
  const activeEncounterId = value['activeEncounterId'];
  const phase = value['phase'];
  const latestOutcomeValue = value['latestOutcome'];
  const latestOutcome =
    latestOutcomeValue === null ? null : decodeCampaignOutcome(latestOutcomeValue);
  if (
    typeof value['id'] !== 'string' ||
    typeof value['title'] !== 'string' ||
    (phase !== 'camp' && phase !== 'encounter' && phase !== 'outcome') ||
    (activeEncounterId !== null && typeof activeEncounterId !== 'string') ||
    hero === undefined ||
    loadout === undefined ||
    encounters === undefined ||
    latestOutcome === undefined ||
    (phase === 'camp' && activeEncounterId !== null) ||
    ((phase === 'encounter' || phase === 'outcome') && activeEncounterId === null) ||
    (phase === 'encounter' && latestOutcome !== null) ||
    (phase === 'outcome' && latestOutcome === null)
  ) {
    return undefined;
  }
  return {
    id: value['id'],
    title: value['title'],
    phase,
    hero,
    loadout,
    activeEncounterId,
    availableEncounters: encounters,
    latestOutcome,
  };
}

function decodeCampaignOutcome(value: unknown): CampaignOutcomeDto | undefined {
  if (!hasExactKeys(value, ['encounterId', 'kind', 'reward', 'rewardItemId', 'summary', 'title'])) {
    return undefined;
  }
  const rewardItemId = value['rewardItemId'];
  const reward = value['reward'];
  return isEncounterOutcomeKind(value['kind']) &&
    typeof value['encounterId'] === 'string' &&
    value['encounterId'].length > 0 &&
    typeof value['title'] === 'string' &&
    typeof value['summary'] === 'string' &&
    (rewardItemId === null || isSafePositiveInteger(rewardItemId)) &&
    (reward === null || typeof reward === 'string') &&
    ((value['kind'] === 'victory' && rewardItemId !== null && reward !== null) ||
      (value['kind'] === 'defeat' && rewardItemId === null && reward === null))
    ? {
        kind: value['kind'],
        encounterId: value['encounterId'],
        title: value['title'],
        summary: value['summary'],
        rewardItemId,
        reward,
      }
    : undefined;
}

function decodeLoadout(value: unknown): LoadoutDto | undefined {
  if (
    !hasExactKeys(value, [
      'capacity',
      'defenses',
      'equipmentSlots',
      'inventorySlots',
      'ownerId',
      'stashOwnerId',
      'stashItems',
    ])
  ) {
    return undefined;
  }
  const inventorySlots = decodeNullableArray(value['inventorySlots'], 256, decodeLoadoutItem);
  const equipmentSlots = decodeArray(value['equipmentSlots'], 64, decodeEquipmentSlot);
  const stashItems = decodeArray(value['stashItems'], 256, decodeLoadoutItem);
  const capacity = decodeLoadoutCapacity(value['capacity']);
  const defenses = decodeArray(value['defenses'], 64, decodeDefenseReadout);
  if (
    !isSafePositiveInteger(value['ownerId']) ||
    !isSafePositiveInteger(value['stashOwnerId']) ||
    value['stashOwnerId'] === value['ownerId'] ||
    inventorySlots === undefined ||
    equipmentSlots === undefined ||
    stashItems === undefined ||
    capacity === undefined ||
    defenses === undefined ||
    defenses.length === 0 ||
    new Set(defenses.map((defense) => defense.id)).size !== defenses.length ||
    capacity.maximum !== inventorySlots.length ||
    capacity.used !== inventorySlots.filter((item) => item !== null).length ||
    capacity.used > capacity.maximum
  ) {
    return undefined;
  }
  const inventoryItems = inventorySlots.filter((item): item is LoadoutItemDto => item !== null);
  const inventoryIds = new Set(inventoryItems.map((item) => item.entityId));
  const stashIds = new Set(stashItems.map((item) => item.entityId));
  const slotIds = new Set(equipmentSlots.map((slot) => slot.id));
  if (
    inventoryIds.size !== inventoryItems.length ||
    stashIds.size !== stashItems.length ||
    slotIds.size !== equipmentSlots.length ||
    [...inventoryIds].some((id) => stashIds.has(id)) ||
    stashItems.some((item) => item.equippedSlotId !== null) ||
    inventoryItems.some(
      (item) =>
        item.equippedSlotId !== null &&
        !equipmentSlots.some(
          (slot) => slot.id === item.equippedSlotId && slot.equipped?.entityId === item.entityId,
        ),
    ) ||
    equipmentSlots.some(
      (slot) =>
        slot.equipped !== null &&
        (!inventoryIds.has(slot.equipped.entityId) ||
          slot.equipped.equippedSlotId !== slot.id ||
          slot.equipped.equipmentSlotId !== slot.id),
    )
  ) {
    return undefined;
  }
  return {
    ownerId: value['ownerId'],
    stashOwnerId: value['stashOwnerId'],
    inventorySlots,
    equipmentSlots,
    stashItems,
    capacity,
    defenses,
  };
}

function decodeDefenseReadout(value: unknown): DefenseReadoutDto | undefined {
  if (!hasExactKeys(value, ['id', 'label', 'sources', 'value'])) {
    return undefined;
  }
  const sources = decodeStrings(value['sources'], 256);
  return typeof value['id'] === 'string' &&
    value['id'].length > 0 &&
    typeof value['label'] === 'string' &&
    value['label'].length > 0 &&
    isSafeInteger(value['value']) &&
    sources !== undefined
    ? {
        id: value['id'],
        label: value['label'],
        value: value['value'],
        sources,
      }
    : undefined;
}

function decodeLoadoutItem(value: unknown): LoadoutItemDto | undefined {
  if (
    !hasExactKeys(value, [
      'definitionId',
      'entityId',
      'equipmentSlotId',
      'equippedSlotId',
      'icon',
      'name',
      'quantity',
      'rarity',
    ])
  ) {
    return undefined;
  }
  const equippedSlotId = value['equippedSlotId'];
  return isSafePositiveInteger(value['entityId']) &&
    typeof value['definitionId'] === 'string' &&
    value['definitionId'].length > 0 &&
    typeof value['name'] === 'string' &&
    typeof value['icon'] === 'string' &&
    isLoadoutRarity(value['rarity']) &&
    isSafePositiveInteger(value['quantity']) &&
    typeof value['equipmentSlotId'] === 'string' &&
    value['equipmentSlotId'].length > 0 &&
    (equippedSlotId === null || typeof equippedSlotId === 'string')
    ? {
        entityId: value['entityId'],
        definitionId: value['definitionId'],
        name: value['name'],
        icon: value['icon'],
        rarity: value['rarity'],
        quantity: value['quantity'],
        equipmentSlotId: value['equipmentSlotId'],
        equippedSlotId,
      }
    : undefined;
}

function decodeEquipmentSlot(value: unknown): EquipmentSlotDto | undefined {
  if (!hasExactKeys(value, ['equipped', 'id', 'label'])) {
    return undefined;
  }
  const equippedValue = value['equipped'];
  const equipped = equippedValue === null ? null : decodeLoadoutItem(equippedValue);
  return typeof value['id'] === 'string' &&
    value['id'].length > 0 &&
    typeof value['label'] === 'string' &&
    equipped !== undefined
    ? { id: value['id'], label: value['label'], equipped }
    : undefined;
}

function decodeLoadoutCapacity(value: unknown): LoadoutCapacityDto | undefined {
  if (!hasExactKeys(value, ['maximum', 'metric', 'used'])) {
    return undefined;
  }
  return typeof value['metric'] === 'string' &&
    value['metric'].length > 0 &&
    isSafeNonNegativeInteger(value['used']) &&
    isSafeNonNegativeInteger(value['maximum'])
    ? {
        metric: value['metric'],
        used: value['used'],
        maximum: value['maximum'],
      }
    : undefined;
}

function decodeEncounterChoice(value: unknown): EncounterChoiceDto | undefined {
  if (!hasExactKeys(value, ['id', 'summary', 'title'])) {
    return undefined;
  }
  return typeof value['id'] === 'string' &&
    typeof value['title'] === 'string' &&
    typeof value['summary'] === 'string'
    ? { id: value['id'], title: value['title'], summary: value['summary'] }
    : undefined;
}

function decodeAdventureChoice(value: unknown): AdventureChoiceDto | undefined {
  if (!hasExactKeys(value, ['details', 'id', 'summary', 'title'])) {
    return undefined;
  }
  const details = decodeStrings(value['details'], 32);
  return typeof value['id'] === 'string' &&
    value['id'].length > 0 &&
    typeof value['title'] === 'string' &&
    value['title'].length > 0 &&
    typeof value['summary'] === 'string' &&
    value['summary'].length > 0 &&
    details !== undefined
    ? {
        id: value['id'],
        title: value['title'],
        summary: value['summary'],
        details,
      }
    : undefined;
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
      'turnOwner',
    ])
  ) {
    return undefined;
  }
  const characters = decodeArray(value['characters'], 16, decodeCharacter);
  const actions = decodeArray(value['actions'], 64, decodeAction);
  const log = decodeArray(value['log'], 64, decodeLogEntry);
  const pendingValue = value['pendingAction'];
  const pendingAction = pendingValue === null ? null : decodePending(pendingValue);
  const turnOwner = value['turnOwner'];
  if (
    !isSafeNonNegativeInteger(value['turn']) ||
    !isSafeNonNegativeInteger(value['nextRoll']) ||
    !isSafePositiveInteger(value['playerId']) ||
    (turnOwner !== null && !isEncounterTurnOwner(turnOwner)) ||
    characters === undefined ||
    actions === undefined ||
    pendingAction === undefined ||
    log === undefined
  ) {
    return undefined;
  }
  const characterIds = new Set(characters.map((character) => character.id));
  if (
    characterIds.size !== characters.length ||
    !characterIds.has(value['playerId']) ||
    (pendingAction !== null &&
      (!characterIds.has(pendingAction.actorId) ||
        !characterIds.has(pendingAction.targetId) ||
        pendingAction.actorId === pendingAction.targetId ||
        turnOwner === null ||
        (turnOwner === 'player' && pendingAction.actorId !== value['playerId']) ||
        (turnOwner === 'opposition' && pendingAction.actorId === value['playerId']))) ||
    (turnOwner === null && pendingAction !== null)
  ) {
    return undefined;
  }
  return {
    turn: value['turn'],
    nextRoll: value['nextRoll'],
    playerId: value['playerId'],
    turnOwner,
    characters,
    actions,
    pendingAction,
    log,
  };
}

function isEncounterTurnOwner(value: unknown): value is EncounterTurnOwnerDto {
  return value === 'player' || value === 'opposition';
}

function isEncounterOutcomeKind(value: unknown): value is EncounterOutcomeKindDto {
  return value === 'victory' || value === 'defeat';
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

function decodeNullableArray<T>(
  value: unknown,
  maximum: number,
  decode: (entry: unknown) => T | undefined,
): (T | null)[] | undefined {
  return decodeArray(value, maximum, (entry) => (entry === null ? null : decode(entry)));
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
    value === 'invalid-slot' ||
    value === 'capacity' ||
    value === 'containment' ||
    value === 'track-bound' ||
    value === 'phase' ||
    value === 'not-found' ||
    value === 'persistence' ||
    value === 'internal'
  );
}

function isLoadoutRarity(value: unknown): value is LoadoutRarityDto {
  return value === 'common' || value === 'uncommon' || value === 'rare' || value === 'epic';
}

function isLogKind(value: unknown): value is GameLogKindDto {
  return (
    value === 'system' ||
    value === 'reaction' ||
    value === 'hit' ||
    value === 'miss' ||
    value === 'turn'
  );
}
