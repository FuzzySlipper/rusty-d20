import type {
  ActionDto,
  CampaignDto,
  CharacterDto,
  EncounterDto,
  EncounterParticipantDto,
  GameLogEntryDto,
  GameSnapshotDto,
  PendingActionDto,
  RuntimeReadoutDto,
} from "@rusty-d20/protocol";

export interface RuntimeReadoutView {
  readonly product: string;
  readonly version: string;
  readonly engineRevision: string;
  readonly engineRevisionShort: string;
  readonly statusLabel: string;
  readonly entityCount: number;
}

export interface GameSnapshotView {
  readonly product: string;
  readonly version: string;
  readonly engineRevision: string;
  readonly engineRevisionShort: string;
  readonly rulesetFingerprint: string;
  readonly rulesetFingerprintShort: string;
  readonly revision: number;
  readonly saved: boolean;
  readonly availableAdventures: GameSnapshotDto["availableAdventures"];
  readonly campaign: CampaignView | null;
  readonly exploration: GameSnapshotDto["exploration"];
  readonly encounter: EncounterView | null;
}

export interface CampaignView {
  readonly id: string;
  readonly title: string;
  readonly phase: CampaignDto["phase"];
  readonly party: CampaignDto["party"];
  readonly activeEncounterId: string | null;
  readonly availableEncounters: CampaignDto["availableEncounters"];
  readonly latestOutcome: CampaignDto["latestOutcome"];
  readonly completedEncounters: CampaignDto["completedEncounters"];
  readonly completion: CampaignDto["completion"];
}

export interface EncounterView {
  readonly round: number;
  readonly nextRoll: number;
  readonly currentActorId: number | null;
  readonly currentActor: CharacterDto | null;
  readonly currentFaction: EncounterParticipantDto["faction"] | null;
  readonly board: EncounterDto["board"];
  readonly party: readonly CharacterDto[];
  readonly targets: readonly CharacterDto[];
  readonly participants: readonly EncounterParticipantDto[];
  readonly actions: readonly ActionDto[];
  readonly legalTargets: EncounterDto["legalTargets"];
  readonly pendingAction: PendingActionDto | null;
  readonly log: readonly GameLogEntryDto[];
}

export function projectRuntimeReadout(
  readout: RuntimeReadoutDto,
): RuntimeReadoutView {
  return {
    product: readout.product,
    version: readout.version,
    engineRevision: readout.engineRevision,
    engineRevisionShort: readout.engineRevision.slice(0, 12),
    statusLabel:
      readout.status === "ready" ? "Runtime ready" : "Runtime unavailable",
    entityCount: readout.entityCount,
  };
}

export function projectGameSnapshot(
  snapshot: GameSnapshotDto,
): GameSnapshotView {
  return {
    product: snapshot.product,
    version: snapshot.version,
    engineRevision: snapshot.engineRevision,
    engineRevisionShort: snapshot.engineRevision.slice(0, 12),
    rulesetFingerprint: snapshot.rulesetFingerprint,
    rulesetFingerprintShort: snapshot.rulesetFingerprint.slice(0, 12),
    revision: snapshot.revision,
    saved: snapshot.saved,
    availableAdventures: snapshot.availableAdventures,
    campaign:
      snapshot.campaign === null ? null : projectCampaign(snapshot.campaign),
    exploration: snapshot.exploration,
    encounter:
      snapshot.encounter === null ? null : projectEncounter(snapshot.encounter),
  };
}

function projectCampaign(campaign: CampaignDto): CampaignView {
  return {
    id: campaign.id,
    title: campaign.title,
    phase: campaign.phase,
    party: campaign.party,
    activeEncounterId: campaign.activeEncounterId,
    availableEncounters: campaign.availableEncounters,
    latestOutcome: campaign.latestOutcome,
    completedEncounters: campaign.completedEncounters,
    completion: campaign.completion,
  };
}

function projectEncounter(encounter: EncounterDto): EncounterView {
  const current = encounter.participants.find(
    (participant) => participant.character.id === encounter.currentActorId,
  );
  return {
    round: encounter.round,
    nextRoll: encounter.nextRoll,
    currentActorId: encounter.currentActorId,
    currentActor: current?.character ?? null,
    currentFaction: current?.faction ?? null,
    board: encounter.board,
    party: encounter.participants
      .filter((participant) => participant.faction === "party")
      .map((participant) => participant.character),
    targets: encounter.participants
      .filter(
        (participant) =>
          participant.faction === "opposition" && !participant.defeated,
      )
      .map((participant) => participant.character),
    participants: encounter.participants,
    actions: encounter.actions,
    legalTargets: encounter.legalTargets,
    pendingAction: encounter.pendingAction,
    log: encounter.log,
  };
}
