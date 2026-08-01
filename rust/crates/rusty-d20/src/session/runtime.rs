use super::*;

#[derive(Debug, Clone)]
pub struct D20Session {
    pub(super) rules: D20Ruleset,
    pub(super) entities: EntityState,
    pub(super) roll_source: RollSourceConfig,
    pub(super) next_roll: u64,
    pub(super) current_turn: u64,
}

impl D20Session {
    pub fn new(
        rules: D20Ruleset,
        seed: RngSeed,
        characters: Vec<CharacterSeed>,
        armor_items: Vec<ArmorItemSeed>,
    ) -> Result<Self, D20SessionError> {
        Self::new_with_loadout(rules, seed, characters, vec![], vec![], armor_items)
    }

    pub fn new_with_loadout(
        rules: D20Ruleset,
        seed: RngSeed,
        characters: Vec<CharacterSeed>,
        inventories: Vec<InventorySeed>,
        storage: Vec<StorageSeed>,
        armor_items: Vec<ArmorItemSeed>,
    ) -> Result<Self, D20SessionError> {
        Self::new_with_equipment_loadout(
            rules,
            seed,
            characters,
            inventories,
            storage,
            armor_items
                .into_iter()
                .map(EquipmentItemSeed::from)
                .collect(),
        )
    }

    pub fn new_with_equipment_loadout(
        rules: D20Ruleset,
        seed: RngSeed,
        characters: Vec<CharacterSeed>,
        inventories: Vec<InventorySeed>,
        storage: Vec<StorageSeed>,
        equipment_items: Vec<EquipmentItemSeed>,
    ) -> Result<Self, D20SessionError> {
        Self::new_with_roll_source(
            rules,
            RollSourceConfig::seeded(seed.raw()),
            characters,
            inventories,
            storage,
            equipment_items,
        )
    }

    pub fn new_with_roll_source(
        rules: D20Ruleset,
        roll_source: RollSourceConfig,
        characters: Vec<CharacterSeed>,
        inventories: Vec<InventorySeed>,
        storage: Vec<StorageSeed>,
        equipment_items: Vec<EquipmentItemSeed>,
    ) -> Result<Self, D20SessionError> {
        roll_source.validate()?;
        let mut definitions = characters
            .iter()
            .map(|character| EntityDefinition::new(character.entity, character.name.clone()))
            .collect::<Vec<_>>();
        definitions.extend(
            storage
                .iter()
                .map(|storage| EntityDefinition::new(storage.entity, storage.name.clone())),
        );
        definitions.extend(equipment_items.iter().map(|item| {
            EntityDefinition::new(item.entity, item.name.clone()).with_containment(item.owner)
        }));
        let registry = d20_component_registry()?;
        let mut entities = EntityState::from_definitions_with_registry(registry, definitions)?;

        for character in &characters {
            validate_character_seed(&rules, character)?;
            attach(
                &mut entities,
                character.entity,
                AbilityScoresComponent::new(character.abilities.clone())?,
            )?;
            attach(
                &mut entities,
                character.entity,
                ActionResourcesComponent::new(character.resources.clone())?,
            )?;
            attach(
                &mut entities,
                character.entity,
                ActivationBudgetsComponent::new(
                    rules
                        .activation_budgets()
                        .map(|budget| ActivationBudget::new(budget.id.clone(), budget.initial))
                        .collect(),
                )?,
            )?;
            attach(
                &mut entities,
                character.entity,
                ScheduledEffectsComponent::new(vec![])?,
            )?;

            let stats = rules
                .defenses()
                .map(|defense| {
                    let score = defense
                        .abilities
                        .iter()
                        .filter_map(|ability| {
                            character
                                .abilities
                                .iter()
                                .find(|score| score.id() == ability)
                                .map(AbilityScore::score)
                        })
                        .max()
                        .expect("character seed validation requires every defense ability");
                    StatValue::new(
                        defense_stat_id(&defense.id),
                        scalar(i64::from(defense.base + ability_modifier(score))),
                    )
                })
                .collect();
            attach(
                &mut entities,
                character.entity,
                StatsComponent::new(rules.mechanics().version().clone(), stats)?,
            )?;
            attach(
                &mut entities,
                character.entity,
                TracksComponent::new(
                    rules.mechanics().version().clone(),
                    vec![TrackValue::new(
                        vitality_track_id(),
                        scalar(i64::from(character.vitality)),
                    )],
                )?,
            )?;
            attach(
                &mut entities,
                character.entity,
                IntrinsicSourcesComponent::new(
                    rules.mechanics().version().clone(),
                    affinity_bindings(&rules, character.entity, &character.affinities)?,
                )?,
            )?;
            attach(
                &mut entities,
                character.entity,
                ActiveEffectsComponent::new(rules.mechanics().version().clone(), vec![])?,
            )?;
            attach(
                &mut entities,
                character.entity,
                EquipmentComponent::new(rules.mechanics().version().clone(), vec![])?,
            )?;
        }
        for inventory in &inventories {
            attach_inventory(
                &mut entities,
                &rules,
                inventory.owner,
                inventory.maximum_items,
            )?;
        }
        for storage in &storage {
            attach_inventory(&mut entities, &rules, storage.entity, storage.maximum_items)?;
        }
        for item in &equipment_items {
            rules
                .equipment_definition(&item.equipment)
                .ok_or_else(|| D20SessionError::UnknownEquipment(item.equipment.clone()))?;
            attach(
                &mut entities,
                item.entity,
                ItemComponent::new(
                    rules.mechanics().version().clone(),
                    item.equipment.mechanics_item_id(),
                ),
            )?;
        }
        gameplay_mechanics::validate_state_against_catalog(&entities, rules.mechanics())?;
        Ok(Self {
            rules,
            entities,
            roll_source,
            next_roll: 0,
            current_turn: 0,
        })
    }

