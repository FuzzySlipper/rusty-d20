export * from "./generated/api-types";

import { D20_PROTOCOL_LIMITS } from "./generated/api-types";
import type {
  ActionDto,
  ActionTargetsDto,
  AdventureCompletionDto,
  AdventureChoiceDto,
  ApiErrorDto,
  ApiErrorKindDto,
  CampaignDto,
  CampaignOutcomeDto,
  CharacterDto,
  CompletedEncounterDto,
  DefenseReadoutDto,
  EncounterChoiceDto,
  EncounterDto,
  EncounterFactionDto,
  EncounterOutcomeKindDto,
  EncounterParticipantDto,
  EquipmentSlotDto,
  ExplorationDepthDto,
  ExplorationDoorDto,
  ExplorationCheckpointDto,
  ExplorationDto,
  ExplorationLandmarkDto,
  ExplorationTreasureDto,
  GameLogEntryDto,
  GameLogKindDto,
  GameSnapshotDto,
  LoadoutCapacityDto,
  LoadoutDto,
  LoadoutItemDto,
  LoadoutRarityDto,
  PartyMemberDto,
  ReactionDto,
  ReactionPromptDto,
  ResourceDto,
  RuntimeReadoutDto,
  SaveStatusDto,
  TacticalBoardDto,
  TacticalCellDto,
  TacticalMoveDto,
} from "./generated/api-types";

export type ClassifiedError =
  | {
      readonly kind: "network";
      readonly message: string;
      readonly retryable: true;
    }
  | {
      readonly kind: "unauthorized";
      readonly message: string;
      readonly retryable: false;
    }
  | {
      readonly kind: ApiErrorKindDto | "unknown";
      readonly message: string;
      readonly retryable: boolean;
    };

export type Result<T> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly error: ClassifiedError };

export const unknownError = (message: string): ClassifiedError => ({
  kind: "unknown",
  message,
  retryable: false,
});

