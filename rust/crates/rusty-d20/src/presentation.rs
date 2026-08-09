//! Rust-owned projection from authoritative D20 snapshots into Engine retained frames.

use rusty_engine::{
    render_host_contracts::RendererCameraPose,
    render_model::{
        Geometry, Material, RenderDiff, RenderFrameDiff, RenderFrameError, RenderHandle,
        RenderLayer, RenderMetadata, RenderNode, Transform,
    },
};

use crate::{
    CampaignPhaseDto, EncounterFactionDto, ExplorationDto, GameSnapshotDto, TacticalBoardDto,
};

const CELL_SIZE: f32 = 2.0;
const WALL_HEIGHT: f32 = 2.5;
const WALL_THICKNESS: f32 = 0.2;
const FIRST_HANDLE: u64 = 100;
const TACTICAL_CELL_HANDLE_BASE: u64 = 10_000;
const TACTICAL_PARTICIPANT_HANDLE_BASE: u64 = 20_000;

#[derive(Debug, Clone, PartialEq)]
pub struct D20PresentationFrame {
    pub frame: RenderFrameDiff,
    pub handles: Vec<RenderHandle>,
    pub camera: RendererCameraPose,
    pub mode: &'static str,
}

pub fn create_game_frame(
    snapshot: &GameSnapshotDto,
    previous_handles: &[RenderHandle],
) -> Result<D20PresentationFrame, RenderFrameError> {
    let mut ops = previous_handles
        .iter()
        .copied()
        .map(|handle| RenderDiff::Destroy { handle })
        .collect::<Vec<_>>();
    let mut handles = Vec::new();

    match snapshot.campaign.as_ref().map(|campaign| campaign.phase) {
        Some(CampaignPhaseDto::Exploration) => {
            if let Some(exploration) = &snapshot.exploration {
                project_exploration(exploration, &mut ops, &mut handles);
                return finish(
                    ops,
                    handles,
                    RendererCameraPose {
                        position: [0.0, 1.35, 0.55],
                        pitch_degrees: 0.0,
                        yaw_degrees: 0.0,
                    },
                    "exploration",
                );
            }
        }
        Some(CampaignPhaseDto::Encounter | CampaignPhaseDto::Outcome) => {
            if let Some(encounter) = &snapshot.encounter {
                project_tactical(&encounter.board, encounter, &mut ops, &mut handles);
                let extent = f64::from(encounter.board.width.max(encounter.board.height));
                return finish(
                    ops,
                    handles,
                    RendererCameraPose {
                        position: [0.0, extent * 0.9 + 2.0, 0.0],
                        pitch_degrees: -90.0,
                        yaw_degrees: 0.0,
                    },
                    if snapshot
                        .campaign
                        .as_ref()
                        .is_some_and(|campaign| campaign.phase == CampaignPhaseDto::Outcome)
                    {
                        "outcome"
                    } else {
                        "encounter"
                    },
                );
            }
        }
        _ => {}
    }

    let mode = match snapshot.campaign.as_ref().map(|campaign| campaign.phase) {
        None => "catalog",
        Some(CampaignPhaseDto::Camp) => "camp",
        Some(CampaignPhaseDto::AdventureComplete) => "complete",
        Some(CampaignPhaseDto::Exploration) => "exploration-loading",
        Some(CampaignPhaseDto::Encounter) => "encounter-loading",
        Some(CampaignPhaseDto::Outcome) => "outcome-loading",
    };
    project_backdrop(mode, &mut ops, &mut handles);
    finish(
        ops,
        handles,
        RendererCameraPose {
            position: [0.0, 2.1, 4.8],
            pitch_degrees: -8.0,
            yaw_degrees: 0.0,
        },
        mode,
    )
}

fn finish(
    ops: Vec<RenderDiff>,
    handles: Vec<RenderHandle>,
    camera: RendererCameraPose,
    mode: &'static str,
) -> Result<D20PresentationFrame, RenderFrameError> {
    Ok(D20PresentationFrame {
        frame: RenderFrameDiff::try_from_ops(ops)?,
        handles,
        camera,
        mode,
    })
}

