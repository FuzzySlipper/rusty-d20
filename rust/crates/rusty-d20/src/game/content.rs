use super::*;

pub(super) fn character_entity(
    rules: &D20Ruleset,
    adventure: &AdventureDefinition,
    character: &D20Id,
) -> Result<EntityId, GameRuntimeError> {
    if !adventure.characters.contains(character) {
        return Err(GameRuntimeError::InvalidState(format!(
            "character {character} is not part of adventure {}",
            adventure.id
        )));
    }
    rules
        .character_template(character)
        .map(|definition| EntityId::new(definition.entity_id))
        .ok_or_else(|| {
            GameRuntimeError::InvalidState(format!("character template {character} is missing"))
        })
}

pub(super) fn storage_entity(
    rules: &D20Ruleset,
    adventure: &AdventureDefinition,
    storage: &D20Id,
) -> Result<EntityId, GameRuntimeError> {
    if !adventure.storage.contains(storage) {
        return Err(GameRuntimeError::InvalidState(format!(
            "storage {storage} is not part of adventure {}",
            adventure.id
        )));
    }
    rules
        .storage(storage)
        .map(|definition| EntityId::new(definition.entity_id))
        .ok_or_else(|| GameRuntimeError::InvalidState(format!("storage {storage} is missing")))
}

pub(super) fn owner_entity(
    rules: &D20Ruleset,
    adventure: &AdventureDefinition,
    owner: &D20Id,
) -> Result<EntityId, GameRuntimeError> {
    character_entity(rules, adventure, owner).or_else(|_| storage_entity(rules, adventure, owner))
}

pub(super) fn character_seed(definition: &CharacterTemplateDefinition) -> CharacterSeed {
    CharacterSeed {
        entity: EntityId::new(definition.entity_id),
        name: definition.name.clone(),
        vitality: definition.vitality,
        abilities: definition
            .abilities
            .iter()
            .map(|(ability, score)| AbilityScore::new(ability.clone(), *score))
            .collect(),
        resources: definition
            .resources
            .iter()
            .map(|(resource, current)| ActionResource::new(resource.clone(), *current))
            .collect(),
        affinities: definition
            .affinities
            .iter()
            .map(|affinity| AffinitySeed {
                damage_type: affinity.damage_type.clone(),
                affinity: match affinity.affinity {
                    CharacterAffinityKindDefinition::Resistant => DamageAffinity::Resistant,
                    CharacterAffinityKindDefinition::Vulnerable => DamageAffinity::Vulnerable,
                },
            })
            .collect(),
    }
}

pub(super) fn product_armor_items(
    rules: &D20Ruleset,
    adventure: &AdventureDefinition,
) -> Result<Vec<ArmorItemSeed>, GameRuntimeError> {
    adventure
        .items
        .iter()
        .map(|item| {
            let definition = rules.item_instance(item).ok_or_else(|| {
                GameRuntimeError::InvalidState(format!("item instance {item} is missing"))
            })?;
            Ok(ArmorItemSeed {
                entity: EntityId::new(definition.entity_id),
                owner: owner_entity(rules, adventure, &definition.owner)?,
                name: definition.name.clone(),
                armor: definition.armor.clone(),
            })
        })
        .collect()
}