export function decodeRuntimeReadout(
  value: unknown,
): Result<RuntimeReadoutDto> {
  if (
    !hasExactKeys(value, [
      "engineRevision",
      "entityCount",
      "product",
      "status",
      "version",
    ])
  ) {
    return {
      ok: false,
      error: unknownError("Runtime readout has an unexpected shape."),
    };
  }
  const product = value["product"];
  const version = value["version"];
  const engineRevision = value["engineRevision"];
  const status = value["status"];
  const entityCount = value["entityCount"];
  if (
    typeof product !== "string" ||
    typeof version !== "string" ||
    typeof engineRevision !== "string" ||
    status !== "ready" ||
    !isSafeNonNegativeInteger(entityCount)
  ) {
    return {
      ok: false,
      error: unknownError("Runtime readout contains invalid values."),
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
        error: unknownError(
          "Game snapshot has an unexpected or invalid shape.",
        ),
      }
    : { ok: true, value: decoded };
}

export function decodeSaveStatus(value: unknown): Result<SaveStatusDto> {
  if (
    !hasExactKeys(value, [
      "campaignId",
      "campaignTitle",
      "persistenceError",
      "revision",
      "saveIdentity",
      "state",
    ])
  ) {
    return {
      ok: false,
      error: unknownError("Save status has an unexpected shape."),
    };
  }
  const saveIdentity = value["saveIdentity"];
  const state = value["state"];
  const campaignId = value["campaignId"];
  const campaignTitle = value["campaignTitle"];
  const revision = value["revision"];
  const persistenceError = value["persistenceError"];
  const valid =
    typeof saveIdentity === "string" &&
    saveIdentity.length > 0 &&
    (state === "empty" || state === "ready" || state === "recovery-required") &&
    (campaignId === null ||
      (typeof campaignId === "string" && campaignId.length > 0)) &&
    (campaignTitle === null ||
      (typeof campaignTitle === "string" && campaignTitle.length > 0)) &&
    (revision === null || isSafeNonNegativeInteger(revision)) &&
    (persistenceError === null ||
      (typeof persistenceError === "string" && persistenceError.length > 0)) &&
    ((state === "empty" &&
      campaignId === null &&
      campaignTitle === null &&
      revision !== null &&
      persistenceError === null) ||
      (state === "ready" &&
        campaignId !== null &&
        campaignTitle !== null &&
        revision !== null &&
        persistenceError === null) ||
      (state === "recovery-required" &&
        campaignId === null &&
        campaignTitle === null &&
        revision === null &&
        persistenceError !== null));
  return valid
    ? {
        ok: true,
        value: {
          saveIdentity,
          state,
          campaignId,
          campaignTitle,
          revision,
          persistenceError,
        },
      }
    : {
        ok: false,
        error: unknownError("Save status contains invalid values."),
      };
}

export function decodeApiError(value: unknown): ApiErrorDto | undefined {
  if (!hasExactKeys(value, ["kind", "message", "retryable"])) {
    return undefined;
  }
  const kind = value["kind"];
  const message = value["message"];
  const retryable = value["retryable"];
  return isApiErrorKind(kind) &&
    typeof message === "string" &&
    typeof retryable === "boolean"
    ? { kind, message, retryable }
    : undefined;
}

function gameSnapshot(value: unknown): GameSnapshotDto | undefined {
  if (
    !hasExactKeys(value, [
      "campaign",
      "availableAdventures",
      "encounter",
      "engineRevision",
      "exploration",
      "product",
      "revision",
      "rulesetFingerprint",
      "saved",
      "version",
    ])
  ) {
    return undefined;
  }
  const campaignValue = value["campaign"];
  const campaign =
    campaignValue === null ? null : decodeCampaign(campaignValue);
  const availableAdventures = decodeArray(
    value["availableAdventures"],
    D20_PROTOCOL_LIMITS.maxAvailableAdventures,
    decodeAdventureChoice,
  );
  const encounterValue = value["encounter"];
  const encounter =
    encounterValue === null ? null : decodeEncounter(encounterValue);
  const explorationValue = value["exploration"];
  const exploration =
    explorationValue === null ? null : decodeExploration(explorationValue);
  if (
    typeof value["product"] !== "string" ||
    typeof value["version"] !== "string" ||
    typeof value["engineRevision"] !== "string" ||
    typeof value["rulesetFingerprint"] !== "string" ||
    !isSafeNonNegativeInteger(value["revision"]) ||
    typeof value["saved"] !== "boolean" ||
    availableAdventures === undefined ||
    availableAdventures.length === 0 ||
    new Set(availableAdventures.map((choice) => choice.id)).size !==
      availableAdventures.length ||
    campaign === undefined ||
    exploration === undefined ||
    encounter === undefined ||
    (campaign === null && encounter !== null) ||
    (campaign === null && exploration !== null) ||
    (campaign?.phase === "camp" &&
      (encounter !== null || exploration !== null)) ||
    (campaign?.phase === "exploration" &&
      (exploration === null || encounter !== null)) ||
    ((campaign?.phase === "encounter" || campaign?.phase === "outcome") &&
      encounter === null) ||
    (campaign?.phase === "adventure-complete" &&
      (exploration !== null || encounter !== null)) ||
    (campaign?.phase === "encounter" &&
      (encounter?.currentActorId === null ||
        campaign.latestOutcome !== null)) ||
    (campaign?.phase === "outcome" &&
      (encounter?.currentActorId !== null || campaign.latestOutcome === null))
  ) {
    return undefined;
  }
  return {
    product: value["product"],
    version: value["version"],
    engineRevision: value["engineRevision"],
    rulesetFingerprint: value["rulesetFingerprint"],
    revision: value["revision"],
    saved: value["saved"],
    availableAdventures,
    campaign,
    exploration,
    encounter,
  };
}

function decodeExploration(value: unknown): ExplorationDto | undefined {
  if (
    !hasExactKeys(value, [
      "canStepBackward",
      "canStepForward",
      "discoveredCells",
      "dungeonTitle",
      "facing",
      "height",
      "checkpoint",
      "doorAhead",
      "landmark",
      "treasure",
      "view",
      "wallStyle",
      "width",
      "x",
      "y",
    ])
  ) {
    return undefined;
  }
  const view = decodeArray(
    value["view"],
    D20_PROTOCOL_LIMITS.maxDungeonViewDepth,
    decodeExplorationDepth,
  );
  const discoveredCells = decodeArray(
    value["discoveredCells"],
    D20_PROTOCOL_LIMITS.maxDungeonCells,
    decodeDiscoveredCell,
  );
  const landmarkValue = value["landmark"];
  const landmark =
    landmarkValue === null ? null : decodeExplorationLandmark(landmarkValue);
  const doorValue = value["doorAhead"];
  const doorAhead =
    doorValue === null ? null : decodeExplorationDoor(doorValue);
  const treasureValue = value["treasure"];
  const treasure =
    treasureValue === null ? null : decodeExplorationTreasure(treasureValue);
  const checkpointValue = value["checkpoint"];
  const checkpoint =
    checkpointValue === null
      ? null
      : decodeExplorationCheckpoint(checkpointValue);
  const width = value["width"];
  const height = value["height"];
  const x = value["x"];
  const y = value["y"];
  let viewOccluded = false;
  const leaksOccludedTopology = view?.some((depth) => {
    if (
      viewOccluded &&
      !(depth.frontBlocked && depth.leftBlocked && depth.rightBlocked)
    ) {
      return true;
    }
    viewOccluded ||= depth.frontBlocked;
    return false;
  });
  if (
    typeof value["dungeonTitle"] !== "string" ||
    value["dungeonTitle"].length === 0 ||
    typeof value["wallStyle"] !== "string" ||
    value["wallStyle"].length === 0 ||
    !isSafePositiveInteger(width) ||
    !isSafePositiveInteger(height) ||
    width * height > D20_PROTOCOL_LIMITS.maxDungeonCells ||
    !isSafeNonNegativeInteger(x) ||
    !isSafeNonNegativeInteger(y) ||
    x >= width ||
    y >= height ||
    !isExplorationFacing(value["facing"]) ||
    typeof value["canStepForward"] !== "boolean" ||
    typeof value["canStepBackward"] !== "boolean" ||
    view === undefined ||
    view.length !== D20_PROTOCOL_LIMITS.maxDungeonViewDepth ||
    view.some((depth, index) => depth.depth !== index) ||
    leaksOccludedTopology ||
    discoveredCells === undefined ||
    discoveredCells.length === 0 ||
    discoveredCells.some((cell) => cell.x >= width || cell.y >= height) ||
    new Set(discoveredCells.map((cell) => `${cell.x}:${cell.y}`)).size !==
      discoveredCells.length ||
    !discoveredCells.some((cell) => cell.x === x && cell.y === y) ||
    landmark === undefined ||
    doorAhead === undefined ||
    treasure === undefined ||
    checkpoint === undefined
  ) {
    return undefined;
  }
  return {
    dungeonTitle: value["dungeonTitle"],
    wallStyle: value["wallStyle"],
    width,
    height,
    x,
    y,
    facing: value["facing"],
    canStepForward: value["canStepForward"],
    canStepBackward: value["canStepBackward"],
    view,
    discoveredCells,
    landmark,
    doorAhead,
    treasure,
    checkpoint,
  };
}

function decodeExplorationDepth(
  value: unknown,
): ExplorationDepthDto | undefined {
  if (
    !hasExactKeys(value, [
      "depth",
      "frontBlocked",
      "leftBlocked",
      "rightBlocked",
    ])
  ) {
    return undefined;
  }
  return isSafeNonNegativeInteger(value["depth"]) &&
    typeof value["frontBlocked"] === "boolean" &&
    typeof value["leftBlocked"] === "boolean" &&
    typeof value["rightBlocked"] === "boolean"
    ? {
        depth: value["depth"],
        frontBlocked: value["frontBlocked"],
        leftBlocked: value["leftBlocked"],
        rightBlocked: value["rightBlocked"],
      }
    : undefined;
}

function decodeDiscoveredCell(
  value: unknown,
): ExplorationDto["discoveredCells"][number] | undefined {
  if (!hasExactKeys(value, ["x", "y"])) {
    return undefined;
  }
  return isSafeNonNegativeInteger(value["x"]) &&
    isSafeNonNegativeInteger(value["y"])
    ? { x: value["x"], y: value["y"] }
    : undefined;
}

function decodeExplorationLandmark(
  value: unknown,
): ExplorationLandmarkDto | undefined {
  if (!hasExactKeys(value, ["id", "inspected", "text", "title"])) {
    return undefined;
  }
  return typeof value["id"] === "string" &&
    value["id"].length > 0 &&
    typeof value["title"] === "string" &&
    value["title"].length > 0 &&
    typeof value["text"] === "string" &&
    value["text"].length > 0 &&
    typeof value["inspected"] === "boolean"
    ? {
        id: value["id"],
        title: value["title"],
        text: value["text"],
        inspected: value["inspected"],
      }
    : undefined;
}

function decodeExplorationDoor(value: unknown): ExplorationDoorDto | undefined {
  if (!hasExactKeys(value, ["id", "locked", "opened", "text", "title"])) {
    return undefined;
  }
  const authored = decodeAuthoredIdentityAndText(value);
  return authored !== undefined &&
    typeof value["opened"] === "boolean" &&
    typeof value["locked"] === "boolean" &&
    !(value["opened"] && value["locked"])
    ? {
        ...authored,
        opened: value["opened"],
        locked: value["locked"],
      }
    : undefined;
}

function decodeExplorationTreasure(
  value: unknown,
): ExplorationTreasureDto | undefined {
  if (!hasExactKeys(value, ["collected", "id", "text", "title"])) {
    return undefined;
  }
  const authored = decodeAuthoredIdentityAndText(value);
  return authored !== undefined && typeof value["collected"] === "boolean"
    ? {
        ...authored,
        collected: value["collected"],
      }
    : undefined;
}

function decodeExplorationCheckpoint(
  value: unknown,
): ExplorationCheckpointDto | undefined {
  if (!hasExactKeys(value, ["active", "id", "text", "title"])) {
    return undefined;
  }
  const authored = decodeAuthoredIdentityAndText(value);
  return authored !== undefined && typeof value["active"] === "boolean"
    ? {
        ...authored,
        active: value["active"],
      }
    : undefined;
}

function decodeAuthoredIdentityAndText(
  value: Record<string, unknown>,
): { id: string; title: string; text: string } | undefined {
  return typeof value["id"] === "string" &&
    value["id"].length > 0 &&
    typeof value["title"] === "string" &&
    value["title"].length > 0 &&
    typeof value["text"] === "string" &&
    value["text"].length > 0
    ? { id: value["id"], title: value["title"], text: value["text"] }
    : undefined;
}

function decodeCampaign(value: unknown): CampaignDto | undefined {
  if (
    !hasExactKeys(value, [
      "activeEncounterId",
      "availableEncounters",
      "completion",
      "completedEncounters",
      "id",
      "latestOutcome",
      "party",
      "phase",
      "title",
    ])
  ) {
    return undefined;
  }
  const party = decodeArray(
    value["party"],
    D20_PROTOCOL_LIMITS.maxPartyMembers,
    decodePartyMember,
  );
  const encounters = decodeArray(
    value["availableEncounters"],
    D20_PROTOCOL_LIMITS.maxCampaignEncounters,
    decodeEncounterChoice,
  );
  const completedEncounters = decodeArray(
    value["completedEncounters"],
    D20_PROTOCOL_LIMITS.maxCampaignEncounters,
    decodeCompletedEncounter,
  );
  const activeEncounterId = value["activeEncounterId"];
  const phase = value["phase"];
  const latestOutcomeValue = value["latestOutcome"];
  const latestOutcome =
    latestOutcomeValue === null
      ? null
      : decodeCampaignOutcome(latestOutcomeValue);
  const completionValue = value["completion"];
  const completion =
    completionValue === null
      ? null
      : decodeAdventureCompletion(completionValue);
  if (
    typeof value["id"] !== "string" ||
    typeof value["title"] !== "string" ||
    (phase !== "camp" &&
      phase !== "exploration" &&
      phase !== "encounter" &&
      phase !== "outcome" &&
      phase !== "adventure-complete") ||
    (activeEncounterId !== null && typeof activeEncounterId !== "string") ||
    party === undefined ||
    party.length === 0 ||
    new Set(party.map((member) => member.character.id)).size !== party.length ||
    !hasCanonicalSharedStash(party) ||
    encounters === undefined ||
    completedEncounters === undefined ||
    new Set(completedEncounters.map((entry) => entry.encounterId)).size !==
      completedEncounters.length ||
    latestOutcome === undefined ||
    completion === undefined ||
    ((phase === "camp" ||
      phase === "exploration" ||
      phase === "adventure-complete") &&
      activeEncounterId !== null) ||
    ((phase === "camp" || phase === "exploration") &&
      latestOutcome === null &&
      completedEncounters.length !== 0) ||
    ((phase === "encounter" || phase === "outcome") &&
      activeEncounterId === null) ||
    (phase !== "adventure-complete" && completion !== null) ||
    (phase === "adventure-complete" &&
      (completion === null ||
        latestOutcome === null ||
        completedEncounters.length === 0 ||
        encounters.length !== 0 ||
        completion.kind !== latestOutcome.kind)) ||
    (phase === "encounter" &&
      completedEncounters.some(
        (completed) => completed.encounterId === activeEncounterId,
      )) ||
    (phase === "encounter" && latestOutcome !== null) ||
    (phase === "outcome" && latestOutcome === null) ||
    (phase === "outcome" && latestOutcome?.encounterId !== activeEncounterId) ||
    (latestOutcome !== null &&
      (completedEncounters.at(-1)?.encounterId !== latestOutcome.encounterId ||
        completedEncounters.at(-1)?.outcome !== latestOutcome.kind))
  ) {
    return undefined;
  }
  return {
    id: value["id"],
    title: value["title"],
    phase,
    party,
    activeEncounterId,
    availableEncounters: encounters,
    latestOutcome,
    completedEncounters,
    completion,
  };
}

function hasCanonicalSharedStash(party: PartyMemberDto[]): boolean {
  const canonical = party[0]?.loadout;
  return (
    canonical !== undefined &&
    party.every(
      ({ loadout }) =>
        loadout.stashOwnerId === canonical.stashOwnerId &&
        loadout.stashCapacity.metric === canonical.stashCapacity.metric &&
        loadout.stashCapacity.used === canonical.stashCapacity.used &&
        loadout.stashCapacity.maximum === canonical.stashCapacity.maximum &&
        loadout.stashItems.length === canonical.stashItems.length &&
        loadout.stashItems.every((item, index) =>
          isSameLoadoutItem(item, canonical.stashItems[index]),
        ),
    )
  );
}

function isSameLoadoutItem(
  left: LoadoutItemDto,
  right: LoadoutItemDto | undefined,
): boolean {
  return (
    right !== undefined &&
    left.entityId === right.entityId &&
    left.definitionId === right.definitionId &&
    left.name === right.name &&
    left.icon === right.icon &&
    left.rarity === right.rarity &&
    left.quantity === right.quantity &&
    left.equipmentSlotId === right.equipmentSlotId &&
    left.equippedSlotId === right.equippedSlotId
  );
}

function decodeAdventureCompletion(
  value: unknown,
): AdventureCompletionDto | undefined {
  if (!hasExactKeys(value, ["details", "kind", "source", "text", "title"])) {
    return undefined;
  }
  const details = decodeStrings(
    value["details"],
    D20_PROTOCOL_LIMITS.maxAdventureDetails,
  );
  return isEncounterOutcomeKind(value["kind"]) &&
    typeof value["source"] === "string" &&
    value["source"].length > 0 &&
    typeof value["title"] === "string" &&
    value["title"].length > 0 &&
    typeof value["text"] === "string" &&
    value["text"].length > 0 &&
    details !== undefined
    ? {
        kind: value["kind"],
        source: value["source"],
        title: value["title"],
        text: value["text"],
        details,
      }
    : undefined;
}

function decodePartyMember(value: unknown): PartyMemberDto | undefined {
  if (!hasExactKeys(value, ["character", "loadout"])) {
    return undefined;
  }
  const character = decodeCharacter(value["character"]);
  const loadout = decodeLoadout(value["loadout"]);
  return character !== undefined &&
    loadout !== undefined &&
    character.id === loadout.ownerId
    ? { character, loadout }
    : undefined;
}

function decodeCampaignOutcome(value: unknown): CampaignOutcomeDto | undefined {
  if (
    !hasExactKeys(value, [
      "encounterId",
      "kind",
      "reward",
      "rewardItemId",
      "summary",
      "title",
    ])
  ) {
    return undefined;
  }
  const rewardItemId = value["rewardItemId"];
  const reward = value["reward"];
  return isEncounterOutcomeKind(value["kind"]) &&
    typeof value["encounterId"] === "string" &&
    value["encounterId"].length > 0 &&
    typeof value["title"] === "string" &&
    typeof value["summary"] === "string" &&
    (rewardItemId === null || isSafePositiveInteger(rewardItemId)) &&
    (reward === null || typeof reward === "string") &&
    ((rewardItemId === null && reward === null) ||
      (value["kind"] === "victory" && rewardItemId !== null && reward !== null))
    ? {
        kind: value["kind"],
        encounterId: value["encounterId"],
        title: value["title"],
        summary: value["summary"],
        rewardItemId,
        reward,
      }
    : undefined;
}

function decodeCompletedEncounter(
  value: unknown,
): CompletedEncounterDto | undefined {
  if (!hasExactKeys(value, ["encounterId", "outcome", "title"])) {
    return undefined;
  }
  return typeof value["encounterId"] === "string" &&
    value["encounterId"].length > 0 &&
    typeof value["title"] === "string" &&
    value["title"].length > 0 &&
    isEncounterOutcomeKind(value["outcome"])
    ? {
        encounterId: value["encounterId"],
        title: value["title"],
        outcome: value["outcome"],
      }
    : undefined;
}

function decodeLoadout(value: unknown): LoadoutDto | undefined {
  if (
    !hasExactKeys(value, [
      "capacity",
      "defenses",
      "equipmentSlots",
      "inventorySlots",
      "ownerId",
      "stashCapacity",
      "stashOwnerId",
      "stashItems",
    ])
  ) {
    return undefined;
  }
  const inventorySlots = decodeNullableArray(
    value["inventorySlots"],
    256,
    decodeLoadoutItem,
  );
  const equipmentSlots = decodeArray(
    value["equipmentSlots"],
    64,
    decodeEquipmentSlot,
  );
  const stashItems = decodeArray(value["stashItems"], 256, decodeLoadoutItem);
  const stashCapacity = decodeLoadoutCapacity(value["stashCapacity"]);
  const capacity = decodeLoadoutCapacity(value["capacity"]);
  const defenses = decodeArray(value["defenses"], 64, decodeDefenseReadout);
  if (
    !isSafePositiveInteger(value["ownerId"]) ||
    !isSafePositiveInteger(value["stashOwnerId"]) ||
    value["stashOwnerId"] === value["ownerId"] ||
    inventorySlots === undefined ||
    equipmentSlots === undefined ||
    stashItems === undefined ||
    stashCapacity === undefined ||
    capacity === undefined ||
    defenses === undefined ||
    defenses.length === 0 ||
    new Set(defenses.map((defense) => defense.id)).size !== defenses.length ||
    capacity.maximum !== inventorySlots.length ||
    capacity.used !== inventorySlots.filter((item) => item !== null).length ||
    capacity.used > capacity.maximum ||
    stashCapacity.used !== stashItems.length ||
    stashCapacity.used > stashCapacity.maximum
  ) {
    return undefined;
  }
  const inventoryItems = inventorySlots.filter(
    (item): item is LoadoutItemDto => item !== null,
  );
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
          (slot) =>
            slot.id === item.equippedSlotId &&
            slot.equipped?.entityId === item.entityId,
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
    ownerId: value["ownerId"],
    stashOwnerId: value["stashOwnerId"],
    inventorySlots,
    equipmentSlots,
    stashItems,
    stashCapacity,
    capacity,
    defenses,
  };
}

