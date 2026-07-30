use serde_json::json;

use super::*;
use crate::StaticActionRoll;
use gameplay_rules::decode_canonical_rule_package;

const PLAYER: EntityId = EntityId::new(101);
const OPPONENT: EntityId = EntityId::new(102);
const CAMP_STASH: EntityId = EntityId::new(103);
const OPPONENT_ARMOR: EntityId = EntityId::new(201);
const PLAYER_CHAIN_ARMOR: EntityId = EntityId::new(202);
const PLAYER_BUCKLER: EntityId = EntityId::new(203);
const STASH_BUCKLER: EntityId = EntityId::new(204);
const OPPONENT_BLADE: EntityId = EntityId::new(205);
const OPPONENT_BOW: EntityId = EntityId::new(207);
const ENCOUNTER_ID: &str = "iron-warden";
const DEFEAT_RECOVERY_VITALITY: u32 = 12;

fn start_test_encounter(runtime: &mut GameRuntime) -> GameSnapshotDto {
    let camp = runtime.new_adventure(runtime.revision).unwrap();
    assert_eq!(
        camp.campaign.as_ref().unwrap().phase,
        CampaignPhaseDto::Camp
    );
    runtime
        .enter_encounter(EnterEncounterRequestDto {
            expected_revision: camp.revision,
            encounter_id: ENCOUNTER_ID.to_owned(),
        })
        .unwrap()
}

fn defense_value(loadout: &LoadoutDto, defense: &str) -> i64 {
    loadout
        .defenses
        .iter()
        .find(|readout| readout.id == defense)
        .unwrap_or_else(|| panic!("missing {defense} defense readout"))
        .value
}

fn subset_party_catalog() -> AuthoredAdventureCatalog {
    let mut artifact: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../rules/artifacts/starter/catalog.json"
    ))
    .unwrap();
    let packages = artifact["packages"].as_array_mut().unwrap();
    let mut replaced = false;
    for canonical in packages {
        let package =
            decode_canonical_rule_package(canonical.as_str().unwrap().as_bytes()).unwrap();
        if package.identity().package().as_str() != "wardens-gate" {
            continue;
        }
        let mut candidate: crate::D20RulesCandidate =
            serde_json::from_value(package.payload().clone()).unwrap();
        let encounter = candidate
            .encounters
            .iter_mut()
            .find(|encounter| encounter.id.as_str() == ENCOUNTER_ID)
            .unwrap();
        encounter.roster.retain(|participant| {
            participant.faction == crate::EncounterFactionCandidate::Opposition
                || participant.character.as_str() == "mara-venn"
        });
        encounter.board.placements.retain(|placement| {
            encounter
                .roster
                .iter()
                .any(|participant| participant.character == placement.character)
        });
        let admitted = crate::admit_d20_candidate(
            crate::D20PackageEnvelope {
                domain: package.identity().domain().clone(),
                package: package.identity().package().clone(),
                version: package.identity().version(),
                dependencies: package.dependencies().to_vec(),
                sources: package.sources().to_vec(),
                provenance: package.provenance().to_vec(),
            },
            candidate,
        )
        .unwrap();
        *canonical = json!(String::from_utf8(admitted.canonical_bytes().to_vec()).unwrap());
        replaced = true;
    }
    assert!(replaced, "the built-in Warden package must be rewritten");
    AuthoredAdventureCatalog::decode(&serde_json::to_string(&artifact).unwrap()).unwrap()
}

fn disconnected_floor_catalog() -> AuthoredAdventureCatalog {
    let mut artifact: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../rules/artifacts/starter/catalog.json"
    ))
    .unwrap();
    let packages = artifact["packages"].as_array_mut().unwrap();
    let mut replaced = false;
    for canonical in packages {
        let package =
            decode_canonical_rule_package(canonical.as_str().unwrap().as_bytes()).unwrap();
        if package.identity().package().as_str() != "wardens-gate" {
            continue;
        }
        let mut candidate: crate::D20RulesCandidate =
            serde_json::from_value(package.payload().clone()).unwrap();
        let encounter = candidate
            .encounters
            .iter_mut()
            .find(|encounter| encounter.id.as_str() == ENCOUNTER_ID)
            .unwrap();
        encounter.board.rows[5] = "#........###".to_owned();
        encounter.board.rows[6] = "#.......##.#".to_owned();
        let admitted = crate::admit_d20_candidate(
            crate::D20PackageEnvelope {
                domain: package.identity().domain().clone(),
                package: package.identity().package().clone(),
                version: package.identity().version(),
                dependencies: package.dependencies().to_vec(),
                sources: package.sources().to_vec(),
                provenance: package.provenance().to_vec(),
            },
            candidate,
        )
        .unwrap();
        *canonical = json!(String::from_utf8(admitted.canonical_bytes().to_vec()).unwrap());
        replaced = true;
    }
    assert!(replaced, "the built-in Warden package must be rewritten");
    AuthoredAdventureCatalog::decode(&serde_json::to_string(&artifact).unwrap()).unwrap()
}

fn subset_party_runtime() -> GameRuntime {
    let catalog = subset_party_catalog();
    let adventure = id("wardens-gate").unwrap();
    let rules = catalog.rules_for(&adventure).unwrap();
    GameRuntime::empty_with_rules(catalog, rules, adventure, RollSourceConfig::default()).unwrap()
}

fn saved_activation_budgets(
    save: &mut serde_json::Value,
    entity: u64,
) -> &mut Vec<serde_json::Value> {
    let component = save["session"]["entityState"]["registeredComponents"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|component| component["typeId"] == "rusty-d20.activation-budgets")
        .unwrap();
    component["values"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|value| value["entity"] == entity)
        .unwrap()["value"]["budgets"]
        .as_array_mut()
        .unwrap()
}

#[test]
fn multi_party_roster_initiative_and_activation_budgets_are_canonical() {
    let mut runtime = GameRuntime::empty().unwrap();
    let started = start_test_encounter(&mut runtime);
    let encounter = started.encounter.as_ref().unwrap();
    assert_eq!(
        encounter
            .participants
            .iter()
            .map(|participant| (
                participant.character.id,
                participant.faction,
                participant.initiative,
            ))
            .collect::<Vec<_>>(),
        vec![
            (101, EncounterFactionDto::Party, 18),
            (107, EncounterFactionDto::Opposition, 17),
            (104, EncounterFactionDto::Party, 16),
            (102, EncounterFactionDto::Opposition, 14),
            (105, EncounterFactionDto::Party, 13),
            (106, EncounterFactionDto::Party, 12),
        ]
    );
    assert_eq!(encounter.current_actor_id, Some(101));
    assert_eq!(encounter.actions.len(), 4);
    assert_eq!(
        encounter
            .legal_targets
            .iter()
            .find(|entry| entry.action_id == "longsword-strike")
            .unwrap()
            .target_ids,
        vec![102]
    );
    assert!(encounter
        .legal_targets
        .iter()
        .all(|entry| !entry.target_ids.is_empty()));
    let standard_action = id("standard-action").unwrap();
    assert_eq!(
        runtime
            .session()
            .unwrap()
            .activation_budgets(PLAYER)
            .unwrap()
            .current(&standard_action),
        Some(1)
    );

    let applied = runtime
        .choose_action(ChooseActionRequestDto {
            expected_revision: started.revision,
            actor_id: PLAYER.raw(),
            target_id: OPPONENT.raw(),
            action_id: "longsword-strike".to_owned(),
        })
        .unwrap();
    assert!(applied
        .encounter
        .as_ref()
        .unwrap()
        .reaction_prompt
        .is_none());
    assert_eq!(
        runtime
            .session()
            .unwrap()
            .activation_budgets(PLAYER)
            .unwrap()
            .current(&standard_action),
        Some(0)
    );
    assert_eq!(
        applied.encounter.as_ref().unwrap().current_actor_id,
        Some(107)
    );
    assert!(applied.encounter.as_ref().unwrap().actions.is_empty());
}

