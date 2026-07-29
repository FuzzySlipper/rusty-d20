use super::*;

const VIEW_DEPTH: u16 = 3;

impl GameRuntime {
    pub fn begin_exploration(
        &mut self,
        expected_revision: u64,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        let snapshot = staged.begin_exploration_inner(expected_revision)?;
        *self = staged;
        Ok(snapshot)
    }

    fn begin_exploration_inner(
        &mut self,
        expected_revision: u64,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(expected_revision)?;
        self.ensure_camp_phase()?;
        self.ensure_mutation_capacity(false, true)?;
        let dungeon = self.adventure()?.dungeon.clone();
        let campaign = self.campaign_mut()?;
        let state = campaign.exploration.get_or_insert_with(|| {
            let position = DungeonPosition {
                x: dungeon.start_x,
                y: dungeon.start_y,
            };
            ExplorationState {
                position,
                facing: dungeon.start_facing,
                discovered: BTreeSet::from([position]),
                inspected_landmarks: BTreeSet::new(),
            }
        });
        state.discovered.insert(state.position);
        campaign.phase = CampaignPhase::Exploration;
        campaign.active_encounter_id = None;
        campaign.turn_owner = None;
        self.bump_revision()?;
        self.saved_revision = None;
        self.push_log(
            GameLogKindDto::System,
            "Expedition",
            &format!("The party enters {}.", dungeon.title),
            vec![
                "Dungeon position, facing, discoveries, and events are Rust-owned.".to_owned(),
                "Encounters begin only when the party reaches their authored trigger.".to_owned(),
            ],
        )?;
        self.snapshot()
    }

    pub fn exploration_command(
        &mut self,
        request: ExplorationCommandRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        let snapshot = staged.exploration_command_inner(request)?;
        *self = staged;
        Ok(snapshot)
    }

    fn exploration_command_inner(
        &mut self,
        request: ExplorationCommandRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_exploration_phase()?;
        self.ensure_mutation_capacity(false, true)?;
        let dungeon = self.adventure()?.dungeon.clone();

        match request.command {
            ExplorationCommandKindDto::TurnLeft | ExplorationCommandKindDto::TurnRight => {
                let state = self.exploration_mut()?;
                state.facing = turn(
                    state.facing,
                    request.command == ExplorationCommandKindDto::TurnRight,
                );
            }
            ExplorationCommandKindDto::StepForward | ExplorationCommandKindDto::StepBackward => {
                let state = self.exploration_mut()?;
                let destination = offset(
                    state.position,
                    state.facing,
                    request.command == ExplorationCommandKindDto::StepForward,
                )
                .filter(|position| dungeon.is_floor(position.x, position.y))
                .ok_or_else(|| {
                    GameRuntimeError::InvalidCommand(
                        "solid dungeon stone blocks that step".to_owned(),
                    )
                })?;
                state.position = destination;
                state.discovered.insert(destination);

                if let Some(trigger) = dungeon
                    .encounters
                    .iter()
                    .find(|trigger| trigger.x == destination.x && trigger.y == destination.y)
                {
                    return self.enter_encounter_inner(EnterEncounterRequestDto {
                        expected_revision: request.expected_revision,
                        encounter_id: trigger.encounter.to_string(),
                    });
                }
            }
            ExplorationCommandKindDto::Interact => {
                let position = self.exploration()?.position;
                let landmark = dungeon
                    .landmarks
                    .iter()
                    .find(|landmark| landmark.x == position.x && landmark.y == position.y)
                    .ok_or_else(|| {
                        GameRuntimeError::InvalidCommand(
                            "there is nothing to inspect at this location".to_owned(),
                        )
                    })?
                    .clone();
                self.exploration_mut()?
                    .inspected_landmarks
                    .insert(landmark.id.to_string());
                self.push_log(
                    GameLogKindDto::System,
                    &landmark.title,
                    &landmark.text,
                    vec![format!(
                        "Inspected at dungeon cell ({}, {}).",
                        landmark.x, landmark.y
                    )],
                )?;
            }
        }

        self.bump_revision()?;
        self.saved_revision = None;
        self.snapshot()
    }

