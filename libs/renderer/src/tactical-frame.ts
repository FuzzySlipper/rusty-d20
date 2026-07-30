import {
  renderHandle,
  type RenderDiff,
  type RenderFrameDiff,
  type RenderHandle,
  type Vec4,
} from "@rusty-engine/render-contracts";

export interface TacticalBoardCellView {
  readonly id: string;
  readonly x: number;
  readonly y: number;
  readonly terrain: "floor" | "wall";
  readonly participantId: number | null;
  readonly participantName: string | null;
  readonly faction: "party" | "opposition" | null;
  readonly defeated: boolean;
  readonly current: boolean;
  readonly selectedTarget: boolean;
  readonly selectable: boolean;
  readonly legalMoveCost: number | null;
  readonly route: readonly TacticalCellCoordinate[] | null;
}

export interface TacticalCellCoordinate {
  readonly x: number;
  readonly y: number;
}

export interface TacticalBoardView {
  readonly width: number;
  readonly height: number;
  readonly cells: readonly TacticalBoardCellView[];
}

export interface TacticalBoardSelection {
  readonly x: number;
  readonly y: number;
  readonly participantId: number | null;
}

export interface TacticalScenePick {
  readonly handle: RenderHandle;
  readonly identity: string;
  readonly label: string;
  readonly selection: TacticalBoardSelection;
}

export interface TacticalCameraFit {
  readonly width: number;
  readonly height: number;
}

export interface TacticalRenderFrame {
  readonly frame: RenderFrameDiff;
  readonly handles: readonly RenderHandle[];
  readonly picks: readonly TacticalScenePick[];
  readonly cameraFit: TacticalCameraFit;
}

const MAX_BOARD_WIDTH = 16;
const CELL_HANDLE_BASE = 10_000;
const PARTICIPANT_HANDLE_BASE = 20_000;
const ACTIVE_MARKER_HANDLE_BASE = 21_000;
const ROUTE_HANDLE_BASE = 30_000;
const CELL_SIZE = 0.84;

const COLORS = {
  active: [0.94, 0.67, 0.22, 1],
  defeated: [0.24, 0.27, 0.27, 1],
  floor: [0.11, 0.15, 0.16, 1],
  legal: [0.12, 0.42, 0.44, 1],
  opposition: [0.66, 0.16, 0.18, 1],
  party: [0.12, 0.47, 0.62, 1],
  route: [0.3, 0.86, 0.8, 1],
  selected: [0.95, 0.78, 0.22, 1],
  wall: [0.16, 0.18, 0.18, 1],
} as const satisfies Readonly<Record<string, Vec4>>;

export function createTacticalRenderFrame(
  view: TacticalBoardView,
  previousHandles: readonly RenderHandle[] = [],
): TacticalRenderFrame {
  const ops: RenderDiff[] = previousHandles.map((handle) => ({
    op: "destroy",
    handle,
  }));
  const handles: RenderHandle[] = [];
  const picks: TacticalScenePick[] = [];

  for (const cell of view.cells) {
    const handle = cellHandle(cell.x, cell.y);
    const isWall = cell.terrain === "wall";
    const color = isWall
      ? COLORS.wall
      : cell.selectedTarget
        ? COLORS.selected
        : cell.legalMoveCost === null
          ? COLORS.floor
          : COLORS.legal;
    handles.push(handle);
    picks.push({
      handle,
      identity: `cell:${cell.x}:${cell.y}`,
      label: cellLabel(cell),
      selection: {
        x: cell.x,
        y: cell.y,
        participantId: cell.participantId,
      },
    });
    ops.push({
      op: "create",
      handle,
      parent: null,
      node: {
        geometry: { kind: "cube" },
        material: { color, wireframe: false },
        transform: {
          translation: [
            cellWorldX(view, cell.x),
            isWall ? 0.3 : 0,
            cellWorldZ(view, cell.y),
          ],
          rotation: [0, 0, 0, 1],
          scale: [CELL_SIZE * 0.92, isWall ? 0.62 : 0.08, CELL_SIZE * 0.92],
        },
        visible: true,
        layer: "scene",
        metadata: {
          sourceEntity: null,
          sourceSceneNode: cell.y * MAX_BOARD_WIDTH + cell.x,
          tags: [
            "rusty-d20",
            "tactical-board",
            "tactical-pickable",
            "tactical-cell",
            cell.terrain,
            cell.legalMoveCost === null ? "static" : "legal-move",
          ],
          label: `tactical-cell-${cell.x}-${cell.y}`,
        },
      },
    });
  }

  const routeEdges = collectRouteEdges(view);
  routeEdges.forEach((edge, index) => {
    const handle = renderHandle(ROUTE_HANDLE_BASE + index);
    handles.push(handle);
    ops.push({
      op: "create",
      handle,
      parent: null,
      node: {
        geometry: {
          kind: "line",
          a: [
            cellWorldX(view, edge.from.x),
            0.12,
            cellWorldZ(view, edge.from.y),
          ],
          b: [cellWorldX(view, edge.to.x), 0.12, cellWorldZ(view, edge.to.y)],
        },
        material: { color: COLORS.route, wireframe: false },
        transform: {
          translation: [0, 0, 0],
          rotation: [0, 0, 0, 1],
          scale: [1, 1, 1],
        },
        visible: true,
        layer: "scene",
        metadata: {
          sourceEntity: null,
          sourceSceneNode: null,
          tags: ["rusty-d20", "tactical-board", "movement-route"],
          label: `tactical-route-${edge.key}`,
        },
      },
    });
  });

  const participants = [...view.cells]
    .filter(
      (cell): cell is TacticalBoardCellView & { participantId: number } =>
        cell.participantId !== null,
    )
    .sort((left, right) => left.participantId - right.participantId);
  participants.forEach((cell, index) => {
    const handle = renderHandle(PARTICIPANT_HANDLE_BASE + index);
    const color = cell.defeated
      ? COLORS.defeated
      : cell.selectedTarget
        ? COLORS.selected
        : cell.current
          ? COLORS.active
          : cell.faction === "party"
            ? COLORS.party
            : COLORS.opposition;
    handles.push(handle);
    picks.push({
      handle,
      identity: `entity:${cell.participantId}`,
      label: cellLabel(cell),
      selection: {
        x: cell.x,
        y: cell.y,
        participantId: cell.participantId,
      },
    });
    ops.push({
      op: "create",
      handle,
      parent: null,
      node: {
        geometry: { kind: "sphere" },
        material: { color, wireframe: false },
        transform: {
          translation: [
            cellWorldX(view, cell.x),
            0.4,
            cellWorldZ(view, cell.y),
          ],
          rotation: [0, 0, 0, 1],
          scale: [0.5, cell.defeated ? 0.18 : 0.5, 0.5],
        },
        visible: true,
        layer: "scene",
        metadata: {
          sourceEntity: cell.participantId,
          sourceSceneNode: null,
          tags: [
            "rusty-d20",
            "tactical-board",
            "tactical-pickable",
            "tactical-participant",
            cell.faction ?? "unknown-faction",
            cell.current ? "active" : "waiting",
            cell.defeated ? "defeated" : "standing",
            cell.selectedTarget ? "selected-target" : "not-selected",
          ],
          label: `tactical-entity-${cell.participantId}`,
        },
      },
    });

    if (cell.current || cell.selectedTarget) {
      const markerHandle = renderHandle(ACTIVE_MARKER_HANDLE_BASE + index);
      handles.push(markerHandle);
      ops.push({
        op: "create",
        handle: markerHandle,
        parent: null,
        node: {
          geometry: { kind: "sphere" },
          material: {
            color: cell.selectedTarget ? COLORS.selected : COLORS.active,
            wireframe: true,
          },
          transform: {
            translation: [
              cellWorldX(view, cell.x),
              0.4,
              cellWorldZ(view, cell.y),
            ],
            rotation: [0, 0, 0, 1],
            scale: [0.66, 0.66, 0.66],
          },
          visible: true,
          layer: "scene",
          metadata: {
            sourceEntity: cell.participantId,
            sourceSceneNode: null,
            tags: [
              "rusty-d20",
              "tactical-board",
              cell.selectedTarget ? "selected-target-marker" : "active-marker",
            ],
            label: `tactical-marker-${cell.participantId}`,
          },
        },
      });
    }
  });

  return {
    frame: { schemaVersion: 1, ops },
    handles: [...handles].sort((left, right) => left - right),
    picks: [...picks].sort((left, right) => left.handle - right.handle),
    cameraFit: {
      width: view.width * CELL_SIZE,
      height: view.height * CELL_SIZE,
    },
  };
}

