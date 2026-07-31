export interface MovementCoordinate {
  readonly x: number;
  readonly y: number;
}

export interface ProjectedTacticalMove {
  readonly x: number;
  readonly y: number;
  readonly cost: number;
  readonly route: readonly MovementCoordinate[];
}

export interface MovementProjection {
  readonly campaignId: string;
  readonly encounterId: string;
  readonly phase: string;
  readonly revision: number;
  readonly currentActorId: number | null;
  readonly currentFaction: "party" | "opposition" | null;
  readonly reactionPending: boolean;
  readonly legalMoves: readonly ProjectedTacticalMove[];
}

export interface TacticalMovementMode {
  readonly campaignId: string;
  readonly encounterId: string;
  readonly revision: number;
  readonly actorId: number;
  readonly moves: readonly ProjectedTacticalMove[];
  readonly preview: ProjectedTacticalMove | null;
}

export type MovementStart =
  | { readonly ok: true; readonly mode: TacticalMovementMode }
  | { readonly ok: false; readonly message: string };

export type MovementSelection =
  | {
      readonly kind: "preview";
      readonly mode: TacticalMovementMode;
      readonly destination: ProjectedTacticalMove;
    }
  | {
      readonly kind: "confirm";
      readonly command: {
        readonly revision: number;
        readonly actorId: number;
        readonly x: number;
        readonly y: number;
      };
      readonly destination: ProjectedTacticalMove;
    }
  | { readonly kind: "rejected"; readonly message: string };

export function startMovement(projection: MovementProjection): MovementStart {
  if (
    projection.phase !== "encounter" ||
    projection.currentFaction !== "party" ||
    projection.currentActorId === null ||
    projection.reactionPending
  ) {
    return {
      ok: false,
      message: "Movement can be selected only during a party activation.",
    };
  }
  if (projection.legalMoves.length === 0) {
    return {
      ok: false,
      message: "Rust projects no legal movement destinations.",
    };
  }
  return {
    ok: true,
    mode: {
      campaignId: projection.campaignId,
      encounterId: projection.encounterId,
      revision: projection.revision,
      actorId: projection.currentActorId,
      moves: cloneMoves(projection.legalMoves),
      preview: null,
    },
  };
}

export function movementIsCurrent(
  mode: TacticalMovementMode,
  projection: MovementProjection,
): boolean {
  return (
    projection.phase === "encounter" &&
    projection.currentFaction === "party" &&
    !projection.reactionPending &&
    projection.campaignId === mode.campaignId &&
    projection.encounterId === mode.encounterId &&
    projection.revision === mode.revision &&
    projection.currentActorId === mode.actorId &&
    sameMoves(projection.legalMoves, mode.moves)
  );
}

export function selectMovementDestination(
  mode: TacticalMovementMode,
  projection: MovementProjection,
  x: number,
  y: number,
): MovementSelection {
  if (!movementIsCurrent(mode, projection)) {
    return {
      kind: "rejected",
      message:
        "Movement was canceled because the authoritative encounter changed.",
    };
  }
  const destination = mode.moves.find((move) => move.x === x && move.y === y);
  if (destination === undefined) {
    return {
      kind: "rejected",
      message: "That cell is not a Rust-projected legal movement destination.",
    };
  }
  if (mode.preview?.x === x && mode.preview.y === y) {
    return {
      kind: "confirm",
      command: {
        revision: mode.revision,
        actorId: mode.actorId,
        x,
        y,
      },
      destination,
    };
  }
  return {
    kind: "preview",
    mode: {
      ...mode,
      preview: cloneMove(destination),
    },
    destination,
  };
}

function cloneMoves(
  moves: readonly ProjectedTacticalMove[],
): readonly ProjectedTacticalMove[] {
  return moves.map(cloneMove);
}

function cloneMove(move: ProjectedTacticalMove): ProjectedTacticalMove {
  return {
    x: move.x,
    y: move.y,
    cost: move.cost,
    route: move.route.map((coordinate) => ({ ...coordinate })),
  };
}

function sameMoves(
  left: readonly ProjectedTacticalMove[],
  right: readonly ProjectedTacticalMove[],
): boolean {
  return (
    left.length === right.length &&
    left.every((move, index) => {
      const other = right[index];
      return (
        other !== undefined &&
        move.x === other.x &&
        move.y === other.y &&
        move.cost === other.cost &&
        move.route.length === other.route.length &&
        move.route.every(
          (coordinate, routeIndex) =>
            coordinate.x === other.route[routeIndex]?.x &&
            coordinate.y === other.route[routeIndex]?.y,
        )
      );
    })
  );
}