function decodeDefenseReadout(value: unknown): DefenseReadoutDto | undefined {
  if (!hasExactKeys(value, ["id", "label", "sources", "value"])) {
    return undefined;
  }
  const sources = decodeStrings(value["sources"], 256);
  return typeof value["id"] === "string" &&
    value["id"].length > 0 &&
    typeof value["label"] === "string" &&
    value["label"].length > 0 &&
    isSafeInteger(value["value"]) &&
    sources !== undefined
    ? {
        id: value["id"],
        label: value["label"],
        value: value["value"],
        sources,
      }
    : undefined;
}

function decodeLoadoutItem(value: unknown): LoadoutItemDto | undefined {
  if (
    !hasExactKeys(value, [
      "definitionId",
      "entityId",
      "equipmentSlotId",
      "equippedSlotId",
      "icon",
      "name",
      "quantity",
      "rarity",
    ])
  ) {
    return undefined;
  }
  const equippedSlotId = value["equippedSlotId"];
  return isSafePositiveInteger(value["entityId"]) &&
    typeof value["definitionId"] === "string" &&
    value["definitionId"].length > 0 &&
    typeof value["name"] === "string" &&
    typeof value["icon"] === "string" &&
    isLoadoutRarity(value["rarity"]) &&
    isSafePositiveInteger(value["quantity"]) &&
    typeof value["equipmentSlotId"] === "string" &&
    value["equipmentSlotId"].length > 0 &&
    (equippedSlotId === null || typeof equippedSlotId === "string")
    ? {
        entityId: value["entityId"],
        definitionId: value["definitionId"],
        name: value["name"],
        icon: value["icon"],
        rarity: value["rarity"],
        quantity: value["quantity"],
        equipmentSlotId: value["equipmentSlotId"],
        equippedSlotId,
      }
    : undefined;
}