pub(super) fn install_product_loadout(
    rules: &D20Ruleset,
    adventure: &AdventureDefinition,
    session: &mut D20Session,
) -> Result<(), GameRuntimeError> {
    let inventory = adventure
        .characters
        .iter()
        .map(|character| {
            let definition = rules.character_template(character).ok_or_else(|| {
                GameRuntimeError::InvalidState(format!("character template {character} is missing"))
            })?;
            Ok(InventorySeed {
                owner: EntityId::new(definition.entity_id),
                maximum_items: definition.inventory_capacity,
            })
        })
        .collect::<Result<Vec<_>, GameRuntimeError>>()?;
    let storage = adventure
        .storage
        .iter()
        .map(|storage| {
            let definition = rules.storage(storage).ok_or_else(|| {
                GameRuntimeError::InvalidState(format!("storage {storage} is missing"))
            })?;
            Ok(StorageSeed {
                entity: EntityId::new(definition.entity_id),
                name: definition.name.clone(),
                maximum_items: definition.capacity,
            })
        })
        .collect::<Result<Vec<_>, GameRuntimeError>>()?;
    let items = product_armor_items(rules, adventure)?
        .into_iter()
        .filter(|item| session.entities().core(item.entity).is_none())
        .collect();
    session.install_loadout(inventory, storage, items)?;
    equip_initial_loadout(rules, adventure, session)
}

