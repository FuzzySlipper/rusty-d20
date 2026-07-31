use super::*;

impl GameRuntime {
    pub(super) fn project_campaign(
        &self,
        campaign: &CampaignState,
        session: &D20Session,
    ) -> Result<CampaignDto, GameRuntimeError> {
        let adventure = self.adventure()?;
        let encounter = current_encounter_definition(&self.rules, adventure, campaign)?;
        let available_encounters = if campaign.phase == CampaignPhase::Camp {
            next_available_encounter_definition(&self.rules, adventure, campaign)?
                .map(|encounter| {
                    vec![EncounterChoiceDto {
                        id: encounter.id.to_string(),
                        title: encounter.title.clone(),
                        summary: encounter.summary.clone(),
                    }]
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let completed_encounters = campaign
            .completed_encounters
            .iter()
            .map(|completed| {
                let id = D20Id::parse(&completed.encounter_id).map_err(|error| {
                    GameRuntimeError::InvalidState(format!(
                        "completed encounter identity is invalid: {error}"
                    ))
                })?;
                let encounter = self.rules.encounter(&id).ok_or_else(|| {
                    GameRuntimeError::InvalidState(format!(
                        "completed encounter {} is missing",
                        completed.encounter_id
                    ))
                })?;
                Ok(CompletedEncounterDto {
                    encounter_id: completed.encounter_id.clone(),
                    title: encounter.title.clone(),
                    outcome: match completed.outcome {
                        EncounterOutcome::Victory => EncounterOutcomeKindDto::Victory,
                        EncounterOutcome::Defeat => EncounterOutcomeKindDto::Defeat,
                    },
                })
            })
            .collect::<Result<Vec<_>, GameRuntimeError>>()?;
        Ok(CampaignDto {
            id: adventure.id.to_string(),
            title: adventure.title.clone(),
            phase: match campaign.phase {
                CampaignPhase::Camp => CampaignPhaseDto::Camp,
                CampaignPhase::Exploration => CampaignPhaseDto::Exploration,
                CampaignPhase::Encounter => CampaignPhaseDto::Encounter,
                CampaignPhase::Outcome => CampaignPhaseDto::Outcome,
                CampaignPhase::AdventureComplete => CampaignPhaseDto::AdventureComplete,
            },
            party: adventure
                .party
                .iter()
                .map(|member| {
                    let character = self
                        .rules
                        .character_template(member)
                        .expect("compiled party member exists");
                    Ok(PartyMemberDto {
                        character: self.project_character(session, character)?,
                        loadout: self
                            .project_loadout(session, EntityId::new(character.entity_id))?,
                    })
                })
                .collect::<Result<Vec<_>, GameRuntimeError>>()?,
            active_encounter_id: campaign.active_encounter_id.clone(),
            available_encounters,
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
            completed_encounters,
            completion: if campaign.phase == CampaignPhase::AdventureComplete {
                campaign.outcome.map(|outcome| AdventureCompletionDto {
                    kind: match outcome {
                        EncounterOutcome::Victory => EncounterOutcomeKindDto::Victory,
                        EncounterOutcome::Defeat => EncounterOutcomeKindDto::Defeat,
                    },
                    source: adventure.completion.source.clone(),
                    title: match outcome {
                        EncounterOutcome::Victory => adventure.completion.victory_title.clone(),
                        EncounterOutcome::Defeat => adventure.completion.defeat_title.clone(),
                    },
                    text: match outcome {
                        EncounterOutcome::Victory => adventure.completion.victory_text.clone(),
                        EncounterOutcome::Defeat => adventure.completion.defeat_text.clone(),
                    },
                    details: adventure.completion.details.clone(),
                })
            } else {
                None
            },
        })
    }

    pub(super) fn project_loadout(
        &self,
        session: &D20Session,
        owner: EntityId,
    ) -> Result<LoadoutDto, GameRuntimeError> {
        let adventure = self.adventure()?;
        party_member_name(&self.rules, adventure, owner)?;
        let stash = storage_entity(&self.rules, adventure, &adventure.camp_storage)?;
        let inventory = session.inventory_view(owner)?;
        let equipment = session
            .entities()
            .component::<EquipmentComponent>(owner)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState("party equipment component is missing".to_owned())
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
            .chain(
                self.rules
                    .implements()
                    .map(|implement| implement.slot.clone()),
            )
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
        let stash_capacity = stash_view
            .capacity()
            .iter()
            .find(|usage| usage.metric.as_str() == "carried-items")
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "camp stash carried-items capacity is missing".to_owned(),
                )
            })?;
        let stash_maximum = stash_capacity.maximum.ok_or_else(|| {
            GameRuntimeError::InvalidState("camp stash carried-items maximum is missing".to_owned())
        })?;
        let defenses = self.project_defenses(session, owner)?;
        Ok(LoadoutDto {
            owner_id: owner.raw(),
            stash_owner_id: stash.raw(),
            inventory_slots,
            equipment_slots,
            stash_items,
            stash_capacity: LoadoutCapacityDto {
                metric: "carried-items".to_owned(),
                used: stash_capacity.used,
                maximum: stash_maximum,
            },
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
        let (definition_id, slot) = self
            .rules
            .equipment_definition(&authored.equipment)
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "loadout equipment {} is missing",
                    authored.equipment.id()
                ))
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
        if component.definition() != &authored.equipment.mechanics_item_id() {
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
            definition_id: definition_id.to_string(),
            name: item_name,
            icon: authored.icon.clone(),
            rarity,
            quantity: 1,
            equipment_slot_id: slot.to_string(),
            equipped_slot_id,
        })
    }

    fn project_defenses(
        &self,
        session: &D20Session,
        owner: EntityId,
    ) -> Result<Vec<DefenseReadoutDto>, GameRuntimeError> {
        self.rules
            .defenses()
            .map(|definition| {
                let defense = StatService::evaluate(
                    session.entities(),
                    self.rules.mechanics(),
                    owner,
                    &defense_stat_id(&definition.id),
                    &operation(&format!(
                        "project-character-{}-defense-{}",
                        owner.raw(),
                        definition.id
                    ))?,
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
            .collect()
    }

    fn project_action(&self, action: &ActionDefinition) -> Result<ActionDto, GameRuntimeError> {
        let (ability, defense, damage, range, implement) = match &action.attack {
            ActionAttackDefinition::Fixed {
                ability,
                defense,
                damage,
                range,
            } => (ability, defense, damage, *range, None),
            ActionAttackDefinition::Implement { implement } => {
                let definition = self.rules.implement(implement).ok_or_else(|| {
                    GameRuntimeError::InvalidState(format!(
                        "action {} implement {implement} is missing",
                        action.id
                    ))
                })?;
                (
                    &definition.ability,
                    &definition.defense,
                    &definition.damage,
                    definition.range,
                    Some(humanize(definition.id.as_str())),
                )
            }
        };
        Ok(ActionDto {
            id: action.id.to_string(),
            label: humanize(action.id.as_str()),
            ability: humanize(ability.as_str()),
            defense: humanize(defense.as_str()),
            damage: format!(
                "{}d{}{}{} {}",
                damage.dice,
                damage.sides,
                if damage.bonus >= 0 { "+" } else { "" },
                damage.bonus,
                humanize(damage.kind.as_str())
            ),
            activation: action
                .activation_costs
                .iter()
                .map(|cost| format!("{} {}", cost.amount, humanize(cost.budget.as_str())))
                .collect(),
            target: format!(
                "{} {} {} · line of effect {}",
                action.target.maximum_targets,
                humanize(&format!("{:?}", action.target.team).to_lowercase()),
                humanize(&format!("{:?}", action.target.kind).to_lowercase()),
                humanize(&format!("{:?}", action.target.line_of_effect).to_lowercase())
            ),
            range,
            implement,
            tags: action
                .tags
                .iter()
                .map(|tag| humanize(tag.as_str()))
                .collect(),
            effect: action
                .effect
                .as_ref()
                .map(|effect| humanize(effect.as_str())),
            forced_movement: action.forced_movement,
        })
    }

    pub(super) fn project_encounter(
        &self,
        campaign: &CampaignState,
        session: &D20Session,
    ) -> Result<EncounterDto, GameRuntimeError> {
        let participants = self
            .ordered_participants()?
            .into_iter()
            .map(|(entity, faction, initiative)| {
                let position = session
                    .encounter_participation(entity)?
                    .ok_or_else(|| {
                        GameRuntimeError::InvalidState(format!(
                            "participant {} has no canonical encounter facts",
                            entity.raw()
                        ))
                    })?
                    .position();
                let character = self
                    .rules
                    .character_templates()
                    .find(|character| character.entity_id == entity.raw())
                    .ok_or_else(|| {
                        GameRuntimeError::InvalidState(format!(
                            "participant {} has no compiled character",
                            entity.raw()
                        ))
                    })?;
                Ok(EncounterParticipantDto {
                    character: self.project_character(session, character)?,
                    faction: match faction {
                        EncounterFaction::Party => EncounterFactionDto::Party,
                        EncounterFaction::Opposition => EncounterFactionDto::Opposition,
                    },
                    initiative,
                    defeated: self.vitality(entity)? == 0,
                    x: position.x(),
                    y: position.y(),
                })
            })
            .collect::<Result<Vec<_>, GameRuntimeError>>()?;
        let current_actor = campaign.current_actor_id.map(EntityId::new);
        let current_is_party = match current_actor {
            Some(actor) => {
                session
                    .encounter_participation(actor)?
                    .ok_or_else(|| {
                        GameRuntimeError::InvalidState(format!(
                            "current actor {} is not an encounter participant",
                            actor.raw()
                        ))
                    })?
                    .faction()
                    == EncounterFaction::Party
            }
            None => false,
        };
        let target_ids = if current_is_party {
            participants
                .iter()
                .filter(|participant| !participant.defeated)
                .map(|participant| participant.character.id)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut legal_targets = Vec::new();
        let mut actions = Vec::new();
        if let Some(actor) = current_actor.filter(|_| current_is_party) {
            let actor_definition = self
                .rules
                .character_templates()
                .find(|character| character.entity_id == actor.raw())
                .expect("canonical current actor has a compiled character");
            for action_id in &actor_definition.actions {
                let action = self
                    .rules
                    .action(action_id)
                    .expect("compiled character action exists");
                let admitted_targets = target_ids
                    .iter()
                    .copied()
                    .filter_map(|target| {
                        match self.action_target_team_is_legal(actor, EntityId::new(target), action)
                        {
                            Ok(true) => {}
                            Ok(false) => return None,
                            Err(error) => return Some(Err(error)),
                        }
                        match self.action_is_spatially_legal(actor, EntityId::new(target), action) {
                            Ok(true) => {}
                            Ok(false) => return None,
                            Err(error) => return Some(Err(error)),
                        }
                        match session.preview_action(
                            actor,
                            EntityId::new(target),
                            action_id,
                            operation(&format!("project-action-{}-{target}", action.id))
                                .expect("compiled action identity forms an operation"),
                        ) {
                            Ok(_) => Some(Ok(target)),
                            Err(error) if is_unavailable_action_error(&error) => None,
                            Err(error) => Some(Err(GameRuntimeError::Session(error))),
                        }
                    })
                    .collect::<Result<Vec<_>, GameRuntimeError>>()?;
                if admitted_targets.is_empty() {
                    continue;
                }
                legal_targets.push(ActionTargetsDto {
                    action_id: action.id.to_string(),
                    target_ids: admitted_targets,
                });
                actions.push(self.project_action(action)?);
            }
        }
        let encounter = current_encounter_definition(&self.rules, self.adventure()?, campaign)?;
        let legal_moves = if let Some(actor) =
            current_actor.filter(|_| current_is_party && self.pending.is_none())
        {
            self.legal_tactical_routes(actor)?
                .into_iter()
                .map(|route| TacticalMoveDto {
                    x: route.destination.x(),
                    y: route.destination.y(),
                    cost: u16::try_from(route.path.len().saturating_sub(1))
                        .expect("compiled tactical route length fits u16"),
                    route: route
                        .path
                        .into_iter()
                        .map(|position| TacticalCellDto {
                            x: position.x(),
                            y: position.y(),
                        })
                        .collect(),
                })
                .collect()
        } else {
            Vec::new()
        };
        Ok(EncounterDto {
            round: session.current_turn(),
            current_actor_id: campaign.current_actor_id,
            board: TacticalBoardDto {
                width: encounter.board.width,
                height: encounter.board.height,
                rows: encounter.board.rows.clone(),
                legal_moves,
            },
            participants,
            actions,
            legal_targets,
            reaction_prompt: self
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
        let ability_scores = session
            .entities()
            .component::<AbilityScoresComponent>(entity)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState("ability scores component is missing".to_owned())
            })?;
        let abilities = ability_scores
            .scores()
            .iter()
            .map(|ability| AbilityReadoutDto {
                id: ability.id().to_string(),
                label: humanize(ability.id().as_str()),
                score: ability.score(),
                modifier: crate::ability_modifier(ability.score()),
            })
            .collect();
        let resource_state = session
            .entities()
            .component::<ActionResourcesComponent>(entity)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState("resources component is missing".to_owned())
            })?;
        let resources = resource_state
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
        let defenses = self.project_defenses(session, entity)?;
        let features = definition
            .features
            .iter()
            .map(|feature_id| {
                let feature = self.rules.feature(feature_id).ok_or_else(|| {
                    GameRuntimeError::InvalidState(format!(
                        "character feature {feature_id} is missing"
                    ))
                })?;
                Ok(FeatureReadoutDto {
                    id: feature.id.to_string(),
                    label: feature.label.clone(),
                    description: feature.description.clone(),
                })
            })
            .collect::<Result<Vec<_>, GameRuntimeError>>()?;
        let actions = definition
            .actions
            .iter()
            .map(|action_id| {
                let action = self.rules.action(action_id).ok_or_else(|| {
                    GameRuntimeError::InvalidState(format!(
                        "character action {action_id} is missing"
                    ))
                })?;
                self.project_action(action)
            })
            .collect::<Result<Vec<_>, GameRuntimeError>>()?;
        let reactions = definition
            .reactions
            .iter()
            .map(|reaction_id| {
                let reaction = self.rules.reaction(reaction_id).ok_or_else(|| {
                    GameRuntimeError::InvalidState(format!(
                        "character reaction {reaction_id} is missing"
                    ))
                })?;
                Ok(CharacterReactionDto {
                    id: reaction.id.to_string(),
                    label: humanize(reaction.id.as_str()),
                    defense: humanize(reaction.defense.as_str()),
                    bonus: reaction.bonus,
                    resource: humanize(reaction.resource.as_str()),
                    cost: reaction.cost,
                    available: resource_state.current(&reaction.resource).ok_or_else(|| {
                        GameRuntimeError::InvalidState(format!(
                            "character reaction {} resource {} is missing",
                            reaction.id, reaction.resource
                        ))
                    })?,
                    activation: reaction
                        .activation_costs
                        .iter()
                        .map(|cost| format!("{} {}", cost.amount, humanize(cost.budget.as_str())))
                        .collect(),
                    effect: humanize(reaction.effect.as_str()),
                })
            })
            .collect::<Result<Vec<_>, GameRuntimeError>>()?;
        let affinities = definition
            .affinities
            .iter()
            .map(|affinity| AffinityReadoutDto {
                damage_type: affinity.damage_type.to_string(),
                label: humanize(affinity.damage_type.as_str()),
                affinity: match affinity.affinity {
                    CharacterAffinityKindDefinition::Resistant => "resistant".to_owned(),
                    CharacterAffinityKindDefinition::Vulnerable => "vulnerable".to_owned(),
                },
            })
            .collect();
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
            experience: definition.experience,
            health_current: vitality.current().get(),
            health_maximum: i64::from(definition.vitality),
            abilities,
            defenses,
            resources,
            effects,
            features,
            actions,
            reactions,
            affinities,
        })
    }

    pub(super) fn project_pending(&self, pending: &PendingAction) -> ReactionPromptDto {
        ReactionPromptDto {
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
