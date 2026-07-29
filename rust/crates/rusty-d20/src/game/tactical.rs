use std::collections::BTreeSet;

use core_space::{
    ChunkCoord, ChunkDims, GridId, LocalVoxelCoord, VoxelCoord, VoxelGridSpec, WorldPos,
};
use core_voxel::VoxelValue;
use svc_collision::{CollisionProjection, Ray};
use svc_pathfinding::{
    build_nav_projection, find_path, NavPathOutcome, NavPathQuery, NavProjectionConfig,
};
use svc_spatial::VoxelWorld;
use svc_volume::VoxelChunk;

use crate::{
    ActionLineOfEffectDefinition, TacticalBoardDefinition, TacticalPosition,
    TacticalPositionDefinition,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TacticalRoute {
    pub(super) destination: TacticalPosition,
    pub(super) path: Vec<TacticalPosition>,
}

pub(super) fn tactical_position(position: TacticalPositionDefinition) -> TacticalPosition {
    TacticalPosition::new(position.x, position.y)
}

pub(super) fn action_is_spatially_legal(
    board: &TacticalBoardDefinition,
    actor: TacticalPosition,
    target: TacticalPosition,
    range: u16,
    line_of_effect: ActionLineOfEffectDefinition,
) -> Result<bool, String> {
    let distance = actor
        .x()
        .abs_diff(target.x())
        .max(actor.y().abs_diff(target.y()));
    if distance == 0 || distance > range {
        return Ok(false);
    }
    match line_of_effect {
        ActionLineOfEffectDefinition::Ignored => Ok(true),
        ActionLineOfEffectDefinition::Required => line_of_effect_is_clear(board, actor, target),
    }
}

pub(super) fn legal_routes(
    board: &TacticalBoardDefinition,
    occupied: &BTreeSet<TacticalPosition>,
    start: TacticalPosition,
    maximum_steps: u16,
) -> Result<Vec<TacticalRoute>, String> {
    if maximum_steps == 0 {
        return Ok(Vec::new());
    }
    let world = board_world(board, occupied)?;
    let projection = build_nav_projection(
        &world,
        NavProjectionConfig {
            agent_height_voxels: 2,
            require_solid_floor: true,
        },
    )
    .map_err(|error| format!("Engine navigation projection rejected the board: {error:?}"))?;
    let start_voxel = board_voxel(start);
    let maximum_visited = usize::from(board.width) * usize::from(board.height);
    let mut routes = Vec::new();
    for y in 0..board.height {
        for x in 0..board.width {
            let destination = TacticalPosition::new(x, y);
            if destination == start
                || occupied.contains(&destination)
                || !board.is_floor(TacticalPositionDefinition { x, y })
            {
                continue;
            }
            let readout = find_path(
                &projection,
                NavPathQuery {
                    start: start_voxel,
                    goal: board_voxel(destination),
                    max_visited: maximum_visited,
                },
            );
            let Ok(readout) = readout else {
                continue;
            };
            if readout.outcome != NavPathOutcome::Reached {
                continue;
            }
            let steps = readout.path.len().saturating_sub(1);
            if steps == 0 || steps > usize::from(maximum_steps) {
                continue;
            }
            routes.push(TacticalRoute {
                destination,
                path: readout.path.into_iter().map(voxel_position).collect(),
            });
        }
    }
    routes.sort_by(|left, right| {
        left.path
            .len()
            .cmp(&right.path.len())
            .then_with(|| left.destination.y().cmp(&right.destination.y()))
            .then_with(|| left.destination.x().cmp(&right.destination.x()))
    });
    Ok(routes)
}

pub(super) fn forced_destination(
    board: &TacticalBoardDefinition,
    occupied: &BTreeSet<TacticalPosition>,
    actor: TacticalPosition,
    target: TacticalPosition,
    maximum_steps: u16,
) -> TacticalPosition {
    let dx = i32::from(target.x()) - i32::from(actor.x());
    let dy = i32::from(target.y()) - i32::from(actor.y());
    let (step_x, step_y) = if dx.abs() >= dy.abs() {
        (dx.signum(), 0)
    } else {
        (0, dy.signum())
    };
    if step_x == 0 && step_y == 0 {
        return target;
    }
    let mut destination = target;
    for _ in 0..maximum_steps {
        let next_x = i32::from(destination.x()) + step_x;
        let next_y = i32::from(destination.y()) + step_y;
        let (Ok(next_x), Ok(next_y)) = (u16::try_from(next_x), u16::try_from(next_y)) else {
            break;
        };
        let next = TacticalPosition::new(next_x, next_y);
        if occupied.contains(&next)
            || !board.is_floor(TacticalPositionDefinition {
                x: next.x(),
                y: next.y(),
            })
        {
            break;
        }
        destination = next;
    }
    destination
}

fn line_of_effect_is_clear(
    board: &TacticalBoardDefinition,
    actor: TacticalPosition,
    target: TacticalPosition,
) -> Result<bool, String> {
    let world = board_world(board, &BTreeSet::new())?;
    let collision = CollisionProjection::build(&world);
    let origin = WorldPos::new(f64::from(actor.x()) + 0.5, 1.5, f64::from(actor.y()) + 0.5);
    let destination = WorldPos::new(
        f64::from(target.x()) + 0.5,
        1.5,
        f64::from(target.y()) + 0.5,
    );
    let direction = destination - origin;
    let distance = direction.length();
    Ok(collision
        .raycast(Ray::new(origin, direction), (distance - 0.01).max(0.01))
        .is_none())
}

fn board_world(
    board: &TacticalBoardDefinition,
    occupied: &BTreeSet<TacticalPosition>,
) -> Result<VoxelWorld, String> {
    let dimensions = ChunkDims::new(u32::from(board.width), 3, u32::from(board.height))
        .ok_or_else(|| "tactical board cannot form an Engine voxel chunk".to_owned())?;
    let grid = VoxelGridSpec::new(GridId::new(0xD20), 1.0, dimensions)
        .ok_or_else(|| "tactical board cannot form an Engine voxel grid".to_owned())?;
    let mut chunk = VoxelChunk::from_spec(&grid);
    for (y, row) in board.rows.iter().enumerate() {
        for (x, cell) in row.bytes().enumerate() {
            let x = u32::try_from(x).map_err(|_| "tactical x coordinate overflow".to_owned())?;
            let y = u32::try_from(y).map_err(|_| "tactical y coordinate overflow".to_owned())?;
            chunk
                .set(LocalVoxelCoord::new(x, 0, y), VoxelValue::solid_raw(1))
                .map_err(|error| error.to_string())?;
            if cell == b'#' {
                for height in 1..=2 {
                    chunk
                        .set(LocalVoxelCoord::new(x, height, y), VoxelValue::solid_raw(1))
                        .map_err(|error| error.to_string())?;
                }
            }
        }
    }
    for position in occupied {
        for height in 1..=2 {
            chunk
                .set(
                    LocalVoxelCoord::new(u32::from(position.x()), height, u32::from(position.y())),
                    VoxelValue::solid_raw(2),
                )
                .map_err(|error| error.to_string())?;
        }
    }
    let mut world = VoxelWorld::new(grid);
    world.insert(ChunkCoord::ORIGIN, chunk);
    Ok(world)
}

fn board_voxel(position: TacticalPosition) -> VoxelCoord {
    VoxelCoord::new(i64::from(position.x()), 1, i64::from(position.y()))
}

fn voxel_position(position: VoxelCoord) -> TacticalPosition {
    TacticalPosition::new(
        u16::try_from(position.x).expect("compiled tactical board x fits u16"),
        u16::try_from(position.z).expect("compiled tactical board y fits u16"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asymmetric_board() -> TacticalBoardDefinition {
        TacticalBoardDefinition {
            width: 7,
            height: 7,
            rows: vec![
                "#######".to_owned(),
                "#..#..#".to_owned(),
                "#..#..#".to_owned(),
                "#.....#".to_owned(),
                "#..#..#".to_owned(),
                "#..#..#".to_owned(),
                "#######".to_owned(),
            ],
            placements: Vec::new(),
        }
    }

    #[test]
    fn engine_collision_blocks_hidden_asymmetric_line_of_effect() {
        let board = asymmetric_board();
        let actor = TacticalPosition::new(1, 2);
        assert!(!action_is_spatially_legal(
            &board,
            actor,
            TacticalPosition::new(5, 2),
            8,
            ActionLineOfEffectDefinition::Required,
        )
        .unwrap());
        assert!(action_is_spatially_legal(
            &board,
            actor,
            TacticalPosition::new(1, 4),
            8,
            ActionLineOfEffectDefinition::Required,
        )
        .unwrap());
        assert!(action_is_spatially_legal(
            &board,
            actor,
            TacticalPosition::new(5, 2),
            8,
            ActionLineOfEffectDefinition::Ignored,
        )
        .unwrap());
    }

    #[test]
    fn engine_pathfinding_respects_terrain_occupancy_and_budget() {
        let board = asymmetric_board();
        let occupied = BTreeSet::from([TacticalPosition::new(2, 1)]);
        let routes = legal_routes(&board, &occupied, TacticalPosition::new(1, 1), 3).unwrap();
        assert!(!routes.is_empty());
        assert!(routes.iter().all(|route| {
            route.destination != TacticalPosition::new(2, 1)
                && route.path.len() <= 4
                && route.path.iter().all(|position| {
                    board.is_floor(TacticalPositionDefinition {
                        x: position.x(),
                        y: position.y(),
                    }) && !occupied.contains(position)
                })
        }));
        assert!(!routes
            .iter()
            .any(|route| route.destination.x() > 3 && route.destination.y() < 3));
    }
}
