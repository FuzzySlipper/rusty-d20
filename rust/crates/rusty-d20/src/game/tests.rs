use serde_json::json;

use super::*;

const PLAYER: EntityId = EntityId::new(101);
const OPPONENT: EntityId = EntityId::new(102);
const CAMP_STASH: EntityId = EntityId::new(103);
const OPPONENT_ARMOR: EntityId = EntityId::new(201);
const PLAYER_CHAIN_ARMOR: EntityId = EntityId::new(202);
const PLAYER_BUCKLER: EntityId = EntityId::new(203);
const STASH_BUCKLER: EntityId = EntityId::new(204);
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
    let loadout = &camp.campaign.as_ref().unwrap().loadout;
    assert_eq!(loadout.capacity.used, 2);
    assert_eq!(loadout.capacity.maximum, 2);
    assert_eq!(loadout.armor_defense, 16);
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
        chain_removed
            .campaign
            .as_ref()
            .unwrap()
            .loadout
            .armor_defense,
        14
    );
    let chain_restored = runtime
        .equip_item(EquipItemRequestDto {
            expected_revision: chain_removed.revision,
            item_id: PLAYER_CHAIN_ARMOR.raw(),
            slot_id: "body".to_owned(),
        })
        .unwrap();
    assert_eq!(
        chain_restored
            .campaign
            .as_ref()
            .unwrap()
            .loadout
            .armor_defense,
        16
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
    let equipped_loadout = &equipped.campaign.as_ref().unwrap().loadout;
    assert_eq!(equipped_loadout.capacity.used, 2);
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
        reopened_snapshot.campaign.as_ref().unwrap().loadout,
        equipped_loadout.clone()
    );
    let encounter = reopened
        .enter_encounter(EnterEncounterRequestDto {
            expected_revision: reopened_snapshot.revision,
            encounter_id: ENCOUNTER_ID.to_owned(),
        })
        .unwrap();
    assert_eq!(
        encounter
            .campaign
            .as_ref()
            .unwrap()
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
fn product_runtime_is_atomic_stale_safe_and_reopens_deterministically() {
    let mut runtime = GameRuntime::empty().unwrap();
    assert!(runtime.snapshot().unwrap().encounter.is_none());
    let started = start_test_encounter(&mut runtime);
    let encounter = started.encounter.unwrap();
    assert_eq!(encounter.characters.len(), 2);
    assert_eq!(encounter.actions.len(), 2);

    let before_stale = runtime.encode_save().unwrap();
    assert!(matches!(
        runtime.preview_action(PreviewActionRequestDto {
            expected_revision: 0,
            actor_id: PLAYER.raw(),
            target_id: OPPONENT.raw(),
            action_id: "longsword-strike".to_owned(),
        }),
        Err(GameRuntimeError::StaleCommand(_))
    ));
    assert_eq!(runtime.encode_save().unwrap(), before_stale);

    let previewed = runtime
        .preview_action(PreviewActionRequestDto {
            expected_revision: started.revision,
            actor_id: PLAYER.raw(),
            target_id: OPPONENT.raw(),
            action_id: "longsword-strike".to_owned(),
        })
        .unwrap();
    let pending = previewed
        .encounter
        .as_ref()
        .unwrap()
        .pending_action
        .as_ref()
        .unwrap();
    assert_eq!(pending.reactions[0].id, "parry");
    assert!(pending
        .defense_sources
        .iter()
        .any(|source| source.contains("Equipped item")));
    let reacted = runtime
        .apply_reaction(ApplyReactionRequestDto {
            expected_revision: previewed.revision,
            preview_token: pending.token.clone(),
            reaction_id: "parry".to_owned(),
        })
        .unwrap();
    let pending = reacted
        .encounter
        .as_ref()
        .unwrap()
        .pending_action
        .as_ref()
        .unwrap();
    assert_eq!(pending.defense, 17);
    let applied = runtime
        .apply_action(ApplyActionRequestDto {
            expected_revision: reacted.revision,
            preview_token: pending.token.clone(),
        })
        .unwrap();
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
        .pending_action
        .is_none());
    assert_eq!(
        reopened_snapshot.encounter.as_ref().unwrap().turn_owner,
        Some(EncounterTurnOwnerDto::Opposition)
    );
    let opposition = reopened
        .begin_opposition_turn(reopened_snapshot.revision)
        .unwrap();
    let same_opposition = same_reopened
        .begin_opposition_turn(reopened_snapshot.revision)
        .unwrap();
    assert_eq!(
        opposition.encounter.as_ref().unwrap().pending_action,
        same_opposition.encounter.as_ref().unwrap().pending_action,
        "the exact save and Rust-owned RNG position select the same opposition action"
    );
    let pending = opposition
        .encounter
        .as_ref()
        .unwrap()
        .pending_action
        .as_ref()
        .unwrap();
    assert_eq!(pending.actor_id, OPPONENT.raw());
    assert_eq!(pending.target_id, PLAYER.raw());
    assert!(matches!(
        pending.action_id.as_str(),
        "longsword-strike" | "precise-shot"
    ));
    let token = pending.token.clone();
    let advanced = reopened
        .apply_action(ApplyActionRequestDto {
            expected_revision: opposition.revision,
            preview_token: token,
        })
        .unwrap();
    let advanced_encounter = advanced.encounter.as_ref().unwrap();
    assert_eq!(advanced_encounter.turn, 1);
    assert_eq!(
        advanced_encounter.turn_owner,
        Some(EncounterTurnOwnerDto::Player)
    );
    assert!(advanced_encounter
        .log
        .last()
        .is_some_and(|entry| entry.source == "Round"
            && entry.text.contains("round 0 to 1")
            && entry
                .details
                .iter()
                .any(|detail| detail.contains("1 scheduled effect(s) expired"))));
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
    assert!(campaign
        .loadout
        .stash_items
        .iter()
        .any(|item| item.entity_id == OPPONENT_ARMOR.raw()));
    assert_eq!(outcome.encounter.as_ref().unwrap().turn_owner, None);
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
    assert!(campaign.available_encounters.is_empty());
    assert_eq!(
        campaign
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
    let outcome = play_to_outcome(&mut runtime, "longsword-strike", true, false);
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
    assert!(!campaign
        .loadout
        .stash_items
        .iter()
        .any(|item| item.entity_id == OPPONENT_ARMOR.raw()));
    assert_eq!(
        campaign.hero.health_current, 0,
        "defeat is derived from authoritative vitality"
    );

    let outcome_save = runtime.encode_save().unwrap();
    let mut reopened = GameRuntime::decode_save(&outcome_save).unwrap();
    let camp = reopened
        .return_to_camp(reopened.snapshot().unwrap().revision)
        .unwrap();
    assert_eq!(
        camp.campaign.as_ref().unwrap().hero.health_current,
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
}

#[test]
fn schema_four_rejects_outcome_that_disagrees_with_authoritative_vitality() {
    let mut active_runtime = GameRuntime::empty().unwrap();
    start_test_encounter(&mut active_runtime);
    let active_save = schema_four_save(&active_runtime.encode_save().unwrap());

    let mut forged_defeat = active_save.clone();
    forged_defeat["campaign"]["phase"] = json!("outcome");
    forged_defeat["campaign"]["turnOwner"] = serde_json::Value::Null;
    forged_defeat["campaign"]["outcome"] = json!("defeat");
    assert_vitality_mismatch_rejected(&forged_defeat);

    let mut dead_active_encounter = active_save;
    set_saved_vitality(&mut dead_active_encounter, PLAYER, 0);
    assert_vitality_mismatch_rejected(&dead_active_encounter);

    let mut victory_runtime = GameRuntime::empty().unwrap();
    start_test_encounter(&mut victory_runtime);
    let victory = play_to_outcome(&mut victory_runtime, "precise-shot", false, true);
    let victory_save =
        serde_json::to_string(&schema_four_save(&victory_runtime.encode_save().unwrap())).unwrap();
    GameRuntime::decode_save(&victory_save).unwrap();

    let mut forged_victory: serde_json::Value = serde_json::from_str(&victory_save).unwrap();
    set_saved_vitality(&mut forged_victory, OPPONENT, 1);
    assert_vitality_mismatch_rejected(&forged_victory);

    victory_runtime.return_to_camp(victory.revision).unwrap();
    GameRuntime::decode_save(
        &serde_json::to_string(&schema_four_save(&victory_runtime.encode_save().unwrap())).unwrap(),
    )
    .unwrap();

    let mut defeat_runtime = GameRuntime::empty().unwrap();
    let camp = defeat_runtime.new_adventure(0).unwrap();
    let without_chain = defeat_runtime
        .unequip_item(UnequipItemRequestDto {
            expected_revision: camp.revision,
            item_id: PLAYER_CHAIN_ARMOR.raw(),
        })
        .unwrap();
    let without_armor = defeat_runtime
        .unequip_item(UnequipItemRequestDto {
            expected_revision: without_chain.revision,
            item_id: PLAYER_BUCKLER.raw(),
        })
        .unwrap();
    defeat_runtime
        .enter_encounter(EnterEncounterRequestDto {
            expected_revision: without_armor.revision,
            encounter_id: ENCOUNTER_ID.to_owned(),
        })
        .unwrap();
    let defeat = play_to_outcome(&mut defeat_runtime, "longsword-strike", true, false);
    GameRuntime::decode_save(
        &serde_json::to_string(&schema_four_save(&defeat_runtime.encode_save().unwrap())).unwrap(),
    )
    .unwrap();
    defeat_runtime.return_to_camp(defeat.revision).unwrap();
    GameRuntime::decode_save(
        &serde_json::to_string(&schema_four_save(&defeat_runtime.encode_save().unwrap())).unwrap(),
    )
    .unwrap();
}

#[test]
fn schema_three_terminal_encounter_remains_migratable() {
    let mut encounter_runtime = GameRuntime::empty().unwrap();
    start_test_encounter(&mut encounter_runtime);
    let encounter_save = encounter_runtime.encode_save().unwrap();

    for schema in 1..=3 {
        let live = legacy_product_save(&encounter_save, schema);
        let live_snapshot = GameRuntime::decode_save(&serde_json::to_string(&live).unwrap())
            .unwrap()
            .snapshot()
            .unwrap();
        assert_eq!(
            live_snapshot.campaign.as_ref().unwrap().phase,
            CampaignPhaseDto::Encounter
        );
        assert_eq!(
            live_snapshot.encounter.as_ref().unwrap().turn_owner,
            Some(EncounterTurnOwnerDto::Player)
        );

        let mut legacy_victory = live.clone();
        set_saved_vitality(&mut legacy_victory, OPPONENT, 0);
        let mut migrated_victory =
            GameRuntime::decode_save(&serde_json::to_string(&legacy_victory).unwrap()).unwrap();
        let victory = migrated_victory.snapshot().unwrap();
        let campaign = victory.campaign.as_ref().unwrap();
        assert_eq!(campaign.phase, CampaignPhaseDto::Outcome);
        assert_eq!(
            campaign.latest_outcome.as_ref().unwrap().kind,
            EncounterOutcomeKindDto::Victory
        );
        assert_eq!(
            campaign
                .loadout
                .stash_items
                .iter()
                .filter(|item| item.entity_id == OPPONENT_ARMOR.raw())
                .count(),
            1
        );
        let schema_four_victory = migrated_victory.encode_save().unwrap();
        let schema_four_value: serde_json::Value =
            serde_json::from_str(&schema_four_victory).unwrap();
        assert_eq!(
            schema_four_value["schemaVersion"],
            json!(GAME_SAVE_SCHEMA_VERSION)
        );
        assert_eq!(schema_four_value["nextOperation"], json!(3));
        assert_eq!(
            GameRuntime::decode_save(&schema_four_victory)
                .unwrap()
                .encode_save()
                .unwrap(),
            schema_four_victory
        );
        let camp = migrated_victory.return_to_camp(victory.revision).unwrap();
        assert_eq!(
            camp.campaign
                .as_ref()
                .unwrap()
                .loadout
                .stash_items
                .iter()
                .filter(|item| item.entity_id == OPPONENT_ARMOR.raw())
                .count(),
            1
        );

        let mut legacy_defeat = live.clone();
        set_saved_vitality(&mut legacy_defeat, PLAYER, 0);
        let mut migrated_defeat =
            GameRuntime::decode_save(&serde_json::to_string(&legacy_defeat).unwrap()).unwrap();
        let defeat = migrated_defeat.snapshot().unwrap();
        let campaign = defeat.campaign.as_ref().unwrap();
        assert_eq!(campaign.phase, CampaignPhaseDto::Outcome);
        assert_eq!(
            campaign.latest_outcome.as_ref().unwrap().kind,
            EncounterOutcomeKindDto::Defeat
        );
        assert!(!campaign
            .loadout
            .stash_items
            .iter()
            .any(|item| item.entity_id == OPPONENT_ARMOR.raw()));
        let schema_four_defeat = migrated_defeat.encode_save().unwrap();
        assert_eq!(
            GameRuntime::decode_save(&schema_four_defeat)
                .unwrap()
                .encode_save()
                .unwrap(),
            schema_four_defeat
        );
        let recovered = migrated_defeat.return_to_camp(defeat.revision).unwrap();
        assert_eq!(
            recovered.campaign.as_ref().unwrap().hero.health_current,
            i64::from(DEFEAT_RECOVERY_VITALITY)
        );

        let mut impossible = live;
        set_saved_vitality(&mut impossible, PLAYER, 0);
        set_saved_vitality(&mut impossible, OPPONENT, 0);
        assert_legacy_vitality_rejected(&impossible);
    }

    let mut camp_runtime = GameRuntime::empty().unwrap();
    camp_runtime.new_adventure(0).unwrap();
    let camp_save = camp_runtime.encode_save().unwrap();
    for schema in 2..=3 {
        let live_camp = legacy_product_save(&camp_save, schema);
        let migrated = GameRuntime::decode_save(&serde_json::to_string(&live_camp).unwrap())
            .unwrap()
            .snapshot()
            .unwrap();
        assert_eq!(
            migrated.campaign.as_ref().unwrap().phase,
            CampaignPhaseDto::Camp
        );

        let mut impossible_camp = live_camp;
        set_saved_vitality(&mut impossible_camp, OPPONENT, 0);
        assert_legacy_vitality_rejected(&impossible_camp);
    }
}

fn play_to_outcome(
    runtime: &mut GameRuntime,
    player_action: &str,
    opponent_reacts: bool,
    player_reacts: bool,
) -> GameSnapshotDto {
    for _ in 0..64 {
        let before_player = runtime.snapshot().unwrap();
        let previewed = runtime
            .preview_action(PreviewActionRequestDto {
                expected_revision: before_player.revision,
                actor_id: PLAYER.raw(),
                target_id: OPPONENT.raw(),
                action_id: player_action.to_owned(),
            })
            .unwrap();
        let mut pending = previewed
            .encounter
            .as_ref()
            .unwrap()
            .pending_action
            .clone()
            .unwrap();
        let mut current = previewed;
        if opponent_reacts && !pending.reactions.is_empty() {
            current = runtime
                .apply_reaction(ApplyReactionRequestDto {
                    expected_revision: current.revision,
                    preview_token: pending.token.clone(),
                    reaction_id: pending.reactions[0].id.clone(),
                })
                .unwrap();
            pending = current
                .encounter
                .as_ref()
                .unwrap()
                .pending_action
                .clone()
                .unwrap();
        }
        let player_result = runtime
            .apply_action(ApplyActionRequestDto {
                expected_revision: current.revision,
                preview_token: pending.token,
            })
            .unwrap();
        if player_result.campaign.as_ref().unwrap().phase == CampaignPhaseDto::Outcome {
            return player_result;
        }

        let opposition = runtime
            .begin_opposition_turn(player_result.revision)
            .unwrap();
        let mut pending = opposition
            .encounter
            .as_ref()
            .unwrap()
            .pending_action
            .clone()
            .unwrap();
        let mut current = opposition;
        if player_reacts && !pending.reactions.is_empty() {
            current = runtime
                .apply_reaction(ApplyReactionRequestDto {
                    expected_revision: current.revision,
                    preview_token: pending.token.clone(),
                    reaction_id: pending.reactions[0].id.clone(),
                })
                .unwrap();
            pending = current
                .encounter
                .as_ref()
                .unwrap()
                .pending_action
                .clone()
                .unwrap();
        }
        let opposition_result = runtime
            .apply_action(ApplyActionRequestDto {
                expected_revision: current.revision,
                preview_token: pending.token,
            })
            .unwrap();
        if opposition_result.campaign.as_ref().unwrap().phase == CampaignPhaseDto::Outcome {
            return opposition_result;
        }
    }
    panic!("deterministic encounter did not reach an outcome within 64 rounds");
}

fn set_saved_vitality(save: &mut serde_json::Value, entity: EntityId, current: i64) {
    let tracks = save["session"]["entityState"]["registeredComponents"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|registered| registered["typeId"] == "rusty.mechanics.tracks")
        .unwrap();
    let entity_tracks = tracks["values"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["entity"] == json!(entity.raw()))
        .unwrap();
    let vitality = entity_tracks["value"]["values"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|track| track["track"] == "vitality")
        .unwrap();
    vitality["current"] = json!(current);
}

fn assert_vitality_mismatch_rejected(save: &serde_json::Value) {
    let error = GameRuntime::decode_save(&serde_json::to_string(save).unwrap()).unwrap_err();
    assert!(
        matches!(
            &error,
            GameRuntimeError::InvalidSave(message)
                if message.contains("contradict authoritative vitality")
        ),
        "unexpected save rejection: {error:?}"
    );
}

fn assert_legacy_vitality_rejected(save: &serde_json::Value) {
    let error = GameRuntime::decode_save(&serde_json::to_string(save).unwrap()).unwrap_err();
    assert!(
        matches!(
            &error,
            GameRuntimeError::InvalidSave(message)
                if message.contains("impossible phase/vitality combination")
        ),
        "unexpected legacy save rejection: {error:?}"
    );
}

fn legacy_product_save(input: &str, schema: u32) -> serde_json::Value {
    let mut save: serde_json::Value = if schema <= 2 {
        serde_json::from_str(&downgrade_to_pre_loadout_v2(input)).unwrap()
    } else {
        serde_json::from_str(input).unwrap()
    };
    save["schemaVersion"] = json!(schema);
    save.as_object_mut()
        .unwrap()
        .remove("compositionFingerprint");
    save["session"]["rulesetFingerprint"] = json!(legacy_rules_fingerprint());
    if schema == 1 {
        save.as_object_mut().unwrap().remove("campaign");
    } else {
        save["campaign"]
            .as_object_mut()
            .unwrap()
            .remove("turnOwner");
        save["campaign"].as_object_mut().unwrap().remove("outcome");
        save["campaign"]
            .as_object_mut()
            .unwrap()
            .remove("resolvedEncounterId");
    }
    save
}

fn schema_four_save(input: &str) -> serde_json::Value {
    let mut save: serde_json::Value = serde_json::from_str(input).unwrap();
    save["schemaVersion"] = json!(4);
    save.as_object_mut()
        .unwrap()
        .remove("compositionFingerprint");
    save["campaign"]
        .as_object_mut()
        .unwrap()
        .remove("resolvedEncounterId");
    save["session"]["rulesetFingerprint"] = json!(legacy_rules_fingerprint());
    save
}

fn legacy_rules_fingerprint() -> String {
    AuthoredAdventureCatalog::builtin()
        .unwrap()
        .rules_for_package("steel-guard")
        .unwrap()
        .fingerprint()
        .to_owned()
}

#[test]
fn campaign_phases_and_legacy_migration_are_strict_and_fail_atomic() {
    let mut runtime = GameRuntime::empty().unwrap();
    assert!(runtime.snapshot().unwrap().campaign.is_none());
    assert!(matches!(
        runtime.new_adventure(1),
        Err(GameRuntimeError::StaleCommand(_))
    ));
    assert!(runtime.snapshot().unwrap().campaign.is_none());

    let camp = runtime.new_adventure(0).unwrap();
    assert_eq!(
        camp.campaign.as_ref().unwrap().phase,
        CampaignPhaseDto::Camp
    );
    assert!(camp.encounter.is_none());
    let camp_save = runtime.encode_save().unwrap();
    assert_eq!(
        GameRuntime::decode_save(&camp_save)
            .unwrap()
            .snapshot()
            .unwrap(),
        {
            let mut saved = camp.clone();
            saved.saved = true;
            saved
        }
    );

    let before_invalid = runtime.snapshot().unwrap();
    assert!(matches!(
        runtime.enter_encounter(EnterEncounterRequestDto {
            expected_revision: camp.revision,
            encounter_id: "unknown".to_owned(),
        }),
        Err(GameRuntimeError::InvalidCommand(_))
    ));
    assert_eq!(runtime.snapshot().unwrap(), before_invalid);
    assert!(matches!(
        runtime.preview_action(PreviewActionRequestDto {
            expected_revision: camp.revision,
            actor_id: PLAYER.raw(),
            target_id: OPPONENT.raw(),
            action_id: "longsword-strike".to_owned(),
        }),
        Err(GameRuntimeError::WrongPhase(_))
    ));
    assert_eq!(runtime.snapshot().unwrap(), before_invalid);

    let encounter = runtime
        .enter_encounter(EnterEncounterRequestDto {
            expected_revision: camp.revision,
            encounter_id: ENCOUNTER_ID.to_owned(),
        })
        .unwrap();
    assert_eq!(
        encounter.campaign.as_ref().unwrap().phase,
        CampaignPhaseDto::Encounter
    );
    assert!(encounter.encounter.is_some());
    let before_duplicate = runtime.snapshot().unwrap();
    assert!(matches!(
        runtime.enter_encounter(EnterEncounterRequestDto {
            expected_revision: encounter.revision,
            encounter_id: ENCOUNTER_ID.to_owned(),
        }),
        Err(GameRuntimeError::InvalidCommand(_))
    ));
    assert_eq!(runtime.snapshot().unwrap(), before_duplicate);

    let legacy_v2 = downgrade_to_pre_loadout_v2(&runtime.encode_save().unwrap());
    let mut legacy: serde_json::Value = serde_json::from_str(&legacy_v2).unwrap();
    legacy["schemaVersion"] = json!(1);
    legacy.as_object_mut().unwrap().remove("campaign");
    let migrated = GameRuntime::decode_save(&serde_json::to_string(&legacy).unwrap()).unwrap();
    assert_eq!(
        migrated.snapshot().unwrap().campaign.unwrap().phase,
        CampaignPhaseDto::Encounter
    );
    let migrated_save: serde_json::Value =
        serde_json::from_str(&migrated.encode_save().unwrap()).unwrap();
    assert_eq!(
        migrated_save["schemaVersion"],
        json!(GAME_SAVE_SCHEMA_VERSION)
    );

    let migrated_v2 = GameRuntime::decode_save(&legacy_v2).unwrap();
    let migrated_loadout = migrated_v2.snapshot().unwrap().campaign.unwrap().loadout;
    assert_eq!(migrated_loadout.capacity.used, 2);
    assert_eq!(migrated_loadout.armor_defense, 16);
    assert_eq!(migrated_loadout.stash_items.len(), 1);

    let mut wrong_legacy_catalog: serde_json::Value = serde_json::from_str(&legacy_v2).unwrap();
    let registered = wrong_legacy_catalog["session"]["entityState"]["registeredComponents"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|registered| registered["typeId"] == "rusty.mechanics.stats")
        .unwrap();
    registered["values"][0]["value"]["catalogVersion"] = json!("rusty-d20.v2");
    assert!(matches!(
        GameRuntime::decode_save(&serde_json::to_string(&wrong_legacy_catalog).unwrap()),
        Err(GameRuntimeError::Save(SessionSaveError::InvalidState(
            D20SessionError::LegacyCatalogVersionMismatch { .. }
        )))
    ));

    let mut invalid: serde_json::Value =
        serde_json::from_str(&runtime.encode_save().unwrap()).unwrap();
    invalid["campaign"]["activeEncounterId"] = serde_json::Value::Null;
    assert!(matches!(
        GameRuntime::decode_save(&serde_json::to_string(&invalid).unwrap()),
        Err(GameRuntimeError::InvalidSave(_))
    ));

    let mut partial_loadout: serde_json::Value =
        serde_json::from_str(&runtime.encode_save().unwrap()).unwrap();
    let inventory = partial_loadout["session"]["entityState"]["registeredComponents"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|registered| registered["typeId"] == "rusty.mechanics.inventory")
        .unwrap();
    inventory["values"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["entity"] != json!(CAMP_STASH.raw()));
    assert!(matches!(
        GameRuntime::decode_save(&serde_json::to_string(&partial_loadout).unwrap()),
        Err(GameRuntimeError::InvalidSave(_))
    ));
}

fn downgrade_to_pre_loadout_v2(input: &str) -> String {
    let mut save: serde_json::Value = serde_json::from_str(input).unwrap();
    save["schemaVersion"] = json!(2);
    save.as_object_mut()
        .unwrap()
        .remove("compositionFingerprint");
    save["campaign"]
        .as_object_mut()
        .unwrap()
        .remove("turnOwner");
    save["campaign"].as_object_mut().unwrap().remove("outcome");
    save["campaign"]
        .as_object_mut()
        .unwrap()
        .remove("resolvedEncounterId");
    save["session"]["rulesetFingerprint"] = json!(legacy_rules_fingerprint());
    save["session"]["schemaVersion"] = json!(1);
    let state = save["session"]["entityState"].as_object_mut().unwrap();
    state
        .get_mut("entities")
        .unwrap()
        .as_array_mut()
        .unwrap()
        .retain(|entity| !matches!(entity["id"].as_u64().unwrap(), 103 | 202 | 203 | 204));
    for registered in state
        .get_mut("registeredComponents")
        .unwrap()
        .as_array_mut()
        .unwrap()
    {
        let type_id = registered["typeId"].as_str().unwrap().to_owned();
        registered
            .get_mut("values")
            .unwrap()
            .as_array_mut()
            .unwrap()
            .retain(|entry| !matches!(entry["entity"].as_u64().unwrap(), 103 | 202 | 203 | 204));
        if type_id.starts_with("rusty.mechanics.") {
            for entry in registered["values"].as_array_mut().unwrap() {
                if let Some(value) = entry["value"].as_object_mut() {
                    if value.contains_key("catalogVersion") {
                        value.insert("catalogVersion".to_owned(), json!("rusty-d20.v1"));
                    }
                }
            }
        }
        if type_id == "rusty.mechanics.equipment" {
            for entry in registered["values"].as_array_mut().unwrap() {
                if entry["entity"] == json!(PLAYER.raw()) {
                    entry["value"]["assignments"] = json!([]);
                }
            }
        }
    }
    state
        .get_mut("registeredComponents")
        .unwrap()
        .as_array_mut()
        .unwrap()
        .retain(|registered| registered["typeId"] != "rusty.mechanics.inventory");
    serde_json::to_string(&save).unwrap()
}

#[test]
fn preview_only_and_reacted_pending_saves_reject_without_mutation() {
    let mut runtime = GameRuntime::empty().unwrap();
    let started = start_test_encounter(&mut runtime);
    let previewed = runtime
        .preview_action(PreviewActionRequestDto {
            expected_revision: started.revision,
            actor_id: PLAYER.raw(),
            target_id: OPPONENT.raw(),
            action_id: "longsword-strike".to_owned(),
        })
        .unwrap();

    assert_pending_save_is_unchanged(&runtime, &previewed);

    let pending_token = previewed
        .encounter
        .as_ref()
        .unwrap()
        .pending_action
        .as_ref()
        .unwrap()
        .token
        .clone();
    let reacted = runtime
        .apply_reaction(ApplyReactionRequestDto {
            expected_revision: previewed.revision,
            preview_token: pending_token,
            reaction_id: "parry".to_owned(),
        })
        .unwrap();
    let opponent = reacted
        .encounter
        .as_ref()
        .unwrap()
        .characters
        .iter()
        .find(|character| character.id == OPPONENT.raw())
        .unwrap();
    assert!(opponent
        .resources
        .iter()
        .any(|resource| resource.id == "guard" && resource.current == 1));
    assert!(opponent
        .effects
        .iter()
        .any(|effect| effect.starts_with("Parry Stance")));

    assert_pending_save_is_unchanged(&runtime, &reacted);
}

fn assert_pending_save_is_unchanged(runtime: &GameRuntime, before: &GameSnapshotDto) {
    let session_before = runtime.session.as_ref().unwrap().encode_save().unwrap();
    assert!(matches!(
        runtime.encode_save_at(before.revision),
        Err(GameRuntimeError::PendingActionCannotBeSaved)
    ));
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
        saturated.preview_action(PreviewActionRequestDto {
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