function decodeEquipmentSlot(value: unknown): EquipmentSlotDto | undefined {
  if (!hasExactKeys(value, ["equipped", "id", "label"])) {
    return undefined;
  }
  const equippedValue = value["equipped"];
  const equipped =
    equippedValue === null ? null : decodeLoadoutItem(equippedValue);
  return typeof value["id"] === "string" &&
    value["id"].length > 0 &&
    typeof value["label"] === "string" &&
    equipped !== undefined
    ? { id: value["id"], label: value["label"], equipped }
    : undefined;
}

function decodeLoadoutCapacity(value: unknown): LoadoutCapacityDto | undefined {
  if (!hasExactKeys(value, ["maximum", "metric", "used"])) {
    return undefined;
  }
  return typeof value["metric"] === "string" &&
    value["metric"].length > 0 &&
    isSafeNonNegativeInteger(value["used"]) &&
    isSafeNonNegativeInteger(value["maximum"])
    ? {
        metric: value["metric"],
        used: value["used"],
        maximum: value["maximum"],
      }
    : undefined;
}

function decodeEncounterChoice(value: unknown): EncounterChoiceDto | undefined {
  if (!hasExactKeys(value, ["id", "summary", "title"])) {
    return undefined;
  }
  return typeof value["id"] === "string" &&
    typeof value["title"] === "string" &&
    typeof value["summary"] === "string"
    ? { id: value["id"], title: value["title"], summary: value["summary"] }
    : undefined;
}

