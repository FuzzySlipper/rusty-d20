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
                checkpoint_id: dungeon.start_checkpoint.to_string(),
                opened_doors: BTreeSet::new(),
                collected_treasures: BTreeSet::new(),
            }
        });
        state.discovered.insert(state.position);
        campaign.phase = CampaignPhase::Exploration;
        campaign.active_encounter_id = None;
        campaign.current_actor_id = None;
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
        let completed_encounters = self
            .campaign
            .as_ref()
            .ok_or(GameRuntimeError::NoEncounter)?
            .completed_encounters
            .iter()
            .map(|completed| completed.encounter_id.clone())
            .collect::<BTreeSet<_>>();

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
                .filter(|position| can_traverse(&dungeon, state, state.position, *position))
                .ok_or_else(|| {
                    GameRuntimeError::InvalidCommand(
                        "solid dungeon stone or a closed door blocks that step".to_owned(),
                    )
                })?;
                state.position = destination;
                state.discovered.insert(destination);

                if let Some(trigger) = dungeon.encounters.iter().find(|trigger| {
                    trigger.x == destination.x
                        && trigger.y == destination.y
                        && !completed_encounters.contains(trigger.encounter.as_str())
                }) {
                    return self.enter_encounter_inner(EnterEncounterRequestDto {
                        expected_revision: request.expected_revision,
                        encounter_id: trigger.encounter.to_string(),
                    });
                }
            }
            ExplorationCommandKindDto::Interact => {
                self.interact_with_dungeon(&dungeon)?;
            }
        }

        self.bump_revision()?;
        self.saved_revision = None;
        self.snapshot()
    }

    fn interact_with_dungeon(
        &mut self,
        dungeon: &crate::DungeonDefinition,
    ) -> Result<(), GameRuntimeError> {
        let state = self.exploration()?.clone();
        if let Some(door) = door_ahead(dungeon, state.position, state.facing)
            .filter(|door| !state.opened_doors.contains(door.id.as_str()))
            .cloned()
        {
            if door
                .requires_treasure
                .as_ref()
                .is_some_and(|required| !state.collected_treasures.contains(required.as_str()))
            {
                return Err(GameRuntimeError::InvalidCommand(
                    "the authored door remains locked until its dungeon treasure is claimed"
                        .to_owned(),
                ));
            }
            self.exploration_mut()?
                .opened_doors
                .insert(door.id.to_string());
            self.push_log(
                GameLogKindDto::System,
                &door.title,
                &door.text,
                vec![format!(
                    "Door {} opened permanently from dungeon cell ({}, {}).",
                    door.id, door.x, door.y
                )],
            )?;
            return Ok(());
        }

        if let Some(treasure) = dungeon
            .treasures
            .iter()
            .find(|treasure| treasure.x == state.position.x && treasure.y == state.position.y)
            .cloned()
        {
            if state.collected_treasures.contains(treasure.id.as_str()) {
                return Err(GameRuntimeError::InvalidCommand(
                    "that dungeon treasure was already collected".to_owned(),
                ));
            }
            let adventure = self.adventure()?.clone();
            let item = self
                .rules
                .item_instance(&treasure.item)
                .expect("compiled treasure item exists")
                .clone();
            let from = owner_entity(&self.rules, &adventure, &item.owner)?;
            let to = storage_entity(&self.rules, &adventure, &adventure.camp_storage)?;
            let serial = self.next_operation;
            self.session_mut()?.transfer_item(
                EntityId::new(item.entity_id),
                from,
                to,
                operation(&format!("dungeon-treasure-{serial}"))?,
            )?;
            self.next_operation = self
                .next_operation
                .checked_add(1)
                .ok_or(GameRuntimeError::CounterOverflow)?;
            self.exploration_mut()?
                .collected_treasures
                .insert(treasure.id.to_string());
            self.push_log(
                GameLogKindDto::System,
                &treasure.title,
                &treasure.text,
                vec![format!(
                    "{} moved into canonical camp storage and cannot be collected again.",
                    item.name
                )],
            )?;
            return Ok(());
        }

        if let Some(checkpoint) = dungeon
            .checkpoints
            .iter()
            .find(|checkpoint| checkpoint.x == state.position.x && checkpoint.y == state.position.y)
            .cloned()
        {
            let campaign = self.campaign_mut()?;
            let exploration = campaign.exploration.as_mut().ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "campaign is missing its exploration state".to_owned(),
                )
            })?;
            exploration.checkpoint_id = checkpoint.id.to_string();
            campaign.phase = CampaignPhase::Camp;
            campaign.active_encounter_id = None;
            campaign.current_actor_id = None;
            self.push_log(
                GameLogKindDto::System,
                &checkpoint.title,
                &checkpoint.text,
                vec![
                    "The party returned safely to camp.".to_owned(),
                    "A later defeat returns the expedition to this durable checkpoint.".to_owned(),
                ],
            )?;
            return Ok(());
        }

        let landmark = dungeon
            .landmarks
            .iter()
            .find(|landmark| landmark.x == state.position.x && landmark.y == state.position.y)
            .ok_or_else(|| {
                GameRuntimeError::InvalidCommand(
                    "there is nothing to inspect or use at this location".to_owned(),
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
        Ok(())
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
        let view = project_view(dungeon, state);
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
        let door_ahead = door_ahead(dungeon, state.position, state.facing).map(|door| {
            let opened = state.opened_doors.contains(door.id.as_str());
            ExplorationDoorDto {
                id: door.id.to_string(),
                title: door.title.clone(),
                text: door.text.clone(),
                opened,
                locked: !opened
                    && door.requires_treasure.as_ref().is_some_and(|required| {
                        !state.collected_treasures.contains(required.as_str())
                    }),
            }
        });
        let treasure = dungeon
            .treasures
            .iter()
            .find(|treasure| treasure.x == state.position.x && treasure.y == state.position.y)
            .map(|treasure| ExplorationTreasureDto {
                id: treasure.id.to_string(),
                title: treasure.title.clone(),
                text: treasure.text.clone(),
                collected: state.collected_treasures.contains(treasure.id.as_str()),
            });
        let checkpoint = dungeon
            .checkpoints
            .iter()
            .find(|checkpoint| checkpoint.x == state.position.x && checkpoint.y == state.position.y)
            .map(|checkpoint| ExplorationCheckpointDto {
                id: checkpoint.id.to_string(),
                title: checkpoint.title.clone(),
                text: checkpoint.text.clone(),
                active: state.checkpoint_id == checkpoint.id.as_str(),
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
                .is_some_and(|position| can_traverse(dungeon, state, state.position, position)),
            can_step_backward: backward
                .is_some_and(|position| can_traverse(dungeon, state, state.position, position)),
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
            door_ahead,
            treasure,
            checkpoint,
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
            Some(
                CampaignPhase::Camp
                | CampaignPhase::Encounter
                | CampaignPhase::Outcome
                | CampaignPhase::AdventureComplete,
            ) => Err(GameRuntimeError::WrongPhase(
                "this command is only available while exploring".to_owned(),
            )),
            None => Err(GameRuntimeError::NoEncounter),
        }
    }
}

fn project_view(
    dungeon: &crate::DungeonDefinition,
    state: &ExplorationState,
) -> Vec<ExplorationDepthDto> {
    let mut occluded = false;
    (0..VIEW_DEPTH)
        .map(|depth| {
            if occluded {
                return ExplorationDepthDto {
                    depth,
                    front_blocked: true,
                    left_blocked: true,
                    right_blocked: true,
                };
            }
            let center = offset_by(state.position, state.facing, i32::from(depth));
            let (left, right) = side_positions(center, state.facing);
            let front = offset_by(center, state.facing, 1);
            let projected = ExplorationDepthDto {
                depth,
                front_blocked: !can_traverse(dungeon, state, center, front),
                left_blocked: !can_traverse(dungeon, state, center, left),
                right_blocked: !can_traverse(dungeon, state, center, right),
            };
            occluded = projected.front_blocked;
            projected
        })
        .collect()
}

fn can_traverse(
    dungeon: &crate::DungeonDefinition,
    state: &ExplorationState,
    from: DungeonPosition,
    to: DungeonPosition,
) -> bool {
    dungeon.is_floor(from.x, from.y)
        && dungeon.is_floor(to.x, to.y)
        && dungeon
            .doors
            .iter()
            .find(|door| door_connects(door, from, to))
            .is_none_or(|door| state.opened_doors.contains(door.id.as_str()))
}

fn door_ahead(
    dungeon: &crate::DungeonDefinition,
    position: DungeonPosition,
    facing: DungeonFacingDefinition,
) -> Option<&crate::DungeonDoorDefinition> {
    let destination = offset(position, facing, true)?;
    dungeon
        .doors
        .iter()
        .find(|door| door_connects(door, position, destination))
}

fn door_connects(
    door: &crate::DungeonDoorDefinition,
    left: DungeonPosition,
    right: DungeonPosition,
) -> bool {
    let origin = DungeonPosition {
        x: door.x,
        y: door.y,
    };
    door_destination(door).is_some_and(|destination| {
        (origin == left && destination == right) || (origin == right && destination == left)
    })
}

fn door_destination(door: &crate::DungeonDoorDefinition) -> Option<DungeonPosition> {
    offset(
        DungeonPosition {
            x: door.x,
            y: door.y,
        },
        door.facing,
        true,
    )
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

fn facing_dto(facing: DungeonFacingDefinition) -> ExplorationFacingDto {
    match facing {
        DungeonFacingDefinition::North => ExplorationFacingDto::North,
        DungeonFacingDefinition::East => ExplorationFacingDto::East,
        DungeonFacingDefinition::South => ExplorationFacingDto::South,
        DungeonFacingDefinition::West => ExplorationFacingDto::West,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::D20Id;

    fn dungeon(rows: &[&str]) -> crate::DungeonDefinition {
        crate::DungeonDefinition {
            title: "Occlusion probe".to_owned(),
            wall_style: D20Id::parse("stone").unwrap(),
            width: 7,
            height: 5,
            rows: rows.iter().map(|row| (*row).to_owned()).collect(),
            start_x: 1,
            start_y: 2,
            start_checkpoint: D20Id::parse("probe-camp").unwrap(),
            start_facing: DungeonFacingDefinition::East,
            encounters: Vec::new(),
            landmarks: Vec::new(),
            doors: Vec::new(),
            treasures: Vec::new(),
            checkpoints: vec![crate::DungeonCheckpointDefinition {
                id: D20Id::parse("probe-camp").unwrap(),
                x: 1,
                y: 2,
                title: "Probe camp".to_owned(),
                text: "Return safely.".to_owned(),
            }],
        }
    }

    #[test]
    fn corridor_projection_neutralizes_asymmetric_topology_behind_front_wall() {
        let open_hidden_space = dungeon(&["#######", "#.....#", "#.#...#", "#.....#", "#######"]);
        let blocked_hidden_space =
            dungeon(&["#######", "#.###.#", "#.##..#", "#.#...#", "#######"]);
        let state = ExplorationState {
            position: DungeonPosition { x: 1, y: 2 },
            facing: DungeonFacingDefinition::East,
            discovered: BTreeSet::from([DungeonPosition { x: 1, y: 2 }]),
            inspected_landmarks: BTreeSet::new(),
            checkpoint_id: "probe-camp".to_owned(),
            opened_doors: BTreeSet::new(),
            collected_treasures: BTreeSet::new(),
        };

        let open = project_view(&open_hidden_space, &state);
        let blocked = project_view(&blocked_hidden_space, &state);

        assert_eq!(open, blocked);
        assert_eq!(
            open,
            vec![
                ExplorationDepthDto {
                    depth: 0,
                    front_blocked: true,
                    left_blocked: false,
                    right_blocked: false,
                },
                ExplorationDepthDto {
                    depth: 1,
                    front_blocked: true,
                    left_blocked: true,
                    right_blocked: true,
                },
                ExplorationDepthDto {
                    depth: 2,
                    front_blocked: true,
                    left_blocked: true,
                    right_blocked: true,
                },
            ]
        );
    }
}