    pub fn rules(&self) -> &D20Ruleset {
        &self.rules
    }

    pub fn entities(&self) -> &EntityState {
        &self.entities
    }

    pub const fn current_turn(&self) -> u64 {
        self.current_turn
    }

    pub const fn next_roll_index(&self) -> u64 {
        self.next_roll
    }

    pub const fn roll_source(&self) -> &RollSourceConfig {
        &self.roll_source
    }

    pub fn choice_index(&self, scope: &str, upper: u32) -> Option<u32> {
        if upper == 0 {
            return None;
        }
        match self.roll_source {
            RollSourceConfig::Seeded { seed } => {
                let mut rng = ScopedRng::new(
                    RngSeed::new(seed),
                    &format!(
                        "d20-choice/{scope}/turn-{}/roll-{}",
                        self.current_turn, self.next_roll
                    ),
                );
                rng.next_bounded_u32(upper)
            }
            RollSourceConfig::Static { ref rolls } => usize::try_from(self.next_roll)
                .ok()
                .and_then(|index| rolls.get(index))
                .map(|roll| (u32::from(roll.d20) - 1) % upper),
        }
    }

    pub fn install_encounter_participation(
        &mut self,
        encounter: D20Id,
        participants: Vec<EncounterParticipationSeed>,
    ) -> Result<(), D20SessionError> {
        if participants.is_empty() {
            return Err(D20SessionError::InvalidEncounterParticipation(
                "an encounter requires at least one participant".to_owned(),
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for participant in &participants {
            if !seen.insert(participant.entity) {
                return Err(D20SessionError::InvalidEncounterParticipation(
                    "encounter participants must be distinct".to_owned(),
                ));
            }
            if self
                .entities
                .component::<AbilityScoresComponent>(participant.entity)?
                .is_none()
            {
                return Err(D20SessionError::MissingComponent {
                    entity: participant.entity,
                    component: AbilityScoresComponent::LABEL,
                });
            }
        }

        let mut staged = self.entities.clone();
        let existing = staged
            .components::<EncounterParticipationComponent>()?
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>();
        for entity in existing {
            let revision = staged.component_revision::<EncounterParticipationComponent>(entity)?;
            EntityAuthoringService.detach_component::<EncounterParticipationComponent>(
                &mut staged,
                revision,
                entity,
            )?;
        }
        for participant in participants {
            attach(
                &mut staged,
                participant.entity,
                EncounterParticipationComponent::new(
                    encounter.clone(),
                    participant.faction,
                    participant.initiative,
                    participant.position,
                ),
            )?;
        }
        self.entities = staged;
        Ok(())
    }

    pub fn clear_encounter_participation(&mut self) -> Result<(), D20SessionError> {
        let mut staged = self.entities.clone();
        let existing = staged
            .components::<EncounterParticipationComponent>()?
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>();
        for entity in existing {
            let revision = staged.component_revision::<EncounterParticipationComponent>(entity)?;
            EntityAuthoringService.detach_component::<EncounterParticipationComponent>(
                &mut staged,
                revision,
                entity,
            )?;
        }
        self.entities = staged;
        Ok(())
    }

    pub fn encounter_participants(
        &self,
    ) -> Result<Vec<(EntityId, EncounterParticipationComponent)>, D20SessionError> {
        Ok(self
            .entities
            .components::<EncounterParticipationComponent>()?
            .map(|(entity, component)| (entity, component.clone()))
            .collect())
    }

    pub fn encounter_participation(
        &self,
        entity: EntityId,
    ) -> Result<Option<&EncounterParticipationComponent>, D20SessionError> {
        Ok(self
            .entities
            .component::<EncounterParticipationComponent>(entity)?)
    }

    pub fn activation_budgets(
        &self,
        entity: EntityId,
    ) -> Result<&ActivationBudgetsComponent, D20SessionError> {
        self.entities
            .component::<ActivationBudgetsComponent>(entity)?
            .ok_or(D20SessionError::MissingComponent {
                entity,
                component: ActivationBudgetsComponent::LABEL,
            })
    }

    pub fn relocate_encounter_participant(
        &mut self,
        entity: EntityId,
        destination: TacticalPosition,
        movement_cost: u16,
    ) -> Result<(), D20SessionError> {
        let participation = self
            .entities
            .component::<EncounterParticipationComponent>(entity)?
            .ok_or(D20SessionError::MissingComponent {
                entity,
                component: EncounterParticipationComponent::LABEL,
            })?
            .clone();
        if movement_cost > 0 {
            if let Some(effect) = self.active_movement_prohibition(entity)? {
                return Err(D20SessionError::MovementForbidden { entity, effect });
            }
        }
        let mut staged = self.entities.clone();
        if movement_cost > 0 {
            let movement = D20Id::parse("movement").expect("movement is a valid d20 identity");
            let budgets = staged
                .component::<ActivationBudgetsComponent>(entity)?
                .ok_or(D20SessionError::MissingComponent {
                    entity,
                    component: ActivationBudgetsComponent::LABEL,
                })?
                .clone();
            let available = budgets.current(&movement).unwrap_or(0);
            let after = budgets.spend(&movement, movement_cost).ok_or({
                D20SessionError::ActivationBudgetUnavailable {
                    entity,
                    budget: movement,
                    required: movement_cost,
                    available,
                }
            })?;
            let revision = staged.component_revision::<ActivationBudgetsComponent>(entity)?;
            EntityAuthoringService.replace_component(&mut staged, revision, entity, after)?;
        }
        let revision = staged.component_revision::<EncounterParticipationComponent>(entity)?;
        EntityAuthoringService.replace_component(
            &mut staged,
            revision,
            entity,
            participation.with_position(destination),
        )?;
        self.entities = staged;
        Ok(())
    }

    pub fn active_movement_prohibition(
        &self,
        entity: EntityId,
    ) -> Result<Option<D20Id>, D20SessionError> {
        let schedule = self
            .entities
            .component::<ScheduledEffectsComponent>(entity)?
            .ok_or(D20SessionError::MissingComponent {
                entity,
                component: ScheduledEffectsComponent::LABEL,
            })?;
        Ok(schedule
            .effects()
            .iter()
            .filter(|effect| effect.expires_at_turn() > self.current_turn)
            .find_map(|scheduled| {
                let definition = self
                    .rules
                    .effect(scheduled.definition())
                    .expect("validated schedules reference compiled effects");
                definition
                    .conditions
                    .iter()
                    .any(|condition| matches!(condition, ConditionClauseDefinition::ForbidMovement))
                    .then(|| definition.id.clone())
            }))
    }

    pub fn reset_activation_budgets(&mut self, entity: EntityId) -> Result<(), D20SessionError> {
        let reset = ActivationBudgetsComponent::new(
            self.rules
                .activation_budgets()
                .map(|budget| ActivationBudget::new(budget.id.clone(), budget.initial))
                .collect(),
        )?;
        let revision = self
            .entities
            .component_revision::<ActivationBudgetsComponent>(entity)?;
        EntityAuthoringService.replace_component(&mut self.entities, revision, entity, reset)?;
        Ok(())
    }

    pub fn restore_vitality(
        &mut self,
        entity: EntityId,
        amount: u32,
        operation: OperationId,
    ) -> Result<TrackMutationReceipt, D20SessionError> {
        let source = request_source(&operation, "restore-vitality");
        Ok(TrackService::restore(
            &mut self.entities,
            self.rules.mechanics(),
            TrackMutationRequest {
                operation,
                source,
                entity,
                track: vitality_track_id(),
                amount: scalar(i64::from(amount)),
                kind: gameplay_mechanics::TrackAdjustmentKind::Restore,
                expected_revision: None,
            },
        )?)
    }

    pub fn equip_armor(
        &mut self,
        owner: EntityId,
        item: EntityId,
        armor: &D20Id,
        operation: OperationId,
    ) -> Result<EquipmentMutationReceipt, D20SessionError> {
        self.equip_item(
            owner,
            item,
            &EquipmentReferenceDefinition::Armor {
                armor: armor.clone(),
            },
            operation,
        )
    }

    pub fn equip_item(
        &mut self,
        owner: EntityId,
        item: EntityId,
        equipment: &EquipmentReferenceDefinition,
        operation: OperationId,
    ) -> Result<EquipmentMutationReceipt, D20SessionError> {
        let (_, slot) = self
            .rules
            .equipment_definition(equipment)
            .ok_or_else(|| D20SessionError::UnknownEquipment(equipment.clone()))?;
        let expected_item = equipment.mechanics_item_id();
        let actual_item = self.entities.component::<ItemComponent>(item)?.ok_or(
            D20SessionError::MissingComponent {
                entity: item,
                component: ItemComponent::LABEL,
            },
        )?;
        if actual_item.definition() != &expected_item {
            return Err(D20SessionError::EquipmentItemMismatch {
                item,
                expected: equipment.clone(),
            });
        }
        let source = request_source(&operation, "equip-item");
        let expected_state_revision = self.entities.revision();
        Ok(EquipmentService::equip(
            &mut self.entities,
            self.rules.mechanics(),
            EquipmentEquipRequest {
                operation,
                source,
                owner,
                item,
                slots: vec![equipment_slot_id(slot)],
                expected_equipment_revision: None,
                expected_state_revision,
            },
        )?)
    }

    pub fn unequip_armor(
        &mut self,
        owner: EntityId,
        item: EntityId,
        operation: OperationId,
    ) -> Result<EquipmentMutationReceipt, D20SessionError> {
        self.unequip_item(owner, item, operation)
    }

    pub fn unequip_item(
        &mut self,
        owner: EntityId,
        item: EntityId,
        operation: OperationId,
    ) -> Result<EquipmentMutationReceipt, D20SessionError> {
        let source = request_source(&operation, "unequip-item");
        let expected_state_revision = self.entities.revision();
        Ok(EquipmentService::unequip(
            &mut self.entities,
            self.rules.mechanics(),
            EquipmentUnequipRequest {
                operation,
                source,
                owner,
                item,
                expected_equipment_revision: None,
                expected_state_revision,
            },
        )?)
    }

    pub fn transfer_armor(
        &mut self,
        item: EntityId,
        from_owner: EntityId,
        to_owner: EntityId,
        operation: OperationId,
    ) -> Result<ItemTransferReceipt, D20SessionError> {
        self.transfer_item(item, from_owner, to_owner, operation)
    }

    pub fn transfer_item(
        &mut self,
        item: EntityId,
        from_owner: EntityId,
        to_owner: EntityId,
        operation: OperationId,
    ) -> Result<ItemTransferReceipt, D20SessionError> {
        let source = request_source(&operation, "transfer-item");
        let expected_relationship_revision = self.entities.revision();
        Ok(EquipmentService::transfer_unique_item(
            &mut self.entities,
            self.rules.mechanics(),
            ItemTransferRequest {
                operation,
                source,
                item,
                from_owner,
                to_owner,
                expected_relationship_revision,
                expected_from_inventory_revision: None,
                expected_to_inventory_revision: None,
            },
        )?)
    }

    pub fn inventory_view(&self, owner: EntityId) -> Result<InventoryView, D20SessionError> {
        Ok(InventoryService::view(
            &self.entities,
            self.rules.mechanics(),
            owner,
        )?)
    }

    pub fn install_loadout(
        &mut self,
        inventories: Vec<InventorySeed>,
        storage: Vec<StorageSeed>,
        armor_items: Vec<ArmorItemSeed>,
    ) -> Result<(), D20SessionError> {
        self.install_equipment_loadout(
            inventories,
            storage,
            armor_items
                .into_iter()
                .map(EquipmentItemSeed::from)
                .collect(),
        )
    }

    pub fn install_equipment_loadout(
        &mut self,
        inventories: Vec<InventorySeed>,
        storage: Vec<StorageSeed>,
        equipment_items: Vec<EquipmentItemSeed>,
    ) -> Result<(), D20SessionError> {
        let mut staged = self.entities.clone();
        let mut definitions = storage
            .iter()
            .map(|storage| EntityDefinition::new(storage.entity, storage.name.clone()))
            .collect::<Vec<_>>();
        definitions.extend(equipment_items.iter().map(|item| {
            EntityDefinition::new(item.entity, item.name.clone()).with_containment(item.owner)
        }));
        let revision = staged.revision();
        EntityAuthoringService.admit(&mut staged, revision, definitions)?;
        for inventory in &inventories {
            attach_inventory(
                &mut staged,
                &self.rules,
                inventory.owner,
                inventory.maximum_items,
            )?;
        }
        for storage in &storage {
            attach_inventory(
                &mut staged,
                &self.rules,
                storage.entity,
                storage.maximum_items,
            )?;
        }
        for item in &equipment_items {
            self.rules
                .equipment_definition(&item.equipment)
                .ok_or_else(|| D20SessionError::UnknownEquipment(item.equipment.clone()))?;
            attach(
                &mut staged,
                item.entity,
                ItemComponent::new(
                    self.rules.mechanics().version().clone(),
                    item.equipment.mechanics_item_id(),
                ),
            )?;
        }
        gameplay_mechanics::validate_state_against_catalog(&staged, self.rules.mechanics())?;
        self.entities = staged;
        Ok(())
    }
}

impl D20Session {
    pub fn advance_turn(
        &mut self,
        next_turn: u64,
        operation: OperationId,
    ) -> Result<AdvanceTurnReceipt, D20SessionError> {
        if next_turn <= self.current_turn {
            return Err(D20SessionError::TurnMustAdvance {
                current: self.current_turn,
                requested: next_turn,
            });
        }
        let scheduled = self
            .entities
            .components::<ScheduledEffectsComponent>()?
            .map(|(entity, component)| (entity, component.clone()))
            .collect::<Vec<_>>();
        let mut staged = self.entities.clone();
        let mut expired_receipts = Vec::new();
        for (entity, schedule) in scheduled {
            let due = schedule
                .effects()
                .iter()
                .filter(|effect| effect.expires_at_turn() <= next_turn)
                .map(|effect| effect.instance().clone())
                .collect::<Vec<_>>();
            if due.is_empty() {
                continue;
            }
            for instance in &due {
                expired_receipts.push(EffectService::expire(
                    &mut staged,
                    self.rules.mechanics(),
                    EffectRemovalRequest {
                        operation: operation.clone(),
                        entity,
                        instance: instance.clone(),
                        expected_revision: None,
                    },
                )?);
            }
            let revision = staged.component_revision::<ScheduledEffectsComponent>(entity)?;
            EntityAuthoringService.replace_component(
                &mut staged,
                revision,
                entity,
                schedule.without_instances(&due)?,
            )?;
        }
        let before = self.current_turn;
        self.entities = staged;
        self.current_turn = next_turn;
        Ok(AdvanceTurnReceipt {
            before,
            after: next_turn,
            expired: expired_receipts,
        })
    }
}

pub(super) fn validate_character_seed(
    rules: &D20Ruleset,
    character: &CharacterSeed,
) -> Result<(), D20SessionError> {
    for ability in rules.abilities() {
        let score = character
            .abilities
            .iter()
            .find(|score| score.id() == &ability.id)
            .ok_or_else(|| D20SessionError::MissingAbility {
                entity: character.entity,
                ability: ability.id.clone(),
            })?;
        if score.score() < ability.minimum || score.score() > ability.maximum {
            return Err(D20SessionError::InvalidAbilityScore {
                entity: character.entity,
                ability: ability.id.clone(),
                score: score.score(),
                minimum: ability.minimum,
                maximum: ability.maximum,
            });
        }
    }
    for score in &character.abilities {
        if rules.ability(score.id()).is_none() {
            return Err(D20SessionError::MissingAbility {
                entity: character.entity,
                ability: score.id().clone(),
            });
        }
    }
    for resource in rules.resources() {
        let current = character
            .resources
            .iter()
            .find(|value| value.id() == &resource.id)
            .ok_or_else(|| D20SessionError::MissingResource {
                entity: character.entity,
                resource: resource.id.clone(),
            })?
            .current();
        if current > resource.maximum {
            return Err(D20SessionError::InvalidResourceValue {
                entity: character.entity,
                resource: resource.id.clone(),
                current,
                maximum: resource.maximum,
            });
        }
    }
    for resource in &character.resources {
        if rules.resource(resource.id()).is_none() {
            return Err(D20SessionError::MissingResource {
                entity: character.entity,
                resource: resource.id().clone(),
            });
        }
    }
    if character.vitality > 1_000_000 {
        return Err(D20SessionError::Mechanics(
            MechanicsError::TrackOutOfBounds {
                entity: character.entity,
                track: vitality_track_id(),
                attempted: i64::from(character.vitality),
                minimum: 0,
                maximum: 1_000_000,
            },
        ));
    }
    Ok(())
}

fn affinity_bindings(
    rules: &D20Ruleset,
    entity: EntityId,
    affinities: &[AffinitySeed],
) -> Result<Vec<IntrinsicSourceBinding>, D20SessionError> {
    let mut seen = std::collections::BTreeSet::new();
    let mut bindings = Vec::new();
    for affinity in affinities {
        if !rules
            .damage_types()
            .any(|kind| kind == &affinity.damage_type)
        {
            return Err(D20SessionError::UnknownDamageType(
                affinity.damage_type.clone(),
            ));
        }
        if !seen.insert(affinity.damage_type.clone()) {
            return Err(D20SessionError::DuplicateAffinity {
                entity,
                damage_type: affinity.damage_type.clone(),
            });
        }
        let definition = match affinity.affinity {
            DamageAffinity::Resistant => resistance_source_id(&affinity.damage_type),
            DamageAffinity::Vulnerable => vulnerability_source_id(&affinity.damage_type),
        };
        bindings.push(IntrinsicSourceBinding::new(
            SourceInstanceId::parse(format!("affinity.{}", affinity.damage_type))
                .expect("validated d20 identity fits mechanics identity"),
            definition,
        ));
    }
    Ok(bindings)
}

fn attach<T: EntityComponent>(
    state: &mut EntityState,
    entity: EntityId,
    component: T,
) -> Result<(), D20SessionError> {
    let revision = state.component_revision::<T>(entity)?;
    EntityAuthoringService.attach_component(state, revision, entity, component)?;
    Ok(())
}

fn attach_inventory(
    state: &mut EntityState,
    rules: &D20Ruleset,
    owner: EntityId,
    maximum_items: u64,
) -> Result<(), D20SessionError> {
    attach(
        state,
        owner,
        InventoryComponent::with_capacity_limits(
            rules.mechanics().version().clone(),
            vec![],
            vec![InventoryCapacityLimit::new(
                loadout_capacity_id(),
                maximum_items,
            )],
        )?,
    )
}
