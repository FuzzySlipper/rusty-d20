use super::*;

impl GameRuntime {
    pub(super) fn project_campaign(
        &self,
        campaign: &CampaignState,
        session: &D20Session,
    ) -> Result<CampaignDto, GameRuntimeError> {
        let adventure = self.adventure()?;
        let hero = self
            .rules
            .character_template(&adventure.hero)
            .expect("compiled hero exists");
        let encounter = current_encounter_definition(&self.rules, adventure, campaign)?;
        Ok(CampaignDto {
            id: adventure.id.to_string(),
            title: adventure.title.clone(),
            phase: match campaign.phase {
                CampaignPhase::Camp => CampaignPhaseDto::Camp,
                CampaignPhase::Encounter => CampaignPhaseDto::Encounter,
                CampaignPhase::Outcome => CampaignPhaseDto::Outcome,
            },
            hero: self.project_character(session, hero)?,
            loadout: self.project_loadout(session)?,
            active_encounter_id: campaign.active_encounter_id.clone(),
            available_encounters: if campaign.outcome.is_none() {
                adventure
                    .encounters
                    .iter()
                    .filter_map(|id| self.rules.encounter(id))
                    .filter(|encounter| encounter.available_from_camp)
                    .map(|encounter| EncounterChoiceDto {
                        id: encounter.id.to_string(),
                        title: encounter.title.clone(),
                        summary: encounter.summary.clone(),
                    })
                    .collect()
            } else {
                Vec::new()
            },
            latest_outcome: campaign.outcome.map(|outcome| match outcome {
                EncounterOutcome::Victory => CampaignOutcomeDto {
                    kind: EncounterOutcomeKindDto::Victory,
                    encounter_id: encounter.id.to_string(),
                    title: encounter.victory.title.clone(),
                    summary: encounter.victory.summary.clone(),
                    reward_item_id: encounter
                        .victory
                        .reward_item
                        .as_ref()
                        .and_then(|item| self.rules.item_instance(item))
                        .map(|item| item.entity_id),
                    reward: encounter.victory.reward_label.clone(),
                },
                EncounterOutcome::Defeat => CampaignOutcomeDto {
                    kind: EncounterOutcomeKindDto::Defeat,
                    encounter_id: encounter.id.to_string(),
                    title: encounter.defeat.title.clone(),
                    summary: encounter.defeat.summary.clone(),
                    reward_item_id: None,
                    reward: None,
                },
            }),
        })
    }