fn project_exploration(
    view: &ExplorationDto,
    ops: &mut Vec<RenderDiff>,
    handles: &mut Vec<RenderHandle>,
) {
    let palette = match view.wall_style.as_str() {
        "ember-vault" => ([0.17, 0.09, 0.05, 1.0], [0.46, 0.19, 0.08, 1.0]),
        "mountain-fortress" => ([0.16, 0.17, 0.16, 1.0], [0.31, 0.36, 0.37, 1.0]),
        _ => ([0.14, 0.14, 0.13, 1.0], [0.30, 0.33, 0.34, 1.0]),
    };
    let mut next = FIRST_HANDLE;
    for depth_view in &view.view {
        let depth = depth_view.depth;
        let center_z = -(f32::from(depth) * CELL_SIZE + CELL_SIZE / 2.0);
        add_cube(
            ops,
            handles,
            &mut next,
            [0.0, -WALL_THICKNESS / 2.0, center_z],
            [CELL_SIZE + WALL_THICKNESS * 2.0, WALL_THICKNESS, CELL_SIZE],
            shade(palette.0, depth),
            if depth == 0 {
                &["rusty-d20", "dungeon", "floor", "native-control"]
            } else {
                &["rusty-d20", "dungeon", "floor"]
            },
            None,
        );
        add_cube(
            ops,
            handles,
            &mut next,
            [0.0, WALL_HEIGHT + WALL_THICKNESS / 2.0, center_z],
            [CELL_SIZE + WALL_THICKNESS * 2.0, WALL_THICKNESS, CELL_SIZE],
            shade([0.11, 0.13, 0.15, 1.0], depth),
            &["rusty-d20", "dungeon", "ceiling"],
            None,
        );
        if depth_view.left_blocked {
            add_cube(
                ops,
                handles,
                &mut next,
                [
                    -(CELL_SIZE + WALL_THICKNESS) / 2.0,
                    WALL_HEIGHT / 2.0,
                    center_z,
                ],
                [WALL_THICKNESS, WALL_HEIGHT, CELL_SIZE],
                shade(palette.1, depth),
                &["rusty-d20", "dungeon", "left-wall"],
                None,
            );
        }
        if depth_view.right_blocked {
            add_cube(
                ops,
                handles,
                &mut next,
                [
                    (CELL_SIZE + WALL_THICKNESS) / 2.0,
                    WALL_HEIGHT / 2.0,
                    center_z,
                ],
                [WALL_THICKNESS, WALL_HEIGHT, CELL_SIZE],
                shade(palette.1, depth),
                &["rusty-d20", "dungeon", "right-wall"],
                None,
            );
        }
        if depth_view.front_blocked {
            add_cube(
                ops,
                handles,
                &mut next,
                [
                    0.0,
                    WALL_HEIGHT / 2.0,
                    -(f32::from(depth) + 1.0) * CELL_SIZE,
                ],
                [
                    CELL_SIZE + WALL_THICKNESS * 2.0,
                    WALL_HEIGHT,
                    WALL_THICKNESS,
                ],
                shade(palette.1, depth),
                &["rusty-d20", "dungeon", "front-wall"],
                None,
            );
            break;
        }
    }
}

fn project_tactical(
    board: &TacticalBoardDto,
    encounter: &crate::EncounterDto,
    ops: &mut Vec<RenderDiff>,
    handles: &mut Vec<RenderHandle>,
) {
    let legal_moves = board
        .legal_moves
        .iter()
        .map(|movement| ((movement.x, movement.y), movement.cost))
        .collect::<std::collections::BTreeMap<_, _>>();
    for (y, row) in board.rows.iter().enumerate() {
        for (x, cell) in row.chars().enumerate() {
            let x = u16::try_from(x).expect("bounded tactical x");
            let y = u16::try_from(y).expect("bounded tactical y");
            let handle =
                RenderHandle::new(TACTICAL_CELL_HANDLE_BASE + u64::from(y) * 32 + u64::from(x));
            handles.push(handle);
            let wall = cell == '#';
            let color = if wall {
                [0.16, 0.18, 0.18, 1.0]
            } else if legal_moves.contains_key(&(x, y)) {
                [0.12, 0.42, 0.44, 1.0]
            } else {
                [0.11, 0.15, 0.16, 1.0]
            };
            ops.push(RenderDiff::Create {
                handle,
                parent: None,
                node: node(
                    Geometry::Cube,
                    world_position(board, x, y, if wall { 0.3 } else { 0.0 }),
                    [0.77, if wall { 0.62 } else { 0.08 }, 0.77],
                    color,
                    vec![
                        "rusty-d20",
                        "tactical-cell",
                        if wall { "wall" } else { "floor" },
                    ],
                    None,
                    Some(u64::from(y) * 32 + u64::from(x)),
                    format!("Tactical cell {x},{y}"),
                ),
            });
        }
    }
    for (index, participant) in encounter.participants.iter().enumerate() {
        let handle = RenderHandle::new(
            TACTICAL_PARTICIPANT_HANDLE_BASE + u64::try_from(index).expect("bounded participant"),
        );
        handles.push(handle);
        let color = if participant.defeated {
            [0.24, 0.27, 0.27, 1.0]
        } else if Some(participant.character.id) == encounter.current_actor_id {
            [0.94, 0.67, 0.22, 1.0]
        } else if participant.faction == EncounterFactionDto::Party {
            [0.12, 0.47, 0.62, 1.0]
        } else {
            [0.66, 0.16, 0.18, 1.0]
        };
        ops.push(RenderDiff::Create {
            handle,
            parent: None,
            node: node(
                Geometry::Sphere,
                world_position(board, participant.x, participant.y, 0.4),
                [0.5, if participant.defeated { 0.18 } else { 0.5 }, 0.5],
                color,
                vec!["rusty-d20", "tactical-participant"],
                Some(participant.character.id),
                None,
                participant.character.name.clone(),
            ),
        });
    }
}

