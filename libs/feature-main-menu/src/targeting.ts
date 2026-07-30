export interface TargetingProjection {
  readonly campaignId: string;
  readonly encounterId: string;
  readonly phase: string;
  readonly revision: number;
  readonly currentActorId: number | null;
  readonly currentFaction: "party" | "opposition" | null;
  readonly reactionPending: boolean;
  readonly actions: readonly {
    readonly id: string;
    readonly label: string;
  }[];
  readonly legalTargets: readonly {
    readonly actionId: string;
    readonly targetIds: readonly number[];
  }[];
}

export interface TacticalTargetingMode {
  readonly campaignId: string;
  readonly encounterId: string;
  readonly revision: number;
  readonly actorId: number;
  readonly actionId: string;
  readonly actionLabel: string;
  readonly targetIds: readonly number[];
}

export type TargetingStart =
  | { readonly ok: true; readonly mode: TacticalTargetingMode }
  | { readonly ok: false; readonly message: string };

export interface TargetingCommand {
  readonly revision: number;
  readonly actorId: number;
  readonly actionId: string;
  readonly targetId: number;
}

export function startTargeting(
  projection: TargetingProjection,
  actionId: string,
): TargetingStart {
  if (
    projection.phase !== "encounter" ||
    projection.currentFaction !== "party" ||
    projection.currentActorId === null ||
    projection.reactionPending
  ) {
    return {
      ok: false,
      message: "Actions can be targeted only during a party activation.",
    };
  }
  const action = projection.actions.find((entry) => entry.id === actionId);
  const legalTargets = projection.legalTargets.find(
    (entry) => entry.actionId === actionId,
  );
  if (action === undefined || legalTargets === undefined) {
    return {
      ok: false,
      message: "That action is no longer available in the Rust projection.",
    };
  }
  return {
    ok: true,
    mode: {
      campaignId: projection.campaignId,
      encounterId: projection.encounterId,
      revision: projection.revision,
      actorId: projection.currentActorId,
      actionId: action.id,
      actionLabel: action.label,
      targetIds: [...legalTargets.targetIds],
    },
  };
}

export function targetingIsCurrent(
  mode: TacticalTargetingMode,
  projection: TargetingProjection,
): boolean {
  const action = projection.actions.find((entry) => entry.id === mode.actionId);
  const legalTargets = projection.legalTargets.find(
    (entry) => entry.actionId === mode.actionId,
  );
  return (
    projection.phase === "encounter" &&
    projection.currentFaction === "party" &&
    !projection.reactionPending &&
    projection.campaignId === mode.campaignId &&
    projection.encounterId === mode.encounterId &&
    projection.revision === mode.revision &&
    projection.currentActorId === mode.actorId &&
    action?.label === mode.actionLabel &&
    legalTargets !== undefined &&
    sameTargets(legalTargets.targetIds, mode.targetIds)
  );
}

export function targetingCommand(
  mode: TacticalTargetingMode,
  projection: TargetingProjection,
  targetId: number,
): TargetingCommand | null {
  return targetingIsCurrent(mode, projection) &&
    mode.targetIds.includes(targetId)
    ? {
        revision: mode.revision,
        actorId: mode.actorId,
        actionId: mode.actionId,
        targetId,
      }
    : null;
}

function sameTargets(
  left: readonly number[],
  right: readonly number[],
): boolean {
  return (
    left.length === right.length &&
    left.every((target, index) => target === right[index])
  );
}