    pub(super) fn project_loadout(
        &self,
        session: &D20Session,
    ) -> Result<LoadoutDto, GameRuntimeError> {
        let adventure = self.adventure()?;
        let hero = character_entity(&self.rules, adventure, &adventure.hero)?;
        let stash = storage_entity(&self.rules, adventure, &adventure.camp_storage)?;
        let inventory = session.inventory_view(hero)?;
        let equipment = session
            .entities()
            .component::<EquipmentComponent>(hero)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState("player equipment component is missing".to_owned())
            })?;
        let equipped_by_item = equipment
            .assignments()
            .iter()
            .map(|assignment| (assignment.item, assignment.slot.to_string()))
            .collect::<BTreeMap<_, _>>();
        let mut inventory_items = inventory
            .unique_items()
            .iter()
            .map(|item| {
                self.project_loadout_item(
                    session,
                    item.entity,
                    equipped_by_item.get(&item.entity).cloned(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let capacity = inventory
            .capacity()
            .iter()
            .find(|usage| usage.metric.as_str() == "carried-items")
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "player carried-items capacity is missing".to_owned(),
                )
            })?;
        let maximum = capacity.maximum.ok_or_else(|| {
            GameRuntimeError::InvalidState("player carried-items maximum is missing".to_owned())
        })?;
        let slot_count = usize::try_from(maximum).map_err(|_| {
            GameRuntimeError::InvalidState(
                "player inventory capacity does not fit memory".to_owned(),
            )
        })?;
        if inventory_items.len() > slot_count {
            return Err(GameRuntimeError::InvalidState(
                "player inventory exceeds its projected slot count".to_owned(),
            ));
        }
        let mut inventory_slots = inventory_items
            .drain(..)
            .map(Some)
            .collect::<Vec<Option<LoadoutItemDto>>>();
        inventory_slots.resize(slot_count, None);

        let mut slot_items = BTreeMap::new();
        for assignment in equipment.assignments() {
            slot_items.insert(
                assignment.slot.to_string(),
                self.project_loadout_item(
                    session,
                    assignment.item,
                    Some(assignment.slot.to_string()),
                )?,
            );
        }
        let equipment_slots = self
            .rules
            .armors()
            .map(|armor| armor.slot.clone())
            .fold(BTreeMap::<String, D20Id>::new(), |mut slots, slot| {
                slots.entry(slot.to_string()).or_insert(slot);
                slots
            })
            .into_iter()
            .map(|(slot, definition)| EquipmentSlotDto {
                id: slot.clone(),
                label: humanize(definition.as_str()),
                equipped: slot_items.remove(&slot),
            })
            .collect::<Vec<_>>();

        let stash_view = session.inventory_view(stash)?;
        let stash_items = stash_view
            .unique_items()
            .iter()
            .map(|item| self.project_loadout_item(session, item.entity, None))
            .collect::<Result<Vec<_>, _>>()?;
        let defenses = self
            .rules
            .defenses()
            .map(|definition| {
                let defense = StatService::evaluate(
                    session.entities(),
                    self.rules.mechanics(),
                    hero,
                    &defense_stat_id(&definition.id),
                    &operation(&format!("project-loadout-{}", definition.id))?,
                    &[],
                )
                .map_err(D20SessionError::from)?;
                Ok(DefenseReadoutDto {
                    id: definition.id.to_string(),
                    label: humanize(definition.id.as_str()),
                    value: defense.value.get(),
                    sources: defense
                        .decisions
                        .iter()
                        .map(|decision| {
                            format!(
                                "{}: {} ({})",
                                source_label(&decision.source),
                                stat_contribution_label(decision.contribution.as_ref()),
                                outcome_label(decision.outcome)
                            )
                        })
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, GameRuntimeError>>()?;
        Ok(LoadoutDto {
            owner_id: hero.raw(),
            stash_owner_id: stash.raw(),
            inventory_slots,
            equipment_slots,
            stash_items,
            capacity: LoadoutCapacityDto {
                metric: "carried-items".to_owned(),
                used: capacity.used,
                maximum,
            },
            defenses,
        })
    }

    pub(super) fn project_loadout_item(
        &self,
        session: &D20Session,
        item: EntityId,
        equipped_slot_id: Option<String>,
    ) -> Result<LoadoutItemDto, GameRuntimeError> {
        let adventure = self.adventure()?;
        let authored = product_loadout_item(&self.rules, adventure, item)?;
        let armor = authored.armor.clone();
        let definition = self.rules.armor(&armor).ok_or_else(|| {
            GameRuntimeError::InvalidState(format!("loadout armor {armor} is missing"))
        })?;
        let component = session
            .entities()
            .component::<ItemComponent>(item)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "loadout item {} is missing ItemComponent",
                    item.raw()
                ))
            })?;
        let expected_definition = format!("armor.{armor}");
        if component.definition().as_str() != expected_definition {
            return Err(GameRuntimeError::InvalidState(format!(
                "loadout item {} definition is inconsistent",
                item.raw()
            )));
        }
        let item_name = session
            .entities()
            .core(item)
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "loadout item {} core facts are missing",
                    item.raw()
                ))
            })?
            .name
            .clone();
        if item_name != authored.name {
            return Err(GameRuntimeError::InvalidState(format!(
                "loadout item {} authored name is inconsistent",
                item.raw()
            )));
        }
        let rarity = match authored.rarity {
            ItemRarityDefinition::Common => LoadoutRarityDto::Common,
            ItemRarityDefinition::Uncommon => LoadoutRarityDto::Uncommon,
            ItemRarityDefinition::Rare => LoadoutRarityDto::Rare,
            ItemRarityDefinition::Epic => LoadoutRarityDto::Epic,
        };
        Ok(LoadoutItemDto {
            entity_id: item.raw(),
            definition_id: armor.to_string(),
            name: item_name,
            icon: authored.icon.clone(),
            rarity,
            quantity: 1,
            equipment_slot_id: definition.slot.to_string(),
            equipped_slot_id,
        })
    }

    pub(super) fn project_encounter(
        &self,
        campaign: &CampaignState,
        session: &D20Session,
    ) -> Result<EncounterDto, GameRuntimeError> {
        let adventure = self.adventure()?;
        let encounter = current_encounter_definition(&self.rules, adventure, campaign)?;
        let hero = self
            .rules
            .character_template(&adventure.hero)
            .expect("compiled hero exists");
        let opponent = self
            .rules
            .character_template(&encounter.opponent)
            .expect("compiled opponent exists");
        Ok(EncounterDto {
            turn: session.current_turn(),
            next_roll: session.next_roll_index(),
            player_id: hero.entity_id,
            turn_owner: campaign.turn_owner.map(|owner| match owner {
                EncounterTurnOwner::Player => EncounterTurnOwnerDto::Player,
                EncounterTurnOwner::Opposition => EncounterTurnOwnerDto::Opposition,
            }),
            characters: vec![
                self.project_character(session, hero)?,
                self.project_character(session, opponent)?,
            ],
            actions: hero
                .actions
                .iter()
                .filter_map(|action| self.rules.action(action))
                .map(|action| ActionDto {
                    id: action.id.to_string(),
                    label: humanize(action.id.as_str()),
                    ability: humanize(action.ability.as_str()),
                    defense: humanize(action.defense.as_str()),
                    damage: format!(
                        "{}d{}{}{} {}",
                        action.damage.dice,
                        action.damage.sides,
                        if action.damage.bonus >= 0 { "+" } else { "" },
                        action.damage.bonus,
                        humanize(action.damage.kind.as_str())
                    ),
                    effect: action
                        .effect
                        .as_ref()
                        .map(|effect| humanize(effect.as_str())),
                })
                .collect(),
            pending_action: self
                .pending
                .as_ref()
                .map(|pending| self.project_pending(pending)),
            log: self.log.clone(),
        })
    }

    pub(super) fn project_character(
        &self,
        session: &D20Session,
        definition: &CharacterTemplateDefinition,
    ) -> Result<CharacterDto, GameRuntimeError> {
        let entity = EntityId::new(definition.entity_id);
        let core = session.entities().core(entity).ok_or_else(|| {
            GameRuntimeError::InvalidState("character entity is missing".to_owned())
        })?;
        let tracks = session
            .entities()
            .component::<TracksComponent>(entity)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState("vitality component is missing".to_owned())
            })?;
        let vitality = tracks
            .values()
            .iter()
            .find(|value| value.track().as_str() == "vitality")
            .ok_or_else(|| {
                GameRuntimeError::InvalidState("vitality track is missing".to_owned())
            })?;
        let resources = session
            .entities()
            .component::<ActionResourcesComponent>(entity)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState("resources component is missing".to_owned())
            })?
            .resources()
            .iter()
            .map(|resource| {
                let maximum = self
                    .rules
                    .resource(resource.id())
                    .ok_or_else(|| {
                        GameRuntimeError::InvalidState(format!(
                            "resource definition {} is missing",
                            resource.id()
                        ))
                    })?
                    .maximum;
                Ok(ResourceDto {
                    id: resource.id().to_string(),
                    label: humanize(resource.id().as_str()),
                    current: resource.current(),
                    maximum,
                })
            })
            .collect::<Result<Vec<_>, GameRuntimeError>>()?;
        let effects: Vec<String> = session
            .entities()
            .component::<ScheduledEffectsComponent>(entity)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState("scheduled effects component is missing".to_owned())
            })?
            .effects()
            .iter()
            .map(|effect| {
                format!(
                    "{} · through turn {}",
                    humanize(effect.definition().as_str()),
                    effect.expires_at_turn()
                )
            })
            .collect();
        let active_count = session
            .entities()
            .component::<ActiveEffectsComponent>(entity)?
            .map_or(0, |active| active.effects().len());
        if active_count != effects.len() {
            return Err(GameRuntimeError::InvalidState(
                "active and scheduled effects diverged".to_owned(),
            ));
        }
        Ok(CharacterDto {
            id: entity.raw(),
            name: core.name.clone(),
            title: definition.title.clone(),
            level: definition.level,
            health_current: vitality.current().get(),
            health_maximum: i64::from(definition.vitality),
            resources,
            effects,
        })
    }

    pub(super) fn project_pending(&self, pending: &PendingAction) -> PendingActionDto {
        PendingActionDto {
            token: pending.token.clone(),
            actor_id: pending.preview.actor().raw(),
            target_id: pending.preview.target().raw(),
            action_id: pending.preview.action().to_string(),
            action_label: humanize(pending.preview.action().as_str()),
            ability_score: pending.preview.ability_score(),
            ability_modifier: pending.preview.ability_modifier(),
            defense: pending.preview.defense().value.get(),
            defense_sources: pending
                .preview
                .defense()
                .decisions
                .iter()
                .map(|decision| {
                    format!(
                        "{}: {} ({})",
                        source_label(&decision.source),
                        stat_contribution_label(decision.contribution.as_ref()),
                        outcome_label(decision.outcome)
                    )
                })
                .collect(),
            reactions: pending
                .preview
                .reactions()
                .iter()
                .map(|reaction| ReactionDto {
                    id: reaction.reaction().to_string(),
                    label: humanize(reaction.reaction().as_str()),
                    resource: humanize(reaction.resource().as_str()),
                    cost: reaction.cost(),
                    available: reaction.available(),
                    bonus: reaction.bonus(),
                    effect: humanize(reaction.effect().as_str()),
                })
                .collect(),
        }
    }
}