function decodeAdventureChoice(value: unknown): AdventureChoiceDto | undefined {
  if (!hasExactKeys(value, ["details", "id", "summary", "title"])) {
    return undefined;
  }
  const details = decodeStrings(
    value["details"],
    D20_PROTOCOL_LIMITS.maxAdventureDetails,
  );
  return typeof value["id"] === "string" &&
    value["id"].length > 0 &&
    typeof value["title"] === "string" &&
    value["title"].length > 0 &&
    typeof value["summary"] === "string" &&
    value["summary"].length > 0 &&
    details !== undefined
    ? {
        id: value["id"],
        title: value["title"],
        summary: value["summary"],
        details,
      }
    : undefined;
}

function decodeEncounter(value: unknown): EncounterDto | undefined {
  if (
    !hasExactKeys(value, [
      "actions",
      "board",
      "currentActorId",
      "legalTargets",
      "log",
      "participants",
      "reactionPrompt",
      "round",
    ])
  ) {
    return undefined;
  }
  const participants = decodeArray(
    value["participants"],
    D20_PROTOCOL_LIMITS.maxEncounterParticipants,
    decodeEncounterParticipant,
  );
  const board = decodeTacticalBoard(value["board"]);
  const actions = decodeArray(value["actions"], 64, decodeAction);
  const legalTargets = decodeArray(
    value["legalTargets"],
    64,
    decodeActionTargets,
  );
  const log = decodeArray(value["log"], 64, decodeLogEntry);
  const promptValue = value["reactionPrompt"];
  const reactionPrompt =
    promptValue === null ? null : decodeReactionPrompt(promptValue);
  const currentActorId = value["currentActorId"];
  if (
    !isSafeNonNegativeInteger(value["round"]) ||
    (currentActorId !== null && !isSafePositiveInteger(currentActorId)) ||
    participants === undefined ||
    participants.length === 0 ||
    board === undefined ||
    actions === undefined ||
    legalTargets === undefined ||
    reactionPrompt === undefined ||
    log === undefined
  ) {
    return undefined;
  }
  const participantById = new Map(
    participants.map((participant) => [participant.character.id, participant]),
  );
  const actionIds = new Set(actions.map((action) => action.id));
  const legalActionIds = new Set(legalTargets.map((entry) => entry.actionId));
  const currentActor =
    currentActorId === null ? undefined : participantById.get(currentActorId);
  const occupied = new Set(
    participants.map((participant) => `${participant.x}:${participant.y}`),
  );
  if (
    participantById.size !== participants.length ||
    occupied.size !== participants.length ||
    participants.some(
      (participant) =>
        participant.x >= board.width ||
        participant.y >= board.height ||
        board.rows[participant.y]?.[participant.x] !== ".",
    ) ||
    new Set(participants.map((participant) => participant.faction)).size !==
      2 ||
    actionIds.size !== actions.length ||
    legalActionIds.size !== legalTargets.length ||
    actionIds.size !== legalActionIds.size ||
    [...actionIds].some((actionId) => !legalActionIds.has(actionId)) ||
    legalTargets.some(
      (entry) =>
        entry.targetIds.length === 0 ||
        new Set(entry.targetIds).size !== entry.targetIds.length ||
        entry.targetIds.some((targetId) => {
          const target = participantById.get(targetId);
          return (
            target === undefined ||
            target.defeated ||
            target.faction !== "opposition"
          );
        }),
    ) ||
    (currentActorId !== null &&
      (currentActor === undefined || currentActor.defeated)) ||
    (actions.length > 0 && currentActor?.faction !== "party") ||
    (reactionPrompt !== null &&
      (!participantById.has(reactionPrompt.actorId) ||
        !participantById.has(reactionPrompt.targetId) ||
        reactionPrompt.actorId === reactionPrompt.targetId ||
        currentActorId === null ||
        reactionPrompt.actorId !== currentActorId)) ||
    (currentActorId === null && reactionPrompt !== null) ||
    (board.legalMoves.length > 0 &&
      (currentActor?.faction !== "party" ||
        reactionPrompt !== null ||
        board.legalMoves.some((move) => {
          const start = move.route[0];
          return (
            start?.x !== currentActor.x ||
            start.y !== currentActor.y ||
            move.route
              .slice(1)
              .some((cell) => occupied.has(`${cell.x}:${cell.y}`))
          );
        })))
  ) {
    return undefined;
  }
  return {
    round: value["round"],
    currentActorId,
    board,
    participants,
    actions,
    legalTargets,
    reactionPrompt,
    log,
  };
}