pub(super) fn validate_product_state(
    rules: &D20Ruleset,
    adventure: &AdventureDefinition,
    session: &D20Session,
    campaign: &CampaignState,
) -> Result<(), GameRuntimeError> {
    let expected_count =
        adventure.characters.len() + adventure.storage.len() + adventure.items.len();
    if session.entities().total_count() != expected_count {
        return Err(GameRuntimeError::InvalidSave(format!(
            "adventure {} entity set is inconsistent",
            adventure.id
        )));
    }
    let mut expected_inventory_owners = adventure
        .characters
        .iter()
        .map(|character| character_entity(rules, adventure, character))
        .chain(
            adventure
                .storage
                .iter()
                .map(|storage| storage_entity(rules, adventure, storage)),
        )
        .collect::<Result<Vec<_>, _>>()?;
    expected_inventory_owners.sort();
    let inventory_owners = session
        .entities()
        .components::<InventoryComponent>()?
        .map(|(entity, _)| entity)
        .collect::<Vec<_>>();
    if inventory_owners != expected_inventory_owners {
        return Err(GameRuntimeError::InvalidSave(format!(
            "adventure {} inventory owners are inconsistent",
            adventure.id
        )));
    }
    let mut expected_items = adventure
        .items
        .iter()
        .map(|item| {
            rules
                .item_instance(item)
                .map(|definition| EntityId::new(definition.entity_id))
                .ok_or_else(|| {
                    GameRuntimeError::InvalidState(format!("item instance {item} is missing"))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    expected_items.sort();
    let item_entities = session
        .entities()
        .components::<ItemComponent>()?
        .map(|(entity, _)| entity)
        .collect::<Vec<_>>();
    if item_entities != expected_items {
        return Err(GameRuntimeError::InvalidSave(format!(
            "adventure {} item set is inconsistent",
            adventure.id
        )));
    }

    for character in &adventure.characters {
        let definition = rules
            .character_template(character)
            .expect("compiled reference");
        validate_inventory(
            session,
            EntityId::new(definition.entity_id),
            definition.inventory_capacity,
        )?;
    }
    for storage in &adventure.storage {
        let definition = rules.storage(storage).expect("compiled reference");
        validate_inventory(
            session,
            EntityId::new(definition.entity_id),
            definition.capacity,
        )?;
    }
    let allowed_owners = expected_inventory_owners;
    for item in &adventure.items {
        let definition = rules.item_instance(item).expect("compiled reference");
        let entity = EntityId::new(definition.entity_id);
        if session
            .entities()
            .contained_in(entity)
            .is_none_or(|owner| !allowed_owners.contains(&owner))
        {
            return Err(GameRuntimeError::InvalidSave(format!(
                "loadout item {} containment is inconsistent",
                entity.raw()
            )));
        }
        let actual = session
            .entities()
            .component::<ItemComponent>(entity)?
            .expect("validated product item component exists");
        if actual.definition().as_str() != format!("armor.{}", definition.armor) {
            return Err(GameRuntimeError::InvalidSave(format!(
                "loadout item {} definition is inconsistent",
                entity.raw()
            )));
        }
    }
    for encounter in adventure
        .encounters
        .iter()
        .filter_map(|encounter| rules.encounter(encounter))
    {
        let Some(reward) = encounter.victory.reward_item.as_ref() else {
            continue;
        };
        let reward = rules
            .item_instance(reward)
            .expect("compiled reward reference");
        let reward_entity = EntityId::new(reward.entity_id);
        let reward_owner = session.entities().contained_in(reward_entity);
        let original_owner = owner_entity(rules, adventure, &reward.owner)?;
        let original_equipment = session
            .entities()
            .component::<EquipmentComponent>(original_owner)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidSave("reward owner equipment is missing".to_owned())
            })?;
        let hero = character_entity(rules, adventure, &adventure.hero)?;
        let stash = storage_entity(rules, adventure, &adventure.camp_storage)?;
        let victory = campaign.completed_encounters.iter().any(|completed| {
            completed.encounter_id == encounter.id.as_str()
                && completed.outcome == EncounterOutcome::Victory
        });
        let reward_is_claimed = matches!(reward_owner, Some(owner) if owner == hero || owner == stash)
            && original_equipment
                .assignments()
                .iter()
                .all(|assignment| assignment.item != reward_entity);
        let reward_is_intact = reward_owner == Some(original_owner)
            && original_equipment
                .assignments()
                .iter()
                .any(|assignment| assignment.item == reward_entity);
        if (victory && !reward_is_claimed) || (!victory && !reward_is_intact) {
            return Err(GameRuntimeError::InvalidSave(
                "encounter reward state is inconsistent with the campaign outcome".to_owned(),
            ));
        }
    }
    validate_campaign_vitality(rules, adventure, session, campaign)?;
    Ok(())
}

pub(super) fn validate_campaign_vitality(
    rules: &D20Ruleset,
    adventure: &AdventureDefinition,
    session: &D20Session,
    campaign: &CampaignState,
) -> Result<(), GameRuntimeError> {
    let encounter = current_encounter_definition(rules, adventure, campaign)?;
    let player = character_entity(rules, adventure, &adventure.hero)?;
    let opponent = character_entity(rules, adventure, &encounter.opponent)?;
    let player_vitality = saved_vitality(session, player)?;
    let opponent_vitality = saved_vitality(session, opponent)?;
    let valid = match (campaign.phase, campaign.outcome) {
        (CampaignPhase::Encounter, None) | (CampaignPhase::Camp, None) => {
            player_vitality > 0 && opponent_vitality > 0
        }
        (CampaignPhase::Outcome | CampaignPhase::Camp, Some(EncounterOutcome::Victory)) => {
            player_vitality > 0 && opponent_vitality == 0
        }
        (CampaignPhase::Outcome, Some(EncounterOutcome::Defeat)) => {
            player_vitality == 0 && opponent_vitality > 0
        }
        (CampaignPhase::Camp, Some(EncounterOutcome::Defeat)) => {
            player_vitality > 0 && opponent_vitality > 0
        }
        (CampaignPhase::Encounter, Some(_)) | (CampaignPhase::Outcome, None) => false,
    };
    if !valid {
        return Err(GameRuntimeError::InvalidSave(format!(
            "campaign phase/outcome contradict authoritative vitality: player={player_vitality}, opponent={opponent_vitality}"
        )));
    }
    Ok(())
}

pub(super) fn saved_vitality(
    session: &D20Session,
    entity: EntityId,
) -> Result<i64, GameRuntimeError> {
    session
        .entities()
        .component::<TracksComponent>(entity)?
        .and_then(|tracks| {
            tracks
                .values()
                .iter()
                .find(|value| value.track().as_str() == "vitality")
                .map(|value| value.current().get())
        })
        .ok_or_else(|| {
            GameRuntimeError::InvalidSave(format!(
                "entity {} authoritative vitality is missing",
                entity.raw()
            ))
        })
}

pub(super) fn migrate_legacy_campaign(
    rules: &D20Ruleset,
    adventure: &AdventureDefinition,
    session: &mut D20Session,
    mut campaign: CampaignState,
    next_operation: &mut u64,
) -> Result<CampaignState, GameRuntimeError> {
    let encounter = current_encounter_definition(rules, adventure, &campaign)?;
    let player = character_entity(rules, adventure, &adventure.hero)?;
    let opponent = character_entity(rules, adventure, &encounter.opponent)?;
    let player_vitality = saved_vitality(session, player)?;
    let opponent_vitality = saved_vitality(session, opponent)?;
    match campaign.phase {
        CampaignPhase::Encounter if player_vitality > 0 && opponent_vitality > 0 => {}
        CampaignPhase::Encounter if player_vitality > 0 && opponent_vitality == 0 => {
            transfer_victory_reward(rules, adventure, encounter, session, next_operation)?;
            campaign.phase = CampaignPhase::Outcome;
            campaign.resolved_encounter_id = campaign.active_encounter_id.clone();
            campaign.turn_owner = None;
            campaign.outcome = Some(EncounterOutcome::Victory);
            campaign.completed_encounters.push(CompletedEncounter {
                encounter_id: encounter.id.to_string(),
                outcome: EncounterOutcome::Victory,
            });
        }
        CampaignPhase::Encounter if player_vitality == 0 && opponent_vitality > 0 => {
            campaign.phase = CampaignPhase::Outcome;
            campaign.resolved_encounter_id = campaign.active_encounter_id.clone();
            campaign.turn_owner = None;
            campaign.outcome = Some(EncounterOutcome::Defeat);
            campaign.completed_encounters.push(CompletedEncounter {
                encounter_id: encounter.id.to_string(),
                outcome: EncounterOutcome::Defeat,
            });
        }
        CampaignPhase::Camp if player_vitality > 0 && opponent_vitality > 0 => {}
        CampaignPhase::Encounter | CampaignPhase::Camp | CampaignPhase::Outcome => {
            return Err(GameRuntimeError::InvalidSave(format!(
                "legacy campaign has an impossible phase/vitality combination: player={player_vitality}, opponent={opponent_vitality}"
            )));
        }
    }
    Ok(campaign)
}

pub(super) fn transfer_victory_reward(
    rules: &D20Ruleset,
    adventure: &AdventureDefinition,
    encounter: &EncounterDefinition,
    session: &mut D20Session,
    next_operation: &mut u64,
) -> Result<(), GameRuntimeError> {
    if *next_operation == 0 {
        return Err(GameRuntimeError::InvalidSave(
            "next operation identity must be nonzero".to_owned(),
        ));
    }
    let reward = encounter.victory.reward_item.as_ref().ok_or_else(|| {
        GameRuntimeError::InvalidState(format!("encounter {} has no victory reward", encounter.id))
    })?;
    let reward = rules.item_instance(reward).ok_or_else(|| {
        GameRuntimeError::InvalidState(format!("reward item {reward} is missing"))
    })?;
    let owner = owner_entity(rules, adventure, &reward.owner)?;
    let item = EntityId::new(reward.entity_id);
    let unequip_serial = *next_operation;
    session.unequip_armor(
        owner,
        item,
        operation(&format!("reward-unequip-{unequip_serial}"))?,
    )?;
    *next_operation = next_operation
        .checked_add(1)
        .ok_or(GameRuntimeError::CounterOverflow)?;
    let transfer_serial = *next_operation;
    session.transfer_armor(
        item,
        owner,
        storage_entity(rules, adventure, &adventure.camp_storage)?,
        operation(&format!("reward-transfer-{transfer_serial}"))?,
    )?;
    *next_operation = next_operation
        .checked_add(1)
        .ok_or(GameRuntimeError::CounterOverflow)?;
    Ok(())
}

pub(super) fn validate_inventory(
    session: &D20Session,
    owner: EntityId,
    expected_maximum: u64,
) -> Result<(), GameRuntimeError> {
    let view = session.inventory_view(owner)?;
    let capacity = view
        .capacity()
        .iter()
        .find(|usage| usage.metric.as_str() == "carried-items")
        .ok_or_else(|| {
            GameRuntimeError::InvalidSave(format!(
                "inventory {} carried-items capacity is missing",
                owner.raw()
            ))
        })?;
    if capacity.maximum != Some(expected_maximum) {
        return Err(GameRuntimeError::InvalidSave(format!(
            "inventory {} carried-items maximum is inconsistent",
            owner.raw()
        )));
    }
    Ok(())
}

pub(super) fn equip_initial_loadout(
    rules: &D20Ruleset,
    adventure: &AdventureDefinition,
    session: &mut D20Session,
) -> Result<(), GameRuntimeError> {
    for item in &adventure.items {
        let definition = rules.item_instance(item).expect("compiled item reference");
        if !definition.equipped {
            continue;
        }
        let owner = owner_entity(rules, adventure, &definition.owner)?;
        let item_entity = EntityId::new(definition.entity_id);
        let already_equipped = session
            .entities()
            .component::<EquipmentComponent>(owner)?
            .is_some_and(|equipment| {
                equipment
                    .assignments()
                    .iter()
                    .any(|assignment| assignment.item == item_entity)
            });
        if !already_equipped {
            session.equip_armor(
                owner,
                item_entity,
                &definition.armor,
                operation(&format!("equip-authored-{item}"))?,
            )?;
        }
    }
    Ok(())
}

pub(super) fn product_loadout_item<'a>(
    rules: &'a D20Ruleset,
    adventure: &AdventureDefinition,
    item: EntityId,
) -> Result<&'a ItemInstanceDefinition, GameRuntimeError> {
    adventure
        .items
        .iter()
        .filter_map(|id| rules.item_instance(id))
        .find(|definition| definition.entity_id == item.raw())
        .ok_or_else(|| {
            GameRuntimeError::InvalidCommand(format!(
                "entity {} is not an authored loadout item",
                item.raw()
            ))
        })
}