#[test]
fn tactical_movement_is_engine_routed_atomic_stale_safe_and_persistent() {
    let mut runtime = GameRuntime::empty().unwrap();
    let started = start_test_encounter(&mut runtime);
    let encounter = started.encounter.as_ref().unwrap();
    let movement = id("movement").unwrap();
    let initial_budget = runtime
        .session()
        .unwrap()
        .activation_budgets(PLAYER)
        .unwrap()
        .current(&movement)
        .unwrap();
    let route = encounter
        .board
        .legal_moves
        .iter()
        .find(|route| route.cost > 1)
        .unwrap()
        .clone();
    assert_eq!(
        route.route.first().unwrap(),
        &TacticalCellDto { x: 7, y: 4 }
    );
    assert_eq!(
        route.route.last().unwrap(),
        &TacticalCellDto {
            x: route.x,
            y: route.y
        }
    );

    let before_stale = runtime.encode_save().unwrap();
    assert!(matches!(
        runtime.move_actor(MoveActorRequestDto {
            expected_revision: started.revision - 1,
            actor_id: PLAYER.raw(),
            x: route.x,
            y: route.y,
        }),
        Err(GameRuntimeError::StaleCommand(_))
    ));
    assert_eq!(runtime.encode_save().unwrap(), before_stale);

    assert!(matches!(
        runtime.move_actor(MoveActorRequestDto {
            expected_revision: started.revision,
            actor_id: PLAYER.raw(),
            x: 6,
            y: 4,
        }),
        Err(GameRuntimeError::InvalidCommand(_))
    ));
    assert_eq!(runtime.encode_save().unwrap(), before_stale);

    let moved = runtime
        .move_actor(MoveActorRequestDto {
            expected_revision: started.revision,
            actor_id: PLAYER.raw(),
            x: route.x,
            y: route.y,
        })
        .unwrap();
    let participant = moved
        .encounter
        .as_ref()
        .unwrap()
        .participants
        .iter()
        .find(|participant| participant.character.id == PLAYER.raw())
        .unwrap();
    assert_eq!((participant.x, participant.y), (route.x, route.y));
    assert_eq!(
        runtime
            .session()
            .unwrap()
            .activation_budgets(PLAYER)
            .unwrap()
            .current(&movement),
        Some(initial_budget - route.cost)
    );
    assert!(moved
        .encounter
        .as_ref()
        .unwrap()
        .log
        .last()
        .unwrap()
        .details
        .iter()
        .any(|detail| detail.contains("Engine pathfinding admitted")));

    let encoded = runtime.encode_save().unwrap();
    let reopened = GameRuntime::decode_save(&encoded).unwrap();
    assert_eq!(reopened.encode_save().unwrap(), encoded);
    let reopened_position = reopened
        .snapshot()
        .unwrap()
        .encounter
        .unwrap()
        .participants
        .into_iter()
        .find(|participant| participant.character.id == PLAYER.raw())
        .unwrap();
    assert_eq!(
        (reopened_position.x, reopened_position.y),
        (route.x, route.y)
    );
}

#[test]
fn restore_rejects_a_forged_position_outside_the_authored_component() {
    let catalog = disconnected_floor_catalog();
    let adventure = id("wardens-gate").unwrap();
    let rules = catalog.rules_for(&adventure).unwrap();
    let mut runtime = GameRuntime::empty_with_rules(
        catalog.clone(),
        rules,
        adventure,
        RollSourceConfig::default(),
    )
    .unwrap();
    let camp = runtime.new_adventure(0).unwrap();
    runtime
        .enter_encounter(EnterEncounterRequestDto {
            expected_revision: camp.revision,
            encounter_id: ENCOUNTER_ID.to_owned(),
        })
        .unwrap();
    let encoded = runtime.encode_save().unwrap();
    assert!(
        GameRuntime::decode_save_with_catalog(&encoded, catalog.clone()).is_ok(),
        "an unchanged admitted encounter must reopen"
    );

    let mut forged: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    let participation = forged["session"]["entityState"]["registeredComponents"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|component| component["typeId"] == "rusty-d20.encounter-participation")
        .unwrap();
    let player = participation["values"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|value| value["entity"] == PLAYER.raw())
        .unwrap();
    player["value"]["position"] = json!({ "x": 10, "y": 6 });

    assert!(matches!(
        GameRuntime::decode_save_with_catalog(
            &serde_json::to_string(&forged).unwrap(),
            catalog
        ),
        Err(GameRuntimeError::InvalidSave(message))
            if message.contains("outside the authored placement component")
    ));
}

#[test]
fn held_condition_forbids_voluntary_tactical_movement_without_mutation() {
    let mut runtime = GameRuntime::empty().unwrap();
    let started = start_test_encounter(&mut runtime);
    let destination = started
        .encounter
        .as_ref()
        .unwrap()
        .board
        .legal_moves
        .first()
        .unwrap()
        .clone();
    let pin = id("pin-in-place").unwrap();
    let mut held = false;
    for attempt in 0..24 {
        let preview = runtime
            .session()
            .unwrap()
            .preview_action(
                OPPONENT,
                PLAYER,
                &pin,
                operation(&format!("held-movement-preview-{attempt}")).unwrap(),
            )
            .unwrap();
        let receipt = runtime
            .session_mut()
            .unwrap()
            .apply_action(ApplyActionRequest {
                preview,
                effect_instance: Some(
                    effect_instance(&format!("held-movement-effect-{attempt}")).unwrap(),
                ),
            })
            .unwrap();
        if receipt.hit {
            held = true;
            break;
        }
        runtime
            .session_mut()
            .unwrap()
            .reset_activation_budgets(OPPONENT)
            .unwrap();
    }
    assert!(held, "the deterministic pin sequence must apply Held");
    assert!(runtime
        .snapshot()
        .unwrap()
        .encounter
        .unwrap()
        .participants
        .into_iter()
        .find(|participant| participant.character.id == PLAYER.raw())
        .unwrap()
        .character
        .effects
        .iter()
        .any(|effect| effect.starts_with("Held")));

    assert!(runtime
        .snapshot()
        .unwrap()
        .encounter
        .unwrap()
        .board
        .legal_moves
        .is_empty());
    assert_eq!(
        runtime
            .session()
            .unwrap()
            .active_movement_prohibition(PLAYER)
            .unwrap(),
        Some(id("held").unwrap())
    );
    let before = runtime.encode_save().unwrap();
    assert!(matches!(
        runtime.move_actor(MoveActorRequestDto {
            expected_revision: started.revision,
            actor_id: PLAYER.raw(),
            x: destination.x,
            y: destination.y,
        }),
        Err(GameRuntimeError::InvalidCommand(_))
    ));
    assert_eq!(runtime.encode_save().unwrap(), before);
}

#[test]
fn encounter_projection_rejects_a_current_actor_without_canonical_participation() {
    let mut runtime = GameRuntime::empty().unwrap();
    start_test_encounter(&mut runtime);
    runtime.campaign.as_mut().unwrap().current_actor_id = Some(CAMP_STASH.raw());

    assert!(matches!(
        runtime.snapshot(),
        Err(GameRuntimeError::InvalidState(message))
            if message == "current actor 103 is not an encounter participant"
    ));
}

#[test]
fn schema_ten_fresh_save_round_trips_and_old_product_or_session_schemas_reject() {
    let mut runtime = GameRuntime::empty().unwrap();
    start_test_encounter(&mut runtime);
    let encoded = runtime.encode_save().unwrap();
    assert_eq!(
        GameRuntime::decode_save(&encoded)
            .unwrap()
            .encode_save()
            .unwrap(),
        encoded
    );

    let mut old_product: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    old_product["schemaVersion"] = json!(9);
    assert!(matches!(
        GameRuntime::decode_save(&serde_json::to_string(&old_product).unwrap()),
        Err(GameRuntimeError::UnsupportedSaveSchema { actual: 9 })
    ));

    let mut old_session: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    old_session["session"]["schemaVersion"] = json!(3);
    assert!(matches!(
        GameRuntime::decode_save(&serde_json::to_string(&old_session).unwrap()),
        Err(GameRuntimeError::Save(
            SessionSaveError::UnsupportedSchema { actual: 3 }
        ))
    ));
}

#[test]
fn exploration_event_and_terminal_save_forgery_is_rejected() {
    let mut runtime = GameRuntime::empty().unwrap();
    let camp = runtime.new_adventure(0).unwrap();
    runtime.begin_exploration(camp.revision).unwrap();
    let encoded = runtime.encode_save().unwrap();

    for (field, forged) in [
        ("openedDoors", json!(["zz-forged-door"])),
        ("collectedTreasures", json!(["zz-forged-treasure"])),
        ("checkpointId", json!("zz-forged-checkpoint")),
    ] {
        let mut value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        value["campaign"]["exploration"][field] = forged;
        assert!(matches!(
            GameRuntime::decode_save(&serde_json::to_string(&value).unwrap()),
            Err(GameRuntimeError::InvalidSave(_))
        ));
    }

    let mut missing_prerequisite: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    missing_prerequisite["campaign"]["exploration"]["openedDoors"] = json!(["inner-sigil-gate"]);
    assert!(matches!(
        GameRuntime::decode_save(&serde_json::to_string(&missing_prerequisite).unwrap()),
        Err(GameRuntimeError::InvalidSave(_))
    ));

    let mut forged_collection: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    forged_collection["campaign"]["exploration"]["position"] = json!({"x": 9, "y": 2});
    forged_collection["campaign"]["exploration"]["discovered"] = json!([
        {"x": 1, "y": 1},
        {"x": 2, "y": 1},
        {"x": 3, "y": 1},
        {"x": 4, "y": 1},
        {"x": 5, "y": 1},
        {"x": 6, "y": 1},
        {"x": 7, "y": 1},
        {"x": 8, "y": 1},
        {"x": 9, "y": 1},
        {"x": 9, "y": 2}
    ]);
    forged_collection["campaign"]["exploration"]["collectedTreasures"] = json!(["sigil-cache"]);
    assert!(matches!(
        GameRuntime::decode_save(&serde_json::to_string(&forged_collection).unwrap()),
        Err(GameRuntimeError::InvalidSave(message))
            if message == "dungeon treasure ownership contradicts the authoritative event state"
    ));

    let mut forged_terminal: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    forged_terminal["campaign"]["phase"] = json!("adventure-complete");
    assert!(matches!(
        GameRuntime::decode_save(&serde_json::to_string(&forged_terminal).unwrap()),
        Err(GameRuntimeError::InvalidSave(_))
    ));
}