function decodeEncounterParticipant(
  value: unknown,
): EncounterParticipantDto | undefined {
  if (
    !hasExactKeys(value, [
      "character",
      "defeated",
      "faction",
      "initiative",
      "x",
      "y",
    ])
  ) {
    return undefined;
  }
  const character = decodeCharacter(value["character"]);
  return character !== undefined &&
    isEncounterFaction(value["faction"]) &&
    isSafeInteger(value["initiative"]) &&
    isSafeNonNegativeInteger(value["x"]) &&
    isSafeNonNegativeInteger(value["y"]) &&
    typeof value["defeated"] === "boolean" &&
    value["defeated"] === (character.healthCurrent === 0)
    ? {
        character,
        faction: value["faction"],
        initiative: value["initiative"],
        defeated: value["defeated"],
        x: value["x"],
        y: value["y"],
      }
    : undefined;
}

function decodeTacticalBoard(value: unknown): TacticalBoardDto | undefined {
  if (!hasExactKeys(value, ["height", "legalMoves", "rows", "width"])) {
    return undefined;
  }
  const width = value["width"];
  const height = value["height"];
  const rows = decodeStrings(
    value["rows"],
    D20_PROTOCOL_LIMITS.maxTacticalBoardHeight,
  );
  const legalMoves = decodeArray(
    value["legalMoves"],
    D20_PROTOCOL_LIMITS.maxTacticalBoardCells,
    decodeTacticalMove,
  );
  if (
    !isSafePositiveInteger(width) ||
    !isSafePositiveInteger(height) ||
    width < 5 ||
    height < 5 ||
    width > D20_PROTOCOL_LIMITS.maxTacticalBoardWidth ||
    height > D20_PROTOCOL_LIMITS.maxTacticalBoardHeight ||
    width * height > D20_PROTOCOL_LIMITS.maxTacticalBoardCells ||
    rows === undefined ||
    rows.length !== height ||
    rows.some(
      (row) =>
        row.length !== width ||
        [...row].some((cell) => cell !== "#" && cell !== "."),
    ) ||
    rows[0]?.split("").some((cell) => cell !== "#") ||
    rows
      .at(-1)
      ?.split("")
      .some((cell) => cell !== "#") ||
    rows.some((row) => row[0] !== "#" || row.at(-1) !== "#") ||
    legalMoves === undefined
  ) {
    return undefined;
  }
  const destinations = new Set(legalMoves.map((move) => `${move.x}:${move.y}`));
  return destinations.size === legalMoves.length &&
    legalMoves.every(
      (move) =>
        move.x < width &&
        move.y < height &&
        rows[move.y]?.[move.x] === "." &&
        move.route.every(
          (cell) =>
            cell.x < width && cell.y < height && rows[cell.y]?.[cell.x] === ".",
        ),
    )
    ? { width, height, rows, legalMoves }
    : undefined;
}