fn project_backdrop(
    mode: &'static str,
    ops: &mut Vec<RenderDiff>,
    handles: &mut Vec<RenderHandle>,
) {
    let colors = match mode {
        "camp" => ([0.12, 0.10, 0.08, 1.0], [0.72, 0.46, 0.20, 1.0]),
        "complete" => ([0.08, 0.10, 0.11, 1.0], [0.42, 0.74, 0.70, 1.0]),
        _ => ([0.08, 0.11, 0.12, 1.0], [0.35, 0.72, 0.68, 1.0]),
    };
    let mut next = FIRST_HANDLE;
    add_cube(
        ops,
        handles,
        &mut next,
        [0.0, -0.15, -5.0],
        [16.0, 0.3, 18.0],
        colors.0,
        &["rusty-d20", "backdrop"],
        None,
    );
    add_cube(
        ops,
        handles,
        &mut next,
        [0.0, 3.2, -13.5],
        [18.0, 6.5, 0.4],
        colors.0,
        &["rusty-d20", "backdrop"],
        None,
    );
    add_cube(
        ops,
        handles,
        &mut next,
        [0.0, 0.12, -6.2],
        [5.6, 0.24, 3.6],
        colors.1,
        &["rusty-d20", "backdrop", mode],
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn add_cube(
    ops: &mut Vec<RenderDiff>,
    handles: &mut Vec<RenderHandle>,
    next: &mut u64,
    translation: [f32; 3],
    scale: [f32; 3],
    color: [f32; 4],
    tags: &[&str],
    source_entity: Option<u64>,
) {
    let handle = RenderHandle::new(*next);
    *next += 1;
    handles.push(handle);
    ops.push(RenderDiff::Create {
        handle,
        parent: None,
        node: node(
            Geometry::Cube,
            translation,
            scale,
            color,
            tags.to_vec(),
            source_entity,
            None,
            tags.join("-"),
        ),
    });
}

#[allow(clippy::too_many_arguments)]
fn node(
    geometry: Geometry,
    translation: [f32; 3],
    scale: [f32; 3],
    color: [f32; 4],
    tags: Vec<&str>,
    source_entity: Option<u64>,
    source_scene_node: Option<u64>,
    label: String,
) -> RenderNode {
    let mut tags = tags.into_iter().map(str::to_owned).collect::<Vec<_>>();
    tags.sort();
    tags.dedup();
    RenderNode {
        geometry,
        material: Material {
            color,
            wireframe: false,
        },
        transform: Transform {
            translation,
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale,
        },
        visible: true,
        layer: RenderLayer::Scene,
        metadata: RenderMetadata {
            source_entity,
            source_scene_node,
            tags,
            label: Some(label),
        },
    }
}

fn world_position(board: &TacticalBoardDto, x: u16, y: u16, height: f32) -> [f32; 3] {
    [
        (f32::from(x) - (f32::from(board.width) - 1.0) / 2.0) * 0.84,
        height,
        (f32::from(y) - (f32::from(board.height) - 1.0) / 2.0) * 0.84,
    ]
}

fn shade(color: [f32; 4], depth: u16) -> [f32; 4] {
    let shade = (1.0 - f32::from(depth) * 0.14).max(0.62);
    [
        color[0] * shade,
        color[1] * shade,
        color[2] * shade,
        color[3],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExplorationCommandKindDto, ExplorationCommandRequestDto, NewAdventureRequestDto};

    #[test]
    fn rust_projection_tracks_authoritative_campaign_revisions() {
        let mut runtime = crate::GameRuntime::empty().expect("empty runtime");
        let initial = runtime.snapshot().expect("initial snapshot");
        let catalog = create_game_frame(&initial, &[]).expect("catalog frame");
        assert_eq!(catalog.mode, "catalog");

        let adventure_id = initial.available_adventures[0].id.clone();
        let camp = runtime
            .new_adventure_for(NewAdventureRequestDto {
                expected_revision: initial.revision,
                adventure_id,
            })
            .expect("new adventure");
        let exploration = runtime
            .begin_exploration(camp.revision)
            .expect("begin exploration");
        let first = create_game_frame(&exploration, &catalog.handles).expect("first dungeon frame");
        assert_eq!(first.mode, "exploration");
        assert!(first.frame.ops.iter().any(|op| matches!(op, RenderDiff::Create { node, .. } if node.metadata.tags.iter().any(|tag| tag == "native-control"))));

        let turned = runtime
            .exploration_command(ExplorationCommandRequestDto {
                expected_revision: exploration.revision,
                command: ExplorationCommandKindDto::TurnRight,
            })
            .expect("turn right");
        assert!(turned.revision > exploration.revision);
        let second = create_game_frame(&turned, &first.handles).expect("turned dungeon frame");
        assert_ne!(first.frame, second.frame);
    }
}