    pub(super) fn project_exploration(
        &self,
        state: &ExplorationState,
    ) -> Result<ExplorationDto, GameRuntimeError> {
        let dungeon = &self.adventure()?.dungeon;
        if !dungeon.is_floor(state.position.x, state.position.y) {
            return Err(GameRuntimeError::InvalidState(
                "exploration position is not traversable".to_owned(),
            ));
        }
        let view = (0..VIEW_DEPTH)
            .map(|depth| {
                let center = offset_by(state.position, state.facing, i32::from(depth));
                let (left, right) = side_positions(center, state.facing);
                let front = offset_by(center, state.facing, 1);
                ExplorationDepthDto {
                    depth,
                    front_blocked: !is_floor(dungeon, front),
                    left_blocked: !is_floor(dungeon, left),
                    right_blocked: !is_floor(dungeon, right),
                }
            })
            .collect();
        let backward = offset(state.position, state.facing, false);
        let forward = offset(state.position, state.facing, true);
        let landmark = dungeon
            .landmarks
            .iter()
            .find(|landmark| landmark.x == state.position.x && landmark.y == state.position.y)
            .map(|landmark| ExplorationLandmarkDto {
                id: landmark.id.to_string(),
                title: landmark.title.clone(),
                text: landmark.text.clone(),
                inspected: state.inspected_landmarks.contains(landmark.id.as_str()),
            });
        Ok(ExplorationDto {
            dungeon_title: dungeon.title.clone(),
            wall_style: dungeon.wall_style.to_string(),
            width: dungeon.width,
            height: dungeon.height,
            x: state.position.x,
            y: state.position.y,
            facing: facing_dto(state.facing),
            can_step_forward: forward
                .is_some_and(|position| dungeon.is_floor(position.x, position.y)),
            can_step_backward: backward
                .is_some_and(|position| dungeon.is_floor(position.x, position.y)),
            view,
            discovered_cells: state
                .discovered
                .iter()
                .map(|position| DiscoveredCellDto {
                    x: position.x,
                    y: position.y,
                })
                .collect(),
            landmark,
        })
    }

    fn exploration(&self) -> Result<&ExplorationState, GameRuntimeError> {
        self.campaign
            .as_ref()
            .and_then(|campaign| campaign.exploration.as_ref())
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "campaign is missing its exploration state".to_owned(),
                )
            })
    }

    fn exploration_mut(&mut self) -> Result<&mut ExplorationState, GameRuntimeError> {
        self.campaign
            .as_mut()
            .and_then(|campaign| campaign.exploration.as_mut())
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "campaign is missing its exploration state".to_owned(),
                )
            })
    }

    fn ensure_exploration_phase(&self) -> Result<(), GameRuntimeError> {
        match self.campaign.as_ref().map(|campaign| campaign.phase) {
            Some(CampaignPhase::Exploration) => Ok(()),
            Some(CampaignPhase::Camp | CampaignPhase::Encounter | CampaignPhase::Outcome) => {
                Err(GameRuntimeError::WrongPhase(
                    "this command is only available while exploring".to_owned(),
                ))
            }
            None => Err(GameRuntimeError::NoEncounter),
        }
    }
}

fn turn(facing: DungeonFacingDefinition, clockwise: bool) -> DungeonFacingDefinition {
    match (facing, clockwise) {
        (DungeonFacingDefinition::North, true) | (DungeonFacingDefinition::South, false) => {
            DungeonFacingDefinition::East
        }
        (DungeonFacingDefinition::East, true) | (DungeonFacingDefinition::West, false) => {
            DungeonFacingDefinition::South
        }
        (DungeonFacingDefinition::South, true) | (DungeonFacingDefinition::North, false) => {
            DungeonFacingDefinition::West
        }
        (DungeonFacingDefinition::West, true) | (DungeonFacingDefinition::East, false) => {
            DungeonFacingDefinition::North
        }
    }
}

fn offset(
    position: DungeonPosition,
    facing: DungeonFacingDefinition,
    forward: bool,
) -> Option<DungeonPosition> {
    let distance = if forward { 1 } else { -1 };
    let position = offset_by(position, facing, distance);
    (position.x != u16::MAX && position.y != u16::MAX).then_some(position)
}

fn offset_by(
    position: DungeonPosition,
    facing: DungeonFacingDefinition,
    distance: i32,
) -> DungeonPosition {
    let (dx, dy) = match facing {
        DungeonFacingDefinition::North => (0, -distance),
        DungeonFacingDefinition::East => (distance, 0),
        DungeonFacingDefinition::South => (0, distance),
        DungeonFacingDefinition::West => (-distance, 0),
    };
    let x = i32::from(position.x) + dx;
    let y = i32::from(position.y) + dy;
    DungeonPosition {
        x: u16::try_from(x).unwrap_or(u16::MAX),
        y: u16::try_from(y).unwrap_or(u16::MAX),
    }
}

fn side_positions(
    position: DungeonPosition,
    facing: DungeonFacingDefinition,
) -> (DungeonPosition, DungeonPosition) {
    (
        offset_by(position, turn(facing, false), 1),
        offset_by(position, turn(facing, true), 1),
    )
}

fn is_floor(dungeon: &crate::DungeonDefinition, position: DungeonPosition) -> bool {
    dungeon.is_floor(position.x, position.y)
}

fn facing_dto(facing: DungeonFacingDefinition) -> ExplorationFacingDto {
    match facing {
        DungeonFacingDefinition::North => ExplorationFacingDto::North,
        DungeonFacingDefinition::East => ExplorationFacingDto::East,
        DungeonFacingDefinition::South => ExplorationFacingDto::South,
        DungeonFacingDefinition::West => ExplorationFacingDto::West,
    }
}