function decodeTacticalMove(value: unknown): TacticalMoveDto | undefined {
  if (!hasExactKeys(value, ["cost", "route", "x", "y"])) {
    return undefined;
  }
  const route = decodeArray(
    value["route"],
    D20_PROTOCOL_LIMITS.maxTacticalBoardCells,
    decodeTacticalCell,
  );
  if (
    !isSafeNonNegativeInteger(value["x"]) ||
    !isSafeNonNegativeInteger(value["y"]) ||
    !isSafePositiveInteger(value["cost"]) ||
    route === undefined ||
    route.length !== value["cost"] + 1 ||
    new Set(route.map((cell) => `${cell.x}:${cell.y}`)).size !== route.length ||
    route.at(-1)?.x !== value["x"] ||
    route.at(-1)?.y !== value["y"] ||
    route.some((cell, index) => {
      const previous = route[index - 1];
      return (
        previous !== undefined &&
        (Math.abs(cell.x - previous.x) > 1 ||
          Math.abs(cell.y - previous.y) > 1 ||
          (cell.x === previous.x && cell.y === previous.y))
      );
    })
  ) {
    return undefined;
  }
  return {
    x: value["x"],
    y: value["y"],
    cost: value["cost"],
    route,
  };
}

function decodeTacticalCell(value: unknown): TacticalCellDto | undefined {
  return hasExactKeys(value, ["x", "y"]) &&
    isSafeNonNegativeInteger(value["x"]) &&
    isSafeNonNegativeInteger(value["y"])
    ? { x: value["x"], y: value["y"] }
    : undefined;
}

function decodeActionTargets(value: unknown): ActionTargetsDto | undefined {
  if (!hasExactKeys(value, ["actionId", "targetIds"])) {
    return undefined;
  }
  const targetIds = decodeArray(
    value["targetIds"],
    D20_PROTOCOL_LIMITS.maxEncounterParticipants,
    (target) => (isSafePositiveInteger(target) ? target : undefined),
  );
  return typeof value["actionId"] === "string" &&
    value["actionId"].length > 0 &&
    targetIds !== undefined
    ? { actionId: value["actionId"], targetIds }
    : undefined;
}

function isEncounterFaction(value: unknown): value is EncounterFactionDto {
  return value === "party" || value === "opposition";
}

function isEncounterOutcomeKind(
  value: unknown,
): value is EncounterOutcomeKindDto {
  return value === "victory" || value === "defeat";
}

function decodeCharacter(value: unknown): CharacterDto | undefined {
  if (
    !hasExactKeys(value, [
      "effects",
      "healthCurrent",
      "healthMaximum",
      "id",
      "level",
      "name",
      "resources",
      "title",
    ])
  ) {
    return undefined;
  }
  const resources = decodeArray(value["resources"], 64, decodeResource);
  const effects = decodeStrings(value["effects"], 64);
  if (
    !isSafePositiveInteger(value["id"]) ||
    typeof value["name"] !== "string" ||
    typeof value["title"] !== "string" ||
    !isSafeNonNegativeInteger(value["level"]) ||
    !isSafeInteger(value["healthCurrent"]) ||
    !isSafeNonNegativeInteger(value["healthMaximum"]) ||
    resources === undefined ||
    effects === undefined
  ) {
    return undefined;
  }
  return {
    id: value["id"],
    name: value["name"],
    title: value["title"],
    level: value["level"],
    healthCurrent: value["healthCurrent"],
    healthMaximum: value["healthMaximum"],
    resources,
    effects,
  };
}

function decodeResource(value: unknown): ResourceDto | undefined {
  if (!hasExactKeys(value, ["current", "id", "label", "maximum"])) {
    return undefined;
  }
  return typeof value["id"] === "string" &&
    typeof value["label"] === "string" &&
    isSafeNonNegativeInteger(value["current"]) &&
    isSafeNonNegativeInteger(value["maximum"])
    ? {
        id: value["id"],
        label: value["label"],
        current: value["current"],
        maximum: value["maximum"],
      }
    : undefined;
}

