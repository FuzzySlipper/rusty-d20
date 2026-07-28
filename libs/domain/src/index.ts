import type {
  ActionDto,
  CharacterDto,
  EncounterDto,
  GameLogEntryDto,
  GameSnapshotDto,
  PendingActionDto,
  RuntimeReadoutDto,
} from '@rusty-d20/protocol';

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
  readonly encounter: EncounterView | null;
}

export interface EncounterView {
  readonly turn: number;
  readonly nextRoll: number;
  readonly playerId: number;
  readonly player: CharacterDto;
  readonly targets: readonly CharacterDto[];
  readonly characters: readonly CharacterDto[];
  readonly actions: readonly ActionDto[];
  readonly pendingAction: PendingActionDto | null;
  readonly log: readonly GameLogEntryDto[];
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

export function projectGameSnapshot(snapshot: GameSnapshotDto): GameSnapshotView {
  return {
    product: snapshot.product,
    version: snapshot.version,
    engineRevision: snapshot.engineRevision,
    engineRevisionShort: snapshot.engineRevision.slice(0, 12),
    rulesetFingerprint: snapshot.rulesetFingerprint,
    rulesetFingerprintShort: snapshot.rulesetFingerprint.slice(0, 12),
    revision: snapshot.revision,
    saved: snapshot.saved,
    encounter: snapshot.encounter === null ? null : projectEncounter(snapshot.encounter),
  };
}

function projectEncounter(encounter: EncounterDto): EncounterView {
  const player = encounter.characters.find((character) => character.id === encounter.playerId);
  if (player === undefined) {
    throw new Error('Rust projection did not include the encounter player.');
  }
  return {
    turn: encounter.turn,
    nextRoll: encounter.nextRoll,
    playerId: encounter.playerId,
    player,
    targets: encounter.characters.filter((character) => character.id !== encounter.playerId),
    characters: encounter.characters,
    actions: encounter.actions,
    pendingAction: encounter.pendingAction,
    log: encounter.log,
  };
}