export function tacticalSelectionAt(
  view: TacticalBoardView,
  x: number,
  y: number,
): TacticalBoardSelection | null {
  const cell = view.cells.find((entry) => entry.x === x && entry.y === y);
  return cell === undefined
    ? null
    : { x: cell.x, y: cell.y, participantId: cell.participantId };
}

export function tacticalCellLabel(
  view: TacticalBoardView,
  x: number,
  y: number,
): string | null {
  const cell = view.cells.find((entry) => entry.x === x && entry.y === y);
  return cell === undefined ? null : cellLabel(cell);
}

function cellHandle(x: number, y: number): RenderHandle {
  return renderHandle(CELL_HANDLE_BASE + y * MAX_BOARD_WIDTH + x);
}

function cellWorldX(view: TacticalBoardView, x: number): number {
  return (x - (view.width - 1) / 2) * CELL_SIZE;
}

function cellWorldZ(view: TacticalBoardView, y: number): number {
  return (y - (view.height - 1) / 2) * CELL_SIZE;
}

function cellLabel(cell: TacticalBoardCellView): string {
  if (cell.terrain === "wall") {
    return `Wall at ${cell.x}, ${cell.y}`;
  }
  if (cell.participantName !== null) {
    return `${cell.participantName}, ${cell.faction}, at ${cell.x}, ${cell.y}${
      cell.current ? ", acting" : ""
    }${cell.defeated ? ", defeated" : ""}`;
  }
  return cell.legalMoveCost === null
    ? `Open terrain at ${cell.x}, ${cell.y}`
    : `Move to ${cell.x}, ${cell.y}, cost ${cell.legalMoveCost}`;
}

interface RouteEdge {
  readonly key: string;
  readonly from: TacticalCellCoordinate;
  readonly to: TacticalCellCoordinate;
}

function collectRouteEdges(view: TacticalBoardView): readonly RouteEdge[] {
  const edges = new Map<string, RouteEdge>();
  for (const cell of view.cells) {
    const route = cell.route;
    if (route === null) {
      continue;
    }
    for (let index = 1; index < route.length; index += 1) {
      const left = route[index - 1];
      const right = route[index];
      if (left === undefined || right === undefined) {
        continue;
      }
      const leftKey = `${left.x}:${left.y}`;
      const rightKey = `${right.x}:${right.y}`;
      const key =
        leftKey < rightKey
          ? `${leftKey}-${rightKey}`
          : `${rightKey}-${leftKey}`;
      edges.set(key, { key, from: left, to: right });
    }
  }
  return [...edges.values()].sort((left, right) =>
    left.key.localeCompare(right.key),
  );
}