function decodeAction(value: unknown): ActionDto | undefined {
  if (
    !hasExactKeys(value, [
      "ability",
      "activation",
      "damage",
      "defense",
      "effect",
      "forcedMovement",
      "id",
      "implement",
      "label",
      "range",
      "tags",
      "target",
    ])
  ) {
    return undefined;
  }
  const activation = decodeStrings(value["activation"], 4);
  const tags = decodeStrings(value["tags"], 16);
  const effect = value["effect"];
  const implement = value["implement"];
  return typeof value["id"] === "string" &&
    typeof value["label"] === "string" &&
    typeof value["ability"] === "string" &&
    typeof value["defense"] === "string" &&
    typeof value["damage"] === "string" &&
    activation !== undefined &&
    typeof value["target"] === "string" &&
    isSafeNonNegativeInteger(value["range"]) &&
    (typeof implement === "string" || implement === null) &&
    tags !== undefined &&
    (typeof effect === "string" || effect === null) &&
    isSafeNonNegativeInteger(value["forcedMovement"])
    ? {
        id: value["id"],
        label: value["label"],
        ability: value["ability"],
        defense: value["defense"],
        damage: value["damage"],
        activation,
        target: value["target"],
        range: value["range"],
        implement,
        tags,
        effect,
        forcedMovement: value["forcedMovement"],
      }
    : undefined;
}

function decodeReaction(value: unknown): ReactionDto | undefined {
  if (
    !hasExactKeys(value, [
      "available",
      "bonus",
      "cost",
      "effect",
      "id",
      "label",
      "resource",
    ])
  ) {
    return undefined;
  }
  return typeof value["id"] === "string" &&
    typeof value["label"] === "string" &&
    typeof value["resource"] === "string" &&
    isSafeNonNegativeInteger(value["cost"]) &&
    isSafeNonNegativeInteger(value["available"]) &&
    isSafeInteger(value["bonus"]) &&
    typeof value["effect"] === "string"
    ? {
        id: value["id"],
        label: value["label"],
        resource: value["resource"],
        cost: value["cost"],
        available: value["available"],
        bonus: value["bonus"],
        effect: value["effect"],
      }
    : undefined;
}

function decodeReactionPrompt(value: unknown): ReactionPromptDto | undefined {
  if (
    !hasExactKeys(value, [
      "abilityModifier",
      "abilityScore",
      "actionId",
      "actionLabel",
      "actorId",
      "defense",
      "defenseSources",
      "reactions",
      "targetId",
      "token",
    ])
  ) {
    return undefined;
  }
  const defenseSources = decodeStrings(value["defenseSources"], 256);
  const reactions = decodeArray(value["reactions"], 64, decodeReaction);
  if (
    typeof value["token"] !== "string" ||
    !isSafePositiveInteger(value["actorId"]) ||
    !isSafePositiveInteger(value["targetId"]) ||
    typeof value["actionId"] !== "string" ||
    typeof value["actionLabel"] !== "string" ||
    !isSafeInteger(value["abilityScore"]) ||
    !isSafeInteger(value["abilityModifier"]) ||
    !isSafeInteger(value["defense"]) ||
    defenseSources === undefined ||
    reactions === undefined
  ) {
    return undefined;
  }
  return {
    token: value["token"],
    actorId: value["actorId"],
    targetId: value["targetId"],
    actionId: value["actionId"],
    actionLabel: value["actionLabel"],
    abilityScore: value["abilityScore"],
    abilityModifier: value["abilityModifier"],
    defense: value["defense"],
    defenseSources,
    reactions,
  };
}

function decodeLogEntry(value: unknown): GameLogEntryDto | undefined {
  if (
    !hasExactKeys(value, ["details", "id", "kind", "source", "text", "turn"])
  ) {
    return undefined;
  }
  const details = decodeStrings(value["details"], 32);
  return isSafePositiveInteger(value["id"]) &&
    isSafeNonNegativeInteger(value["turn"]) &&
    isLogKind(value["kind"]) &&
    typeof value["source"] === "string" &&
    typeof value["text"] === "string" &&
    details !== undefined
    ? {
        id: value["id"],
        turn: value["turn"],
        kind: value["kind"],
        source: value["source"],
        text: value["text"],
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
  return decodeArray(value, maximum, (entry) =>
    entry === null ? null : decode(entry),
  );
}

function decodeStrings(value: unknown, maximum: number): string[] | undefined {
  return Array.isArray(value) &&
    value.length <= maximum &&
    value.every((entry) => typeof entry === "string" && entry.length <= 512)
    ? value
    : undefined;
}

function hasExactKeys(
  value: unknown,
  expected: readonly string[],
): value is Readonly<Record<string, unknown>> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return false;
  }
  const actual = Object.keys(value).sort();
  const keys = [...expected].sort();
  return (
    actual.length === keys.length &&
    actual.every((key, index) => key === keys[index])
  );
}

function isSafeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value);
}

function isSafeNonNegativeInteger(value: unknown): value is number {
  return isSafeInteger(value) && value >= 0;
}

function isSafePositiveInteger(value: unknown): value is number {
  return isSafeInteger(value) && value > 0;
}

function isApiErrorKind(value: unknown): value is ApiErrorKindDto {
  return (
    value === "stale" ||
    value === "invalid" ||
    value === "invalid-slot" ||
    value === "capacity" ||
    value === "containment" ||
    value === "track-bound" ||
    value === "phase" ||
    value === "not-found" ||
    value === "persistence" ||
    value === "internal"
  );
}

function isLoadoutRarity(value: unknown): value is LoadoutRarityDto {
  return (
    value === "common" ||
    value === "uncommon" ||
    value === "rare" ||
    value === "epic"
  );
}

function isExplorationFacing(
  value: unknown,
): value is ExplorationDto["facing"] {
  return (
    value === "north" ||
    value === "east" ||
    value === "south" ||
    value === "west"
  );
}

function isLogKind(value: unknown): value is GameLogKindDto {
  return (
    value === "system" ||
    value === "reaction" ||
    value === "hit" ||
    value === "miss" ||
    value === "turn"
  );
}