pub(super) fn current_encounter_definition<'a>(
    rules: &'a D20Ruleset,
    adventure: &AdventureDefinition,
    campaign: &CampaignState,
) -> Result<&'a EncounterDefinition, GameRuntimeError> {
    let encounter = campaign
        .active_encounter_id
        .as_deref()
        .or(campaign.resolved_encounter_id.as_deref())
        .and_then(|active| {
            adventure
                .encounters
                .iter()
                .find(|candidate| candidate.as_str() == active)
        })
        .or_else(|| adventure.encounters.first())
        .ok_or_else(|| {
            GameRuntimeError::InvalidState(format!("adventure {} has no encounter", adventure.id))
        })?;
    rules.encounter(encounter).ok_or_else(|| {
        GameRuntimeError::InvalidState(format!("encounter definition {encounter} is missing"))
    })
}

pub(super) fn next_available_encounter_definition<'a>(
    rules: &'a D20Ruleset,
    adventure: &AdventureDefinition,
    campaign: &CampaignState,
) -> Result<Option<&'a EncounterDefinition>, GameRuntimeError> {
    for candidate in &adventure.encounters {
        if campaign
            .completed_encounters
            .iter()
            .any(|completed| completed.encounter_id == candidate.as_str())
        {
            continue;
        }
        let encounter = rules.encounter(candidate).ok_or_else(|| {
            GameRuntimeError::InvalidState(format!("encounter definition {candidate} is missing"))
        })?;
        if !encounter.available_from_camp {
            return Err(GameRuntimeError::InvalidState(format!(
                "next authored encounter {candidate} is not available from camp"
            )));
        }
        return Ok(Some(encounter));
    }
    Ok(None)
}