#[test]
fn restored_activation_budgets_require_the_exact_authored_identity_set_and_bounds() {
    let mut runtime = GameRuntime::empty().unwrap();
    start_test_encounter(&mut runtime);
    let encoded = runtime.encode_save().unwrap();
    assert_eq!(
        GameRuntime::decode_save(&encoded)
            .unwrap()
            .encode_save()
            .unwrap(),
        encoded,
        "a canonical activation budget set must reopen exactly"
    );

    let mut unknown_extra: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    saved_activation_budgets(&mut unknown_extra, PLAYER.raw()).push(json!({
        "id": "zz-forged-budget",
        "current": 1
    }));
    assert!(matches!(
        GameRuntime::decode_save(&serde_json::to_string(&unknown_extra).unwrap()),
        Err(GameRuntimeError::Save(SessionSaveError::InvalidState(
            D20SessionError::ActivationBudgetUnavailable { budget, .. }
        ))) if budget.as_str() == "zz-forged-budget"
    ));

    let mut missing_known: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    saved_activation_budgets(&mut missing_known, PLAYER.raw())
        .retain(|budget| budget["id"] != "bonus-action");
    assert!(matches!(
        GameRuntime::decode_save(&serde_json::to_string(&missing_known).unwrap()),
        Err(GameRuntimeError::Save(SessionSaveError::InvalidState(
            D20SessionError::ActivationBudgetUnavailable { budget, .. }
        ))) if budget.as_str() == "bonus-action"
    ));

    let mut above_initial: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    saved_activation_budgets(&mut above_initial, PLAYER.raw())
        .iter_mut()
        .find(|budget| budget["id"] == "standard-action")
        .unwrap()["current"] = json!(2);
    assert!(matches!(
        GameRuntime::decode_save(&serde_json::to_string(&above_initial).unwrap()),
        Err(GameRuntimeError::Save(SessionSaveError::InvalidState(
            D20SessionError::ActivationBudgetUnavailable {
                budget,
                required: 2,
                available: 1,
                ..
            }
        ))) if budget.as_str() == "standard-action"
    ));
}

