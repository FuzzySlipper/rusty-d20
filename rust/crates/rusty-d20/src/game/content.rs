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

pub(super) fn product_equipment_items(
    rules: &D20Ruleset,
    adventure: &AdventureDefinition,
) -> Result<Vec<EquipmentItemSeed>, GameRuntimeError> {
    adventure
        .items
        .iter()
        .map(|item| {
            let definition = rules.item_instance(item).ok_or_else(|| {
                GameRuntimeError::InvalidState(format!("item instance {item} is missing"))
            })?;
            Ok(EquipmentItemSeed {
                entity: EntityId::new(definition.entity_id),
                owner: owner_entity(rules, adventure, &definition.owner)?,
                name: definition.name.clone(),
                equipment: definition.equipment.clone(),
            })
        })
        .collect()
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
        if actual.definition() != &definition.equipment.mechanics_item_id() {
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
        let stash = storage_entity(rules, adventure, &adventure.camp_storage)?;
        let party = adventure
            .party
            .iter()
            .map(|member| character_entity(rules, adventure, member))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let victory = campaign.completed_encounters.iter().any(|completed| {
            completed.encounter_id == encounter.id.as_str()
                && completed.outcome == EncounterOutcome::Victory
        });
        let reward_is_claimed = matches!(
            reward_owner,
            Some(owner) if owner == stash || party.contains(&owner)
        ) && original_equipment
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
    let adventure_party_alive = adventure
        .party
        .iter()
        .map(|member| character_entity(rules, adventure, member))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entity| saved_vitality(session, entity))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|vitality| *vitality > 0)
        .count();
    let participation = session.encounter_participants()?;
    if matches!(
        campaign.phase,
        CampaignPhase::Camp | CampaignPhase::Exploration
    ) {
        if campaign.current_actor_id.is_some()
            || !participation.is_empty()
            || adventure_party_alive == 0
        {
            return Err(GameRuntimeError::InvalidSave(
                "camp/exploration phase contradicts canonical party or participation state"
                    .to_owned(),
            ));
        }
        return Ok(());
    }
    let encounter = current_encounter_definition(rules, adventure, campaign)?;
    let initiative_ability = id("finesse")?;
    let expected = encounter
        .roster
        .iter()
        .map(|participant| {
            let character = rules
                .character_template(&participant.character)
                .expect("compiled encounter character exists");
            Ok((
                EntityId::new(character.entity_id),
                (
                    match participant.faction {
                        EncounterFactionDefinition::Party => EncounterFaction::Party,
                        EncounterFactionDefinition::Opposition => EncounterFaction::Opposition,
                    },
                    *character
                        .abilities
                        .get(&initiative_ability)
                        .ok_or_else(|| {
                            GameRuntimeError::InvalidSave(format!(
                                "participant {} has no finesse initiative",
                                character.id
                            ))
                        })?,
                ),
            ))
        })
        .collect::<Result<BTreeMap<_, _>, GameRuntimeError>>()?;
    let actual = participation
        .iter()
        .filter(|(_, component)| component.encounter() == &encounter.id)
        .map(|(entity, component)| (*entity, (component.faction(), component.initiative())))
        .collect::<BTreeMap<_, _>>();
    if actual != expected || participation.len() != expected.len() {
        return Err(GameRuntimeError::InvalidSave(
            "encounter roster does not match canonical participation facts".to_owned(),
        ));
    }
    let positions = participation
        .iter()
        .map(|(_, component)| component.position())
        .collect::<BTreeSet<_>>();
    if positions.len() != participation.len()
        || positions.iter().any(|position| {
            !encounter.board.is_floor(crate::TacticalPositionDefinition {
                x: position.x(),
                y: position.y(),
            })
        })
    {
        return Err(GameRuntimeError::InvalidSave(
            "encounter positions overlap or contradict the authored tactical board".to_owned(),
        ));
    }
    let party_alive = actual
        .iter()
        .filter(|(_, (faction, _))| *faction == EncounterFaction::Party)
        .map(|(entity, _)| saved_vitality(session, *entity))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|vitality| *vitality > 0)
        .count();
    let opposition_alive = actual
        .iter()
        .filter(|(_, (faction, _))| *faction == EncounterFaction::Opposition)
        .map(|(entity, _)| saved_vitality(session, *entity))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|vitality| *vitality > 0)
        .count();
    let valid = match (campaign.phase, campaign.outcome) {
        (CampaignPhase::Encounter, None) => {
            campaign.current_actor_id.is_some_and(|actor| {
                actual.contains_key(&EntityId::new(actor))
                    && saved_vitality(session, EntityId::new(actor)).is_ok_and(|value| value > 0)
            }) && party_alive > 0
                && opposition_alive > 0
        }
        (CampaignPhase::Outcome, Some(EncounterOutcome::Victory)) => {
            campaign.current_actor_id.is_none() && party_alive > 0 && opposition_alive == 0
        }
        (CampaignPhase::Outcome, Some(EncounterOutcome::Defeat)) => {
            campaign.current_actor_id.is_none() && party_alive == 0 && opposition_alive > 0
        }
        _ => false,
    };
    if !valid {
        return Err(GameRuntimeError::InvalidSave(format!(
            "campaign phase/outcome contradicts canonical factions: party alive={party_alive}, opposition alive={opposition_alive}"
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
    session.unequip_item(
        owner,
        item,
        operation(&format!("reward-unequip-{unequip_serial}"))?,
    )?;
    *next_operation = next_operation
        .checked_add(1)
        .ok_or(GameRuntimeError::CounterOverflow)?;
    let transfer_serial = *next_operation;
    session.transfer_item(
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
            session.equip_item(
                owner,
                item_entity,
                &definition.equipment,
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