#[test]
fn subset_party_defeat_reopens_but_cannot_borrow_nonparticipant_liveness() {
    let mut runtime = subset_party_runtime();
    let camp = runtime.new_adventure(0).unwrap();
    runtime
        .enter_encounter(EnterEncounterRequestDto {
            expected_revision: camp.revision,
            encounter_id: ENCOUNTER_ID.to_owned(),
        })
        .unwrap();
    let outcome = play_to_outcome(&mut runtime, "pass", false, false);
    assert_eq!(
        outcome.campaign.as_ref().unwrap().phase,
        CampaignPhaseDto::Outcome
    );
    assert_eq!(
        outcome
            .campaign
            .as_ref()
            .unwrap()
            .latest_outcome
            .as_ref()
            .unwrap()
            .kind,
        EncounterOutcomeKindDto::Defeat
    );
    assert!(outcome
        .campaign
        .as_ref()
        .unwrap()
        .party
        .iter()
        .any(|member| member.character.id != PLAYER.raw() && member.character.health_current > 0));

    let encoded = runtime.encode_save().unwrap();
    assert_eq!(
        GameRuntime::decode_save_with_catalog(&encoded, subset_party_catalog())
            .unwrap()
            .encode_save()
            .unwrap(),
        encoded,
        "defeat is determined by the admitted encounter party, not idle adventure members"
    );

    let living_opposition = outcome
        .encounter
        .as_ref()
        .unwrap()
        .participants
        .iter()
        .find(|participant| {
            participant.faction == EncounterFactionDto::Opposition
                && participant.character.health_current > 0
        })
        .unwrap()
        .character
        .id;
    let mut forged_active: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    forged_active["campaign"]["phase"] = json!("encounter");
    forged_active["campaign"]["resolvedEncounterId"] = serde_json::Value::Null;
    forged_active["campaign"]["currentActorId"] = json!(living_opposition);
    forged_active["campaign"]["outcome"] = serde_json::Value::Null;
    forged_active["campaign"]["completedEncounters"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(matches!(
        GameRuntime::decode_save_with_catalog(
            &serde_json::to_string(&forged_active).unwrap(),
            subset_party_catalog()
        ),
        Err(GameRuntimeError::InvalidSave(message))
            if message.contains("party alive=0")
    ));
}

#[test]
fn dungeon_exploration_is_authoritative_atomic_persistent_and_triggers_encounters() {
    let mut runtime = GameRuntime::empty().unwrap();
    let camp = runtime.new_adventure(0).unwrap();

    let before_stale = runtime.snapshot().unwrap();
    assert!(matches!(
        runtime.begin_exploration(0),
        Err(GameRuntimeError::StaleCommand(_))
    ));
    assert_eq!(runtime.snapshot().unwrap(), before_stale);

    let mut exploring = runtime.begin_exploration(camp.revision).unwrap();
    assert_eq!(
        exploring.campaign.as_ref().unwrap().phase,
        CampaignPhaseDto::Exploration
    );
    let initial = exploring.exploration.as_ref().unwrap();
    assert_eq!((initial.x, initial.y), (1, 1));
    assert_eq!(initial.facing, ExplorationFacingDto::East);
    assert_eq!(
        initial
            .discovered_cells
            .iter()
            .map(|cell| (cell.x, cell.y))
            .collect::<Vec<_>>(),
        vec![(1, 1)]
    );

    exploring = runtime
        .exploration_command(ExplorationCommandRequestDto {
            expected_revision: exploring.revision,
            command: ExplorationCommandKindDto::TurnLeft,
        })
        .unwrap();
    assert_eq!(
        exploring.exploration.as_ref().unwrap().facing,
        ExplorationFacingDto::North
    );
    let before_wall = runtime.snapshot().unwrap();
    assert!(matches!(
        runtime.exploration_command(ExplorationCommandRequestDto {
            expected_revision: before_wall.revision,
            command: ExplorationCommandKindDto::StepForward,
        }),
        Err(GameRuntimeError::InvalidCommand(message))
            if message.contains("solid dungeon stone")
    ));
    assert_eq!(runtime.snapshot().unwrap(), before_wall);

    exploring = runtime
        .exploration_command(ExplorationCommandRequestDto {
            expected_revision: exploring.revision,
            command: ExplorationCommandKindDto::TurnRight,
        })
        .unwrap();
    for expected_x in 2..=5 {
        exploring = runtime
            .exploration_command(ExplorationCommandRequestDto {
                expected_revision: exploring.revision,
                command: ExplorationCommandKindDto::StepForward,
            })
            .unwrap();
        assert_eq!(
            exploring.exploration.as_ref().map(|state| state.x),
            Some(expected_x)
        );
    }
    assert_eq!(
        exploring
            .exploration
            .as_ref()
            .and_then(|state| state.landmark.as_ref())
            .map(|landmark| landmark.id.as_str()),
        Some("gate-murder-holes")
    );
    exploring = runtime
        .exploration_command(ExplorationCommandRequestDto {
            expected_revision: exploring.revision,
            command: ExplorationCommandKindDto::Interact,
        })
        .unwrap();
    assert!(exploring
        .exploration
        .as_ref()
        .and_then(|state| state.landmark.as_ref())
        .is_some_and(|landmark| landmark.inspected));

    let exploration_save = runtime.encode_save().unwrap();
    let mut reopened = GameRuntime::decode_save(&exploration_save).unwrap();
    assert_eq!(reopened.encode_save().unwrap(), exploration_save);
    exploring = reopened.snapshot().unwrap();

    for expected_x in 6..=8 {
        exploring = reopened
            .exploration_command(ExplorationCommandRequestDto {
                expected_revision: exploring.revision,
                command: ExplorationCommandKindDto::StepForward,
            })
            .unwrap();
        assert_eq!(
            exploring.exploration.as_ref().map(|state| state.x),
            Some(expected_x)
        );
        assert!(exploring.encounter.is_none());
    }
    let encounter = reopened
        .exploration_command(ExplorationCommandRequestDto {
            expected_revision: exploring.revision,
            command: ExplorationCommandKindDto::StepForward,
        })
        .unwrap();
    assert_eq!(
        encounter.campaign.as_ref().unwrap().phase,
        CampaignPhaseDto::Encounter
    );
    assert_eq!(
        encounter
            .campaign
            .as_ref()
            .unwrap()
            .active_encounter_id
            .as_deref(),
        Some(ENCOUNTER_ID)
    );
    assert!(encounter.exploration.is_some());
    assert!(encounter.encounter.is_some());
}

#[test]
fn alternate_ember_adventure_selection_is_atomic_distinct_and_persistent() {
    let mut runtime = GameRuntime::empty().unwrap();
    let empty = runtime.snapshot().unwrap();
    assert_eq!(
        empty
            .available_adventures
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        vec!["embers-wake", "wardens-gate"]
    );

    assert!(matches!(
        runtime.new_adventure_for(NewAdventureRequestDto {
            expected_revision: 1,
            adventure_id: "embers-wake".to_owned(),
        }),
        Err(GameRuntimeError::StaleCommand(_))
    ));
    assert_eq!(runtime.snapshot().unwrap(), empty);
    assert!(matches!(
        runtime.new_adventure_for(NewAdventureRequestDto {
            expected_revision: 0,
            adventure_id: "unknown-path".to_owned(),
        }),
        Err(GameRuntimeError::InvalidCommand(_))
    ));
    assert_eq!(runtime.snapshot().unwrap(), empty);
    assert!(matches!(
        runtime.new_adventure_for(NewAdventureRequestDto {
            expected_revision: 0,
            adventure_id: "catalog-probe".to_owned(),
        }),
        Err(GameRuntimeError::InvalidCommand(_))
    ));
    assert_eq!(runtime.snapshot().unwrap(), empty);

    let camp = runtime
        .new_adventure_for(NewAdventureRequestDto {
            expected_revision: 0,
            adventure_id: "embers-wake".to_owned(),
        })
        .unwrap();
    assert_ne!(camp.ruleset_fingerprint, empty.ruleset_fingerprint);
    let campaign = camp.campaign.as_ref().unwrap();
    assert_eq!(campaign.id, "embers-wake");
    assert_eq!(campaign.title, "Ember's Wake");
    assert_eq!(campaign.party[0].character.id, 111);
    assert_eq!(campaign.party[0].character.name, "Sera Vale");
    let nerve = campaign
        .party
        .first()
        .unwrap()
        .loadout
        .defenses
        .iter()
        .find(|defense| defense.id == "nerve")
        .unwrap();
    assert!(nerve.sources.iter().any(|source| source.contains("212")));
    assert!(nerve.sources.iter().any(|source| source.contains("213")));

    let before_reselection = runtime.snapshot().unwrap();
    assert!(matches!(
        runtime.new_adventure_for(NewAdventureRequestDto {
            expected_revision: camp.revision,
            adventure_id: "wardens-gate".to_owned(),
        }),
        Err(GameRuntimeError::InvalidCommand(_))
    ));
    assert_eq!(runtime.snapshot().unwrap(), before_reselection);

    let encounter = runtime
        .enter_encounter(EnterEncounterRequestDto {
            expected_revision: camp.revision,
            encounter_id: "ash-seer".to_owned(),
        })
        .unwrap();
    assert_eq!(
        encounter
            .encounter
            .as_ref()
            .unwrap()
            .actions
            .iter()
            .map(|action| action.id.as_str())
            .collect::<Vec<_>>(),
        vec!["fire-bolt", "mind-spike"]
    );
    assert_eq!(
        encounter
            .encounter
            .as_ref()
            .unwrap()
            .participants
            .iter()
            .find(|participant| participant.character.id == 112)
            .unwrap()
            .character
            .name,
        "Ash Seer"
    );

    let encoded = runtime.encode_save().unwrap();
    let reopened = GameRuntime::decode_save(&encoded).unwrap();
    assert_eq!(reopened.encode_save().unwrap(), encoded);
    assert_eq!(reopened.snapshot().unwrap(), {
        let mut saved = encounter;
        saved.saved = true;
        saved
    });

    let mut mismatched: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    mismatched["campaign"]["adventureId"] = json!("wardens-gate");
    assert!(matches!(
        GameRuntime::decode_save(&serde_json::to_string(&mismatched).unwrap()),
        Err(GameRuntimeError::CompositionFingerprintMismatch { .. })
    ));
}

#[test]
fn content_only_adventure_uses_shared_orchestration_and_exact_composition() {
    let default_fingerprint = GameRuntime::empty()
        .unwrap()
        .snapshot()
        .unwrap()
        .ruleset_fingerprint;
    let mut runtime = GameRuntime::empty_for("catalog-probe").unwrap();
    let empty = runtime.snapshot().unwrap();
    assert_ne!(empty.ruleset_fingerprint, default_fingerprint);

    let camp = runtime.new_adventure(0).unwrap();
    let campaign = camp.campaign.as_ref().unwrap();
    assert_eq!(campaign.id, "catalog-probe");
    assert_eq!(campaign.title, "Authored Catalog Probe");
    assert_eq!(
        runtime.log.last().unwrap().text,
        "The content-only catalog probe is ready."
    );
    let encounter = runtime
        .enter_encounter(EnterEncounterRequestDto {
            expected_revision: camp.revision,
            encounter_id: ENCOUNTER_ID.to_owned(),
        })
        .unwrap();
    assert_eq!(
        encounter.campaign.as_ref().unwrap().active_encounter_id,
        Some(ENCOUNTER_ID.to_owned())
    );

    let encoded = runtime.encode_save().unwrap();
    let reopened = GameRuntime::decode_save(&encoded).unwrap();
    assert_eq!(reopened.encode_save().unwrap(), encoded);
    assert_eq!(
        reopened.snapshot().unwrap().campaign.unwrap().id,
        "catalog-probe"
    );

    let mut mismatched: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    mismatched["compositionFingerprint"] = json!("0".repeat(64));
    assert!(matches!(
        GameRuntime::decode_save(&serde_json::to_string(&mismatched).unwrap()),
        Err(GameRuntimeError::CompositionFingerprintMismatch { .. })
    ));
    assert_eq!(
        GameRuntime::decode_save(&encoded)
            .unwrap()
            .encode_save()
            .unwrap(),
        encoded
    );
}

#[test]
fn camp_loadout_is_engine_backed_typed_atomic_and_persistent() {
    let mut runtime = GameRuntime::empty().unwrap();
    let camp = runtime.new_adventure(0).unwrap();
    let loadout = &camp.campaign.as_ref().unwrap().party[0].loadout;
    assert_eq!(loadout.capacity.used, 4);
    assert_eq!(loadout.capacity.maximum, 4);
    assert_eq!(defense_value(loadout, "armor"), 18);
    assert_eq!(
        loadout
            .equipment_slots
            .iter()
            .find(|slot| slot.id == "body")
            .unwrap()
            .equipped
            .as_ref()
            .unwrap()
            .entity_id,
        PLAYER_CHAIN_ARMOR.raw()
    );

    let before_invalid = runtime.snapshot().unwrap();
    let invalid_slot = runtime
        .equip_item(EquipItemRequestDto {
            expected_revision: camp.revision,
            item_id: PLAYER_CHAIN_ARMOR.raw(),
            slot_id: "off-hand".to_owned(),
        })
        .unwrap_err();
    assert_eq!(invalid_slot.api_error().kind, ApiErrorKindDto::InvalidSlot);
    assert_eq!(runtime.snapshot().unwrap(), before_invalid);

    let capacity = runtime
        .transfer_item(TransferItemRequestDto {
            expected_revision: camp.revision,
            item_id: STASH_BUCKLER.raw(),
            from_owner_id: CAMP_STASH.raw(),
            to_owner_id: PLAYER.raw(),
        })
        .unwrap_err();
    assert_eq!(capacity.api_error().kind, ApiErrorKindDto::Capacity);
    assert_eq!(runtime.snapshot().unwrap(), before_invalid);

    let containment = runtime
        .transfer_item(TransferItemRequestDto {
            expected_revision: camp.revision,
            item_id: PLAYER_CHAIN_ARMOR.raw(),
            from_owner_id: PLAYER.raw(),
            to_owner_id: CAMP_STASH.raw(),
        })
        .unwrap_err();
    assert_eq!(containment.api_error().kind, ApiErrorKindDto::Containment);
    assert_eq!(runtime.snapshot().unwrap(), before_invalid);

    let chain_removed = runtime
        .unequip_item(UnequipItemRequestDto {
            expected_revision: camp.revision,
            item_id: PLAYER_CHAIN_ARMOR.raw(),
        })
        .unwrap();
    assert_eq!(
        defense_value(
            &chain_removed.campaign.as_ref().unwrap().party[0].loadout,
            "armor",
        ),
        16
    );
    let chain_restored = runtime
        .equip_item(EquipItemRequestDto {
            expected_revision: chain_removed.revision,
            item_id: PLAYER_CHAIN_ARMOR.raw(),
            slot_id: "body".to_owned(),
        })
        .unwrap();
    assert_eq!(
        defense_value(
            &chain_restored.campaign.as_ref().unwrap().party[0].loadout,
            "armor",
        ),
        18
    );

    let buckler_removed = runtime
        .unequip_item(UnequipItemRequestDto {
            expected_revision: chain_restored.revision,
            item_id: PLAYER_BUCKLER.raw(),
        })
        .unwrap();
    let stored = runtime
        .transfer_item(TransferItemRequestDto {
            expected_revision: buckler_removed.revision,
            item_id: PLAYER_BUCKLER.raw(),
            from_owner_id: PLAYER.raw(),
            to_owner_id: CAMP_STASH.raw(),
        })
        .unwrap();
    let taken = runtime
        .transfer_item(TransferItemRequestDto {
            expected_revision: stored.revision,
            item_id: STASH_BUCKLER.raw(),
            from_owner_id: CAMP_STASH.raw(),
            to_owner_id: PLAYER.raw(),
        })
        .unwrap();
    let equipped = runtime
        .equip_item(EquipItemRequestDto {
            expected_revision: taken.revision,
            item_id: STASH_BUCKLER.raw(),
            slot_id: "off-hand".to_owned(),
        })
        .unwrap();
    let equipped_loadout = &equipped.campaign.as_ref().unwrap().party[0].loadout;
    assert_eq!(equipped_loadout.capacity.used, 4);
    assert_eq!(
        equipped_loadout
            .equipment_slots
            .iter()
            .find(|slot| slot.id == "off-hand")
            .unwrap()
            .equipped
            .as_ref()
            .unwrap()
            .entity_id,
        STASH_BUCKLER.raw()
    );

    let stale_before = runtime.snapshot().unwrap();
    let stale = runtime
        .unequip_item(UnequipItemRequestDto {
            expected_revision: taken.revision,
            item_id: STASH_BUCKLER.raw(),
        })
        .unwrap_err();
    assert_eq!(stale.api_error().kind, ApiErrorKindDto::Stale);
    assert_eq!(runtime.snapshot().unwrap(), stale_before);

    let encoded = runtime.encode_save().unwrap();
    let mut reopened = GameRuntime::decode_save(&encoded).unwrap();
    assert_eq!(reopened.encode_save().unwrap(), encoded);
    let reopened_snapshot = reopened.snapshot().unwrap();
    assert_eq!(
        reopened_snapshot.campaign.as_ref().unwrap().party[0].loadout,
        equipped_loadout.clone()
    );
    let encounter = reopened
        .enter_encounter(EnterEncounterRequestDto {
            expected_revision: reopened_snapshot.revision,
            encounter_id: ENCOUNTER_ID.to_owned(),
        })
        .unwrap();
    assert_eq!(
        encounter.campaign.as_ref().unwrap().party[0]
            .loadout
            .equipment_slots
            .iter()
            .find(|slot| slot.id == "off-hand")
            .unwrap()
            .equipped
            .as_ref()
            .unwrap()
            .entity_id,
        STASH_BUCKLER.raw()
    );
    let phase_before = reopened.snapshot().unwrap();
    let phase_error = reopened
        .unequip_item(UnequipItemRequestDto {
            expected_revision: encounter.revision,
            item_id: STASH_BUCKLER.raw(),
        })
        .unwrap_err();
    assert_eq!(phase_error.api_error().kind, ApiErrorKindDto::Phase);
    assert_eq!(reopened.snapshot().unwrap(), phase_before);
}

#[test]
fn equipment_track_bound_failure_keeps_its_public_error_identity() {
    let error = GameRuntimeError::Session(D20SessionError::Mechanics(
        MechanicsError::EquipmentWouldInvalidateTrack {
            owner: PLAYER,
            track: gameplay_mechanics::TrackId::parse("vitality").unwrap(),
            current: 100,
            prospective_minimum: 0,
            prospective_maximum: 90,
        },
    ));
    assert_eq!(error.api_error().kind, ApiErrorKindDto::TrackBound);
}

#[test]
fn translated_action_rejections_keep_public_invalid_and_stale_identities() {
    let unavailable = GameRuntimeError::Session(D20SessionError::RequiredImplementNotEquipped {
        entity: PLAYER,
        implement: D20Id::parse("training-blade").unwrap(),
    })
    .api_error();
    assert_eq!(unavailable.kind, ApiErrorKindDto::Invalid);
    assert!(!unavailable.retryable);

    let stale = GameRuntimeError::Session(D20SessionError::StalePreview {
        reason: "equipment changed",
    })
    .api_error();
    assert_eq!(stale.kind, ApiErrorKindDto::Stale);
    assert!(stale.retryable);
}

#[test]
fn opposition_filters_condition_forbidden_actions_without_retry_deadlock() {
    let mut runtime = GameRuntime::empty().unwrap();
    start_test_encounter(&mut runtime);

    for _ in 0..96 {
        let current = runtime.snapshot().unwrap();
        let encounter = current.encounter.as_ref().unwrap();
        let current_actor = encounter.current_actor_id.unwrap();
        let participant = encounter
            .participants
            .iter()
            .find(|participant| participant.character.id == current_actor)
            .unwrap();
        let opponent_unsettled = encounter
            .participants
            .iter()
            .find(|participant| participant.character.id == OPPONENT.raw())
            .unwrap()
            .character
            .effects
            .iter()
            .any(|effect| effect.starts_with("Unsettled"));

        if participant.faction == EncounterFactionDto::Party {
            if current_actor == PLAYER.raw() {
                runtime
                    .choose_action(ChooseActionRequestDto {
                        expected_revision: current.revision,
                        actor_id: PLAYER.raw(),
                        target_id: OPPONENT.raw(),
                        action_id: "disrupt".to_owned(),
                    })
                    .unwrap();
            } else {
                runtime.end_activation(current.revision).unwrap();
            }
            continue;
        }

        let opposition = runtime
            .begin_opposition_turn(current.revision)
            .expect("a forbidden deterministic choice must not deadlock opposition");
        let Some(pending) = opposition
            .encounter
            .as_ref()
            .unwrap()
            .reaction_prompt
            .as_ref()
        else {
            continue;
        };
        if current_actor == OPPONENT.raw() && opponent_unsettled {
            assert!(
                matches!(
                    pending.action_id.as_str(),
                    "longsword-strike" | "precise-shot"
                ),
                "Unsettled forbids the opponent's control-tagged Pin In Place and Disrupt actions"
            );
            return;
        }
        runtime
            .decline_reaction(DeclineReactionRequestDto {
                expected_revision: opposition.revision,
                prompt_token: pending.token.clone(),
            })
            .unwrap();
    }

    panic!("the deterministic Disrupt sequence never applied Unsettled");
}

#[test]
fn disrupt_forces_the_target_without_spending_its_movement_budget() {
    let mut runtime = GameRuntime::empty().unwrap();
    start_test_encounter(&mut runtime);
    let movement = id("movement").unwrap();

    for _ in 0..96 {
        let current = runtime.snapshot().unwrap();
        let encounter = current.encounter.as_ref().unwrap();
        let current_actor = encounter.current_actor_id.unwrap();
        let participant = encounter
            .participants
            .iter()
            .find(|participant| participant.character.id == current_actor)
            .unwrap();

        if participant.faction == EncounterFactionDto::Party {
            if current_actor != PLAYER.raw() {
                runtime.end_activation(current.revision).unwrap();
                continue;
            }
            let before_position = encounter
                .participants
                .iter()
                .find(|participant| participant.character.id == OPPONENT.raw())
                .map(|participant| (participant.x, participant.y))
                .unwrap();
            let before_budget = runtime
                .session()
                .unwrap()
                .activation_budgets(OPPONENT)
                .unwrap()
                .current(&movement);
            let resolved = runtime
                .choose_action(ChooseActionRequestDto {
                    expected_revision: current.revision,
                    actor_id: PLAYER.raw(),
                    target_id: OPPONENT.raw(),
                    action_id: "disrupt".to_owned(),
                })
                .unwrap();
            let after_position = resolved
                .encounter
                .as_ref()
                .unwrap()
                .participants
                .iter()
                .find(|participant| participant.character.id == OPPONENT.raw())
                .map(|participant| (participant.x, participant.y))
                .unwrap();
            if after_position != before_position {
                assert_eq!(before_position, (8, 4));
                assert_eq!(after_position, (10, 4));
                assert_eq!(
                    runtime
                        .session()
                        .unwrap()
                        .activation_budgets(OPPONENT)
                        .unwrap()
                        .current(&movement),
                    before_budget
                );
                assert!(resolved
                    .encounter
                    .as_ref()
                    .unwrap()
                    .log
                    .iter()
                    .flat_map(|entry| &entry.details)
                    .any(|detail| detail.contains("without spending movement")));
                return;
            }
            continue;
        }

        let opposition = runtime.begin_opposition_turn(current.revision).unwrap();
        if let Some(pending) = opposition
            .encounter
            .as_ref()
            .unwrap()
            .reaction_prompt
            .as_ref()
        {
            runtime
                .decline_reaction(DeclineReactionRequestDto {
                    expected_revision: opposition.revision,
                    prompt_token: pending.token.clone(),
                })
                .unwrap();
        }
    }

    panic!("the deterministic Disrupt sequence never exercised forced movement");
}

#[test]
fn opposition_with_no_legal_action_explicitly_advances_the_activation() {
    let mut runtime = GameRuntime::empty().unwrap();
    start_test_encounter(&mut runtime);

    for _ in 0..96 {
        let current = runtime.snapshot().unwrap();
        let encounter = current.encounter.as_ref().unwrap();
        let current_actor = encounter.current_actor_id.unwrap();
        let participant = encounter
            .participants
            .iter()
            .find(|participant| participant.character.id == current_actor)
            .unwrap();
        let opponent_unsettled = encounter
            .participants
            .iter()
            .find(|participant| participant.character.id == OPPONENT.raw())
            .unwrap()
            .character
            .effects
            .iter()
            .any(|effect| effect.starts_with("Unsettled"));

        if participant.faction == EncounterFactionDto::Party {
            if current_actor == PLAYER.raw() {
                runtime
                    .choose_action(ChooseActionRequestDto {
                        expected_revision: current.revision,
                        actor_id: PLAYER.raw(),
                        target_id: OPPONENT.raw(),
                        action_id: "disrupt".to_owned(),
                    })
                    .unwrap();
            } else {
                runtime.end_activation(current.revision).unwrap();
            }
            continue;
        }

        if current_actor == OPPONENT.raw() && opponent_unsettled {
            runtime
                .session_mut()
                .unwrap()
                .unequip_item(
                    OPPONENT,
                    OPPONENT_BLADE,
                    operation("test-unequip-opponent-blade").unwrap(),
                )
                .unwrap();
            runtime
                .session_mut()
                .unwrap()
                .unequip_item(
                    OPPONENT,
                    OPPONENT_BOW,
                    operation("test-unequip-opponent-bow").unwrap(),
                )
                .unwrap();

            let progressed = runtime.begin_opposition_turn(current.revision).unwrap();
            let encounter = progressed.encounter.as_ref().unwrap();
            assert_ne!(encounter.current_actor_id, Some(OPPONENT.raw()));
            assert!(encounter.reaction_prompt.is_none());
            assert!(
                encounter.log.last().unwrap().details.iter().any(|detail| {
                    detail.contains("no legal authored action")
                        && detail.contains("16 unavailable choice(s)")
                }),
                "{:?}",
                encounter.log.last().unwrap()
            );
            return;
        }

        let opposition = runtime.begin_opposition_turn(current.revision).unwrap();
        let Some(pending) = opposition
            .encounter
            .as_ref()
            .unwrap()
            .reaction_prompt
            .as_ref()
        else {
            continue;
        };
        runtime
            .decline_reaction(DeclineReactionRequestDto {
                expected_revision: opposition.revision,
                prompt_token: pending.token.clone(),
            })
            .unwrap();
    }

    panic!("the deterministic Disrupt sequence never exercised no-legal-action progression");
}

#[test]
fn product_runtime_is_atomic_stale_safe_and_reopens_deterministically() {
    let mut runtime = GameRuntime::empty().unwrap();
    assert!(runtime.snapshot().unwrap().encounter.is_none());
    let started = start_test_encounter(&mut runtime);
    let encounter = started.encounter.unwrap();
    assert_eq!(encounter.participants.len(), 6);
    assert_eq!(encounter.actions.len(), 4);

    let before_stale = runtime.encode_save().unwrap();
    assert!(matches!(
        runtime.choose_action(ChooseActionRequestDto {
            expected_revision: 0,
            actor_id: PLAYER.raw(),
            target_id: OPPONENT.raw(),
            action_id: "longsword-strike".to_owned(),
        }),
        Err(GameRuntimeError::StaleCommand(_))
    ));
    assert_eq!(runtime.encode_save().unwrap(), before_stale);

    let applied = runtime
        .choose_action(ChooseActionRequestDto {
            expected_revision: started.revision,
            actor_id: PLAYER.raw(),
            target_id: OPPONENT.raw(),
            action_id: "longsword-strike".to_owned(),
        })
        .unwrap();
    assert!(applied
        .encounter
        .as_ref()
        .unwrap()
        .reaction_prompt
        .is_none());
    assert!(applied
        .encounter
        .as_ref()
        .unwrap()
        .log
        .iter()
        .any(|entry| entry.details.iter().any(|detail| detail.contains("d20"))));

    let encoded = runtime.encode_save().unwrap();
    let mut reopened = GameRuntime::decode_save(&encoded).unwrap();
    let mut same_reopened = GameRuntime::decode_save(&encoded).unwrap();
    assert_eq!(reopened.encode_save().unwrap(), encoded);
    let reopened_snapshot = reopened.snapshot().unwrap();
    assert!(reopened_snapshot
        .encounter
        .as_ref()
        .unwrap()
        .reaction_prompt
        .is_none());
    assert_eq!(
        reopened_snapshot
            .encounter
            .as_ref()
            .unwrap()
            .current_actor_id,
        Some(107)
    );
    let opposition = reopened
        .begin_opposition_turn(reopened_snapshot.revision)
        .unwrap();
    let same_opposition = same_reopened
        .begin_opposition_turn(reopened_snapshot.revision)
        .unwrap();
    assert_eq!(
        opposition.encounter.as_ref().unwrap().reaction_prompt,
        same_opposition.encounter.as_ref().unwrap().reaction_prompt,
        "the exact save and Rust-owned RNG position select the same opposition action"
    );
    let advanced = if let Some(prompt) = opposition
        .encounter
        .as_ref()
        .unwrap()
        .reaction_prompt
        .as_ref()
    {
        assert_eq!(prompt.actor_id, 107);
        assert!(matches!(prompt.target_id, 101 | 104 | 105 | 106));
        reopened
            .decline_reaction(DeclineReactionRequestDto {
                expected_revision: opposition.revision,
                prompt_token: prompt.token.clone(),
            })
            .unwrap()
    } else {
        opposition
    };
    let advanced_encounter = advanced.encounter.as_ref().unwrap();
    assert_eq!(advanced_encounter.round, 0);
    assert_eq!(advanced_encounter.current_actor_id, Some(104));
    assert!(advanced_encounter
        .log
        .last()
        .is_some_and(|entry| entry.source == "Initiative" && entry.text.contains("Ilyra Fen")));
}

#[test]
fn product_static_roll_source_resolves_without_a_roll_prompt_and_reopens_exactly() {
    let roll_source = RollSourceConfig::static_rolls(vec![StaticActionRoll {
        d20: 20,
        damage: vec![8],
    }])
    .unwrap();
    let mut runtime = GameRuntime::empty_with_roll_source(roll_source.clone()).unwrap();
    let started = start_test_encounter(&mut runtime);
    let resolved = runtime
        .choose_action(ChooseActionRequestDto {
            expected_revision: started.revision,
            actor_id: PLAYER.raw(),
            target_id: OPPONENT.raw(),
            action_id: "longsword-strike".to_owned(),
        })
        .unwrap();
    let encounter = resolved.encounter.as_ref().unwrap();
    assert!(encounter.reaction_prompt.is_none());
    assert!(encounter.log.iter().any(|entry| {
        entry
            .details
            .iter()
            .any(|detail| detail.starts_with("d20 20 +"))
    }));
    assert!(encounter.log.iter().any(|entry| {
        entry
            .details
            .iter()
            .any(|detail| detail == "Roll-source position 0.")
    }));

    let encoded = runtime.encode_save().unwrap();
    let reopened = GameRuntime::decode_save(&encoded).unwrap();
    assert_eq!(reopened.roll_source(), &roll_source);
    assert_eq!(reopened.encode_save().unwrap(), encoded);
}

#[test]
fn complete_encounter_victory_grants_reward_once_and_reopens_exactly() {
    let mut runtime = GameRuntime::empty().unwrap();
    start_test_encounter(&mut runtime);
    let outcome = play_to_outcome(&mut runtime, "precise-shot", false, true);
    let campaign = outcome.campaign.as_ref().unwrap();
    assert_eq!(campaign.phase, CampaignPhaseDto::Outcome);
    assert_eq!(
        campaign.latest_outcome.as_ref().unwrap().kind,
        EncounterOutcomeKindDto::Victory
    );
    assert_eq!(
        campaign.latest_outcome.as_ref().unwrap().reward_item_id,
        Some(OPPONENT_ARMOR.raw())
    );
    assert!(campaign.party[0]
        .loadout
        .stash_items
        .iter()
        .any(|item| item.entity_id == OPPONENT_ARMOR.raw()));
    assert_eq!(outcome.encounter.as_ref().unwrap().current_actor_id, None);
    assert!(outcome
        .encounter
        .as_ref()
        .unwrap()
        .log
        .iter()
        .any(|entry| entry.text.contains("yields the Warden chain armor")));

    let encoded_outcome = runtime.encode_save().unwrap();
    let mut reopened = GameRuntime::decode_save(&encoded_outcome).unwrap();
    assert_eq!(reopened.encode_save().unwrap(), encoded_outcome);
    let before_late = reopened.snapshot().unwrap();
    let late = reopened
        .begin_opposition_turn(before_late.revision)
        .unwrap_err();
    assert_eq!(late.api_error().kind, ApiErrorKindDto::Phase);
    assert_eq!(reopened.snapshot().unwrap(), before_late);

    let camp = reopened.return_to_camp(before_late.revision).unwrap();
    let campaign = camp.campaign.as_ref().unwrap();
    assert_eq!(campaign.phase, CampaignPhaseDto::Camp);
    assert_eq!(
        campaign
            .available_encounters
            .iter()
            .map(|encounter| encounter.id.as_str())
            .collect::<Vec<_>>(),
        vec!["seal-guard"]
    );
    assert_eq!(campaign.completed_encounters.len(), 1);
    assert_eq!(
        campaign.party[0]
            .loadout
            .stash_items
            .iter()
            .filter(|item| item.entity_id == OPPONENT_ARMOR.raw())
            .count(),
        1
    );
    let before_duplicate = reopened.snapshot().unwrap();
    assert!(matches!(
        reopened.return_to_camp(before_duplicate.revision),
        Err(GameRuntimeError::WrongPhase(_))
    ));
    assert_eq!(reopened.snapshot().unwrap(), before_duplicate);
    let camp_save = reopened.encode_save().unwrap();
    assert_eq!(
        GameRuntime::decode_save(&camp_save)
            .unwrap()
            .encode_save()
            .unwrap(),
        camp_save
    );
}

#[test]
fn ordered_campaign_advances_through_three_encounters_to_authored_terminal_completion() {
    let mut runtime = GameRuntime::empty().unwrap();
    start_test_encounter(&mut runtime);
    let first = play_to_outcome(&mut runtime, "precise-shot", false, true);
    assert_eq!(
        first
            .campaign
            .as_ref()
            .unwrap()
            .latest_outcome
            .as_ref()
            .unwrap()
            .kind,
        EncounterOutcomeKindDto::Victory
    );
    let camp = runtime.return_to_camp(first.revision).unwrap();
    runtime
        .enter_encounter(EnterEncounterRequestDto {
            expected_revision: camp.revision,
            encounter_id: "seal-guard".to_owned(),
        })
        .unwrap();
    let entered_save = runtime.encode_save().unwrap();
    let mut reopened = GameRuntime::decode_save(&entered_save).unwrap();
    assert_eq!(reopened.encode_save().unwrap(), entered_save);

    let second = play_to_outcome(&mut reopened, "precise-shot", false, true);
    let second_campaign = second.campaign.as_ref().unwrap();
    assert_eq!(second_campaign.completed_encounters.len(), 2);
    assert_eq!(
        second_campaign
            .completed_encounters
            .iter()
            .map(|entry| entry.encounter_id.as_str())
            .collect::<Vec<_>>(),
        vec!["iron-warden", "seal-guard"]
    );
    assert_eq!(
        second_campaign.party[0]
            .loadout
            .stash_items
            .iter()
            .filter(|item| item.entity_id == OPPONENT_ARMOR.raw())
            .count(),
        1
    );
    assert_eq!(
        second_campaign
            .latest_outcome
            .as_ref()
            .unwrap()
            .reward_item_id,
        None
    );
    let camp = reopened.return_to_camp(second.revision).unwrap();
    let final_encounter = reopened
        .enter_encounter(EnterEncounterRequestDto {
            expected_revision: camp.revision,
            encounter_id: "wardens-reckoning".to_owned(),
        })
        .unwrap();
    for restored in [OPPONENT.raw(), 110] {
        let opponent = &final_encounter
            .encounter
            .as_ref()
            .unwrap()
            .participants
            .iter()
            .find(|participant| participant.character.id == restored)
            .unwrap()
            .character;
        assert_eq!(opponent.health_current, opponent.health_maximum);
    }
    let final_outcome = play_to_outcome(&mut reopened, "precise-shot", false, true);
    let final_kind = final_outcome
        .campaign
        .as_ref()
        .unwrap()
        .latest_outcome
        .as_ref()
        .unwrap()
        .kind;
    let complete = reopened.return_to_camp(final_outcome.revision).unwrap();
    let complete_campaign = complete.campaign.as_ref().unwrap();
    assert_eq!(complete_campaign.phase, CampaignPhaseDto::AdventureComplete);
    assert!(complete_campaign.available_encounters.is_empty());
    assert_eq!(complete_campaign.completed_encounters.len(), 3);
    assert_eq!(
        complete_campaign.completion.as_ref().unwrap().kind,
        final_kind
    );
    let complete_save = reopened.encode_save().unwrap();
    assert_eq!(
        GameRuntime::decode_save(&complete_save)
            .unwrap()
            .encode_save()
            .unwrap(),
        complete_save
    );
}

#[test]
fn complete_encounter_defeat_has_no_reward_and_applies_bounded_recovery() {
    let mut runtime = GameRuntime::empty().unwrap();
    let camp = runtime.new_adventure(0).unwrap();
    let without_chain = runtime
        .unequip_item(UnequipItemRequestDto {
            expected_revision: camp.revision,
            item_id: PLAYER_CHAIN_ARMOR.raw(),
        })
        .unwrap();
    let without_armor = runtime
        .unequip_item(UnequipItemRequestDto {
            expected_revision: without_chain.revision,
            item_id: PLAYER_BUCKLER.raw(),
        })
        .unwrap();
    runtime
        .enter_encounter(EnterEncounterRequestDto {
            expected_revision: without_armor.revision,
            encounter_id: ENCOUNTER_ID.to_owned(),
        })
        .unwrap();
    let outcome = play_to_outcome(&mut runtime, "pass", true, false);
    let campaign = outcome.campaign.as_ref().unwrap();
    assert_eq!(campaign.phase, CampaignPhaseDto::Outcome);
    assert_eq!(
        campaign.latest_outcome.as_ref().unwrap().kind,
        EncounterOutcomeKindDto::Defeat
    );
    assert_eq!(
        campaign.latest_outcome.as_ref().unwrap().reward_item_id,
        None
    );
    assert!(!campaign.party[0]
        .loadout
        .stash_items
        .iter()
        .any(|item| item.entity_id == OPPONENT_ARMOR.raw()));
    assert_eq!(
        campaign.party[0].character.health_current, 0,
        "defeat is derived from authoritative vitality"
    );

    let outcome_save = runtime.encode_save().unwrap();
    let mut reopened = GameRuntime::decode_save(&outcome_save).unwrap();
    let camp = reopened
        .return_to_camp(reopened.snapshot().unwrap().revision)
        .unwrap();
    assert_eq!(
        camp.campaign.as_ref().unwrap().party[0]
            .character
            .health_current,
        i64::from(DEFEAT_RECOVERY_VITALITY)
    );
    assert!(camp
        .campaign
        .as_ref()
        .unwrap()
        .latest_outcome
        .as_ref()
        .is_some_and(|outcome| outcome.kind == EncounterOutcomeKindDto::Defeat));
    let recovered_save = reopened.encode_save().unwrap();
    assert_eq!(
        GameRuntime::decode_save(&recovered_save)
            .unwrap()
            .encode_save()
            .unwrap(),
        recovered_save
    );

    let mut exploring = reopened.begin_exploration(camp.revision).unwrap();
    for expected_x in 2..=9 {
        exploring = reopened
            .exploration_command(ExplorationCommandRequestDto {
                expected_revision: exploring.revision,
                command: ExplorationCommandKindDto::StepForward,
            })
            .unwrap();
        assert_eq!(
            exploring.exploration.as_ref().map(|state| state.x),
            Some(expected_x)
        );
        assert_eq!(
            exploring.campaign.as_ref().unwrap().phase,
            CampaignPhaseDto::Exploration,
            "the completed first trigger must remain consumed"
        );
        assert!(exploring.encounter.is_none());
    }

    let before_stale = reopened.snapshot().unwrap();
    assert!(matches!(
        reopened.exploration_command(ExplorationCommandRequestDto {
            expected_revision: before_stale.revision - 1,
            command: ExplorationCommandKindDto::TurnLeft,
        }),
        Err(GameRuntimeError::StaleCommand(_))
    ));
    assert_eq!(reopened.snapshot().unwrap(), before_stale);

    exploring = reopened
        .exploration_command(ExplorationCommandRequestDto {
            expected_revision: exploring.revision,
            command: ExplorationCommandKindDto::TurnRight,
        })
        .unwrap();
    exploring = reopened
        .exploration_command(ExplorationCommandRequestDto {
            expected_revision: exploring.revision,
            command: ExplorationCommandKindDto::StepForward,
        })
        .unwrap();
    assert_eq!(exploring.exploration.as_ref().map(|state| state.y), Some(2));
    exploring = reopened
        .exploration_command(ExplorationCommandRequestDto {
            expected_revision: exploring.revision,
            command: ExplorationCommandKindDto::Interact,
        })
        .unwrap();
    assert!(exploring.campaign.as_ref().unwrap().party[0]
        .loadout
        .stash_items
        .iter()
        .any(|item| item.entity_id == 227));
    exploring = reopened
        .exploration_command(ExplorationCommandRequestDto {
            expected_revision: exploring.revision,
            command: ExplorationCommandKindDto::StepForward,
        })
        .unwrap();
    exploring = reopened
        .exploration_command(ExplorationCommandRequestDto {
            expected_revision: exploring.revision,
            command: ExplorationCommandKindDto::TurnLeft,
        })
        .unwrap();
    let safe_camp = reopened
        .exploration_command(ExplorationCommandRequestDto {
            expected_revision: exploring.revision,
            command: ExplorationCommandKindDto::Interact,
        })
        .unwrap();
    assert_eq!(
        safe_camp.campaign.as_ref().unwrap().phase,
        CampaignPhaseDto::Camp
    );
    exploring = reopened.begin_exploration(safe_camp.revision).unwrap();
    exploring = reopened
        .exploration_command(ExplorationCommandRequestDto {
            expected_revision: exploring.revision,
            command: ExplorationCommandKindDto::TurnRight,
        })
        .unwrap();
    exploring = reopened
        .exploration_command(ExplorationCommandRequestDto {
            expected_revision: exploring.revision,
            command: ExplorationCommandKindDto::StepForward,
        })
        .unwrap();
    assert_eq!(exploring.exploration.as_ref().map(|state| state.y), Some(4));
    exploring = reopened
        .exploration_command(ExplorationCommandRequestDto {
            expected_revision: exploring.revision,
            command: ExplorationCommandKindDto::Interact,
        })
        .unwrap();
    assert!(exploring
        .exploration
        .as_ref()
        .unwrap()
        .door_ahead
        .as_ref()
        .is_some_and(|door| door.opened));
    let second = reopened
        .exploration_command(ExplorationCommandRequestDto {
            expected_revision: exploring.revision,
            command: ExplorationCommandKindDto::StepForward,
        })
        .unwrap();
    assert_eq!(
        second.campaign.as_ref().unwrap().phase,
        CampaignPhaseDto::Encounter
    );
    assert_eq!(
        second
            .campaign
            .as_ref()
            .unwrap()
            .active_encounter_id
            .as_deref(),
        Some("seal-guard")
    );
    let save = reopened.encode_save().unwrap();
    assert_eq!(
        GameRuntime::decode_save(&save)
            .unwrap()
            .encode_save()
            .unwrap(),
        save
    );
}

fn play_to_outcome(
    runtime: &mut GameRuntime,
    player_action: &str,
    _opponent_reacts: bool,
    player_reacts: bool,
) -> GameSnapshotDto {
    for _ in 0..512 {
        let before = runtime.snapshot().unwrap();
        let encounter = before.encounter.as_ref().unwrap();
        let party_activation = encounter
            .current_actor_id
            .and_then(|actor| {
                encounter
                    .participants
                    .iter()
                    .find(|participant| participant.character.id == actor)
            })
            .is_some_and(|participant| participant.faction == EncounterFactionDto::Party);
        if party_activation {
            if player_action == "pass" {
                let skipped = runtime.end_activation(before.revision).unwrap();
                if skipped.campaign.as_ref().unwrap().phase == CampaignPhaseDto::Outcome {
                    return skipped;
                }
                continue;
            }
            let Some(action) = encounter
                .actions
                .iter()
                .find(|action| action.id == player_action)
                .or_else(|| encounter.actions.first())
            else {
                let skipped = runtime.end_activation(before.revision).unwrap();
                if skipped.campaign.as_ref().unwrap().phase == CampaignPhaseDto::Outcome {
                    return skipped;
                }
                continue;
            };
            let target = encounter
                .legal_targets
                .iter()
                .find(|entry| entry.action_id == action.id)
                .and_then(|entry| entry.target_ids.first())
                .copied()
                .unwrap();
            let resolved = runtime
                .choose_action(ChooseActionRequestDto {
                    expected_revision: before.revision,
                    actor_id: encounter.current_actor_id.unwrap(),
                    target_id: target,
                    action_id: action.id.clone(),
                })
                .unwrap();
            if resolved.campaign.as_ref().unwrap().phase == CampaignPhaseDto::Outcome {
                return resolved;
            }
            continue;
        }

        let selected = runtime.begin_opposition_turn(before.revision).unwrap();
        if selected.campaign.as_ref().unwrap().phase == CampaignPhaseDto::Outcome {
            return selected;
        }
        let Some(prompt) = selected.encounter.as_ref().unwrap().reaction_prompt.clone() else {
            continue;
        };
        let resolved = if player_reacts && !prompt.reactions.is_empty() {
            runtime
                .apply_reaction(ApplyReactionRequestDto {
                    expected_revision: selected.revision,
                    prompt_token: prompt.token.clone(),
                    reaction_id: prompt.reactions[0].id.clone(),
                })
                .unwrap()
        } else {
            runtime
                .decline_reaction(DeclineReactionRequestDto {
                    expected_revision: selected.revision,
                    prompt_token: prompt.token,
                })
                .unwrap()
        };
        if resolved.campaign.as_ref().unwrap().phase == CampaignPhaseDto::Outcome {
            return resolved;
        }
    }
    panic!("deterministic encounter did not reach an outcome within 512 activations");
}

#[test]
fn reaction_prompt_save_rejects_without_mutation_and_reaction_resolves_the_roll() {
    let mut runtime = GameRuntime::empty().unwrap();
    let started = start_test_encounter(&mut runtime);
    let party_resolved = runtime
        .choose_action(ChooseActionRequestDto {
            expected_revision: started.revision,
            actor_id: PLAYER.raw(),
            target_id: OPPONENT.raw(),
            action_id: "longsword-strike".to_owned(),
        })
        .unwrap();
    let mut prompted = party_resolved;
    for _ in 0..32 {
        let encounter = prompted.encounter.as_ref().unwrap();
        if encounter.reaction_prompt.is_some() {
            break;
        }
        let current_actor = encounter.current_actor_id.unwrap();
        let faction = encounter
            .participants
            .iter()
            .find(|participant| participant.character.id == current_actor)
            .unwrap()
            .faction;
        prompted = if faction == EncounterFactionDto::Party {
            runtime.end_activation(prompted.revision).unwrap()
        } else {
            runtime.begin_opposition_turn(prompted.revision).unwrap()
        };
    }
    assert!(
        prompted
            .encounter
            .as_ref()
            .unwrap()
            .reaction_prompt
            .is_some(),
        "the authored encounter must expose a player reaction window"
    );

    assert_reaction_prompt_save_is_unchanged(&runtime, &prompted);

    let prompt = prompted
        .encounter
        .as_ref()
        .unwrap()
        .reaction_prompt
        .as_ref()
        .unwrap();
    let resolved = runtime
        .apply_reaction(ApplyReactionRequestDto {
            expected_revision: prompted.revision,
            prompt_token: prompt.token.clone(),
            reaction_id: prompt.reactions[0].id.clone(),
        })
        .unwrap();
    assert!(resolved
        .encounter
        .as_ref()
        .unwrap()
        .reaction_prompt
        .is_none());
    assert!(runtime.encode_save_at(resolved.revision).is_ok());
}

fn assert_reaction_prompt_save_is_unchanged(runtime: &GameRuntime, before: &GameSnapshotDto) {
    let session_before = runtime.session.as_ref().unwrap().encode_save().unwrap();
    let result = runtime.encode_save_at(before.revision);
    assert!(
        matches!(result, Err(GameRuntimeError::ReactionPromptCannotBeSaved)),
        "{result:?}"
    );
    assert_eq!(runtime.snapshot().unwrap(), *before);
    assert_eq!(
        runtime.session.as_ref().unwrap().encode_save().unwrap(),
        session_before
    );
}

#[test]
fn saturated_product_counters_and_oversized_saves_fail_before_mutation() {
    let mut runtime = GameRuntime::empty().unwrap();
    let started = start_test_encounter(&mut runtime);
    let mut save: serde_json::Value =
        serde_json::from_str(&runtime.encode_save().unwrap()).unwrap();
    save["revision"] = json!(u64::MAX);
    let mut saturated = GameRuntime::decode_save(&serde_json::to_string(&save).unwrap()).unwrap();
    let before = saturated.encode_save().unwrap();
    assert!(matches!(
        saturated.choose_action(ChooseActionRequestDto {
            expected_revision: u64::MAX,
            actor_id: PLAYER.raw(),
            target_id: OPPONENT.raw(),
            action_id: "longsword-strike".to_owned(),
        }),
        Err(GameRuntimeError::CounterOverflow)
    ));
    assert_eq!(saturated.encode_save().unwrap(), before);
    assert!(matches!(
        GameRuntime::decode_save(&"x".repeat(MAX_GAME_SAVE_BYTES + 1)),
        Err(GameRuntimeError::InvalidSave(_))
    ));
    assert!(started.encounter.is_some());
}
