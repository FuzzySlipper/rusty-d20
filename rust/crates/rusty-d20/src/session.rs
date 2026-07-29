use core_ids::EntityId;
use entity_state::{
    encode_snapshot, ComponentAccessError, ComponentRegistrationError, ComponentRevision,
    EntityAuthoringError, EntityAuthoringService, EntityComponent, EntityDefinition,
    EntityDefinitionError, EntityState,
};
use gameplay_mechanics::{
    decode_snapshot_with_catalog_and_registry, ActiveEffectsComponent, DamagePart, DamageReceipt,
    DamageRequest, DamageService, EffectApplyRequest, EffectInstanceId, EffectMutationReceipt,
    EffectRefreshRequest, EffectRemovalRequest, EffectService, EquipmentComponent,
    EquipmentEquipRequest, EquipmentMutationReceipt, EquipmentService, EquipmentUnequipRequest,
    IntrinsicSourceBinding, IntrinsicSourcesComponent, InventoryCapacityLimit, InventoryComponent,
    InventoryService, InventoryView, ItemComponent, ItemTransferReceipt, ItemTransferRequest,
    MechanicsComponentKind, MechanicsError, MechanicsScalar, MechanicsSnapshotError,
    ObservedComponentRevision, OperationId, SourceInstanceId, SourceInstanceIdentity,
    StatEvaluation, StatService, StatValue, StatsComponent, TrackMutationReceipt,
    TrackMutationRequest, TrackService, TrackValue, TracksComponent,
};
use serde::{Deserialize, Serialize};
use svc_rng::{RngSeed, ScopedRng};

use crate::compiler::{
    damage_kind_id, defense_stat_id, equipment_slot_id, loadout_capacity_id, mechanics_effect_id,
    resistance_source_id, vitality_track_id, vulnerability_source_id,
};
use crate::{
    d20_component_registry, AbilityScore, AbilityScoresComponent, ActionAttackDefinition,
    ActionDefinition, ActionResource, ActionResourcesComponent, ActivationBudget,
    ActivationBudgetsComponent, ActivationCostDefinition, ConditionClauseDefinition,
    D20ComponentDataError, D20Id, D20Ruleset, DamageDefinition, EncounterFaction,
    EncounterParticipationComponent, EquipmentReferenceDefinition, ScheduledEffect,
    ScheduledEffectsComponent, ENGINE_REVISION,
};

const D20_SAVE_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DamageAffinity {
    Resistant,
    Vulnerable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffinitySeed {
    pub damage_type: D20Id,
    pub affinity: DamageAffinity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterSeed {
    pub entity: EntityId,
    pub name: String,
    pub vitality: u32,
    pub abilities: Vec<AbilityScore>,
    pub resources: Vec<ActionResource>,
    pub affinities: Vec<AffinitySeed>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmorItemSeed {
    pub entity: EntityId,
    pub owner: EntityId,
    pub name: String,
    pub armor: D20Id,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipmentItemSeed {
    pub entity: EntityId,
    pub owner: EntityId,
    pub name: String,
    pub equipment: EquipmentReferenceDefinition,
}

impl From<ArmorItemSeed> for EquipmentItemSeed {
    fn from(value: ArmorItemSeed) -> Self {
        Self {
            entity: value.entity,
            owner: value.owner,
            name: value.name,
            equipment: EquipmentReferenceDefinition::Armor { armor: value.armor },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventorySeed {
    pub owner: EntityId,
    pub maximum_items: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageSeed {
    pub entity: EntityId,
    pub name: String,
    pub maximum_items: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncounterParticipationSeed {
    pub entity: EntityId,
    pub faction: EncounterFaction,
    pub initiative: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionOption {
    reaction: D20Id,
    resource: D20Id,
    cost: u16,
    available: u16,
    bonus: i16,
    effect: D20Id,
}

impl ReactionOption {
    pub const fn reaction(&self) -> &D20Id {
        &self.reaction
    }

    pub const fn resource(&self) -> &D20Id {
        &self.resource
    }

    pub const fn cost(&self) -> u16 {
        self.cost
    }

    pub const fn available(&self) -> u16 {
        self.available
    }

    pub const fn bonus(&self) -> i16 {
        self.bonus
    }

    pub const fn effect(&self) -> &D20Id {
        &self.effect
    }
}

/// An immutable authority token plus read-only preview projection.
///
/// Callers can inspect a preview but cannot rewrite the action selected by
/// Rust:
///
/// ```compile_fail,E0616
/// fn rewrite_action(mut preview: rusty_d20::ActionPreview) {
///     preview.action = rusty_d20::D20Id::parse("other-action").unwrap();
/// }
/// ```
///
/// Outcome inputs are likewise not caller-controlled:
///
/// ```compile_fail,E0616
/// fn rewrite_modifier(mut preview: rusty_d20::ActionPreview) {
///     preview.ability_modifier = 100;
/// }
/// ```
///
/// ```compile_fail,E0616
/// fn rewrite_defense(mut preview: rusty_d20::ActionPreview) {
///     preview.defense.value = gameplay_mechanics::MechanicsScalar::zero();
/// }
/// ```
///
/// The reaction projection is an immutable slice, so a consumer cannot inject
/// a reaction for a different defense:
///
/// ```compile_fail,E0616
/// fn inject_reaction(mut preview: rusty_d20::ActionPreview) {
///     preview.reactions.clear();
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ActionPreview {
    actor: EntityId,
    target: EntityId,
    action: D20Id,
    operation: OperationId,
    ability_score: i16,
    ability_modifier: i16,
    damage: DamageDefinition,
    defense: StatEvaluation,
    reactions: Vec<ReactionOption>,
    actor_abilities_revision: ComponentRevision,
    actor_activation_budgets_revision: ComponentRevision,
    actor_equipment_revision: ComponentRevision,
    actor_scheduled_effects_revision: ComponentRevision,
    target_resources_revision: ComponentRevision,
    target_activation_budgets_revision: ComponentRevision,
    target_tracks_revision: ComponentRevision,
    target_scheduled_effects_revision: ComponentRevision,
    turn: u64,
    roll_index: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedActionDefinition {
    pub ability: D20Id,
    pub defense: D20Id,
    pub damage: DamageDefinition,
    pub range: u16,
    pub implement: Option<D20Id>,
}

impl ActionPreview {
    pub const fn actor(&self) -> EntityId {
        self.actor
    }

    pub const fn target(&self) -> EntityId {
        self.target
    }

    pub const fn action(&self) -> &D20Id {
        &self.action
    }

    pub const fn operation(&self) -> &OperationId {
        &self.operation
    }

    pub const fn ability_score(&self) -> i16 {
        self.ability_score
    }

    pub const fn ability_modifier(&self) -> i16 {
        self.ability_modifier
    }

    pub const fn defense(&self) -> &StatEvaluation {
        &self.defense
    }

    pub fn reactions(&self) -> &[ReactionOption] {
        &self.reactions
    }
}

#[derive(Debug, Clone)]
pub struct ApplyActionRequest {
    pub preview: ActionPreview,
    pub effect_instance: Option<EffectInstanceId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionReceipt {
    pub actor: EntityId,
    pub target: EntityId,
    pub action: D20Id,
    pub operation: OperationId,
    pub roll_index: u64,
    pub d20: u8,
    pub ability_modifier: i16,
    pub total: i32,
    pub defense: i64,
    pub hit: bool,
    pub rolled_damage: u32,
    pub damage: Option<DamageReceipt>,
    pub effect: Option<EffectMutationReceipt>,
    pub expires_at_turn: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionReceipt {
    pub reaction: D20Id,
    pub target: EntityId,
    pub resource: D20Id,
    pub before: u16,
    pub after: u16,
    pub effect: EffectMutationReceipt,
    pub expires_at_turn: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvanceTurnReceipt {
    pub before: u64,
    pub after: u64,
    pub expired: Vec<EffectMutationReceipt>,
}

#[derive(Debug, Clone)]
pub struct D20Session {
    rules: D20Ruleset,
    entities: EntityState,
    seed: RngSeed,
    next_roll: u64,
    current_turn: u64,
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
            seed,
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

    pub fn deterministic_choice_index(&self, scope: &str, upper: u32) -> Option<u32> {
        let mut rng = ScopedRng::new(
            self.seed,
            &format!(
                "d20-choice/{scope}/turn-{}/roll-{}",
                self.current_turn, self.next_roll
            ),
        );
        rng.next_bounded_u32(upper)
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

    pub fn preview_action(
        &self,
        actor: EntityId,
        target: EntityId,
        action: &D20Id,
        operation: OperationId,
    ) -> Result<ActionPreview, D20SessionError> {
        let action_definition = self
            .rules
            .action(action)
            .ok_or_else(|| D20SessionError::UnknownAction(action.clone()))?;
        self.ensure_activation_costs(actor, &action_definition.activation_costs)?;
        let resolved = self.resolve_action_definition(actor, action_definition)?;
        let abilities = self
            .entities
            .component::<AbilityScoresComponent>(actor)?
            .ok_or(D20SessionError::MissingComponent {
                entity: actor,
                component: AbilityScoresComponent::LABEL,
            })?;
        let ability_score =
            abilities
                .score(&resolved.ability)
                .ok_or_else(|| D20SessionError::MissingAbility {
                    entity: actor,
                    ability: resolved.ability.clone(),
                })?;
        let attack_penalty = self.active_attack_penalty(actor, action_definition)?;
        let defense = StatService::evaluate(
            &self.entities,
            self.rules.mechanics(),
            target,
            &defense_stat_id(&resolved.defense),
            &operation,
            &[],
        )?;
        let resources = self
            .entities
            .component::<ActionResourcesComponent>(target)?
            .ok_or(D20SessionError::MissingComponent {
                entity: target,
                component: ActionResourcesComponent::LABEL,
            })?;
        let target_budgets = self.activation_budgets(target)?;
        let target_template = self
            .rules
            .character_templates()
            .find(|character| character.entity_id == target.raw())
            .ok_or_else(|| {
                D20SessionError::InvalidEncounterParticipation(format!(
                    "entity {target} is not a compiled character"
                ))
            })?;
        let reactions = target_template
            .reactions
            .iter()
            .filter_map(|reaction_id| self.rules.reaction(reaction_id))
            .filter(|reaction| reaction.defense == resolved.defense)
            .filter_map(|reaction| {
                let available = resources.current(&reaction.resource)?;
                let budgets_available = reaction.activation_costs.iter().all(|cost| {
                    target_budgets
                        .current(&cost.budget)
                        .is_some_and(|available| available >= cost.amount)
                });
                (available >= reaction.cost && budgets_available).then(|| ReactionOption {
                    reaction: reaction.id.clone(),
                    resource: reaction.resource.clone(),
                    cost: reaction.cost,
                    available,
                    bonus: reaction.bonus,
                    effect: reaction.effect.clone(),
                })
            })
            .collect();
        Ok(ActionPreview {
            actor,
            target,
            action: action.clone(),
            operation,
            ability_score,
            ability_modifier: ability_modifier(ability_score).saturating_add(attack_penalty),
            damage: resolved.damage,
            defense,
            reactions,
            actor_abilities_revision: self
                .entities
                .component_revision::<AbilityScoresComponent>(actor)?,
            actor_activation_budgets_revision: self
                .entities
                .component_revision::<ActivationBudgetsComponent>(actor)?,
            actor_equipment_revision: self
                .entities
                .component_revision::<EquipmentComponent>(actor)?,
            actor_scheduled_effects_revision: self
                .entities
                .component_revision::<ScheduledEffectsComponent>(actor)?,
            target_resources_revision: self
                .entities
                .component_revision::<ActionResourcesComponent>(target)?,
            target_activation_budgets_revision: self
                .entities
                .component_revision::<ActivationBudgetsComponent>(target)?,
            target_tracks_revision: self
                .entities
                .component_revision::<TracksComponent>(target)?,
            target_scheduled_effects_revision: self
                .entities
                .component_revision::<ScheduledEffectsComponent>(target)?,
            turn: self.current_turn,
            roll_index: self.next_roll,
        })
    }

    pub(crate) fn action_definition_profile(
        &self,
        action: &D20Id,
    ) -> Result<ResolvedActionDefinition, D20SessionError> {
        let definition = self
            .rules
            .action(action)
            .ok_or_else(|| D20SessionError::UnknownAction(action.clone()))?;
        Ok(self.static_action_definition(definition))
    }

    fn resolve_action_definition(
        &self,
        actor: EntityId,
        action: &ActionDefinition,
    ) -> Result<ResolvedActionDefinition, D20SessionError> {
        let resolved = self.static_action_definition(action);
        if let Some(implement) = &resolved.implement {
            let equipment = self
                .entities
                .component::<EquipmentComponent>(actor)?
                .ok_or(D20SessionError::MissingComponent {
                    entity: actor,
                    component: EquipmentComponent::LABEL,
                })?;
            let required_item = crate::compiler::implement_item_id(implement);
            let equipped = equipment.assignments().iter().any(|assignment| {
                self.entities
                    .component::<ItemComponent>(assignment.item)
                    .ok()
                    .flatten()
                    .is_some_and(|item| item.definition() == &required_item)
            });
            if !equipped {
                return Err(D20SessionError::RequiredImplementNotEquipped {
                    entity: actor,
                    implement: implement.clone(),
                });
            }
        }
        Ok(resolved)
    }

    fn static_action_definition(&self, action: &ActionDefinition) -> ResolvedActionDefinition {
        match &action.attack {
            ActionAttackDefinition::Fixed {
                ability,
                defense,
                damage,
                range,
            } => ResolvedActionDefinition {
                ability: ability.clone(),
                defense: defense.clone(),
                damage: damage.clone(),
                range: *range,
                implement: None,
            },
            ActionAttackDefinition::Implement { implement } => {
                let definition = self
                    .rules
                    .implement(implement)
                    .expect("compiled action references a known implement");
                ResolvedActionDefinition {
                    ability: definition.ability.clone(),
                    defense: definition.defense.clone(),
                    damage: definition.damage.clone(),
                    range: definition.range,
                    implement: Some(definition.id.clone()),
                }
            }
        }
    }

    fn ensure_activation_costs(
        &self,
        entity: EntityId,
        costs: &[ActivationCostDefinition],
    ) -> Result<(), D20SessionError> {
        let budgets = self.activation_budgets(entity)?;
        for cost in costs {
            let available = budgets.current(&cost.budget).unwrap_or(0);
            if available < cost.amount {
                return Err(D20SessionError::ActivationBudgetUnavailable {
                    entity,
                    budget: cost.budget.clone(),
                    required: cost.amount,
                    available,
                });
            }
        }
        Ok(())
    }

    fn active_attack_penalty(
        &self,
        actor: EntityId,
        action: &ActionDefinition,
    ) -> Result<i16, D20SessionError> {
        let schedule = self
            .entities
            .component::<ScheduledEffectsComponent>(actor)?
            .ok_or(D20SessionError::MissingComponent {
                entity: actor,
                component: ScheduledEffectsComponent::LABEL,
            })?;
        let mut penalty = 0_i16;
        for scheduled in schedule
            .effects()
            .iter()
            .filter(|effect| effect.expires_at_turn() > self.current_turn)
        {
            let definition = self
                .rules
                .effect(scheduled.definition())
                .expect("restored and authored schedules reference compiled effects");
            for condition in &definition.conditions {
                match condition {
                    ConditionClauseDefinition::ForbidActionTag { tag }
                        if action.tags.contains(tag) =>
                    {
                        return Err(D20SessionError::ActionForbidden {
                            entity: actor,
                            action: action.id.clone(),
                            effect: definition.id.clone(),
                        });
                    }
                    ConditionClauseDefinition::AttackPenalty { amount } => {
                        penalty = penalty.saturating_add(*amount);
                    }
                    ConditionClauseDefinition::ForbidMovement
                    | ConditionClauseDefinition::ForbidActionTag { .. } => {}
                }
            }
        }
        Ok(penalty)
    }

    pub fn apply_reaction(
        &mut self,
        preview: &ActionPreview,
        reaction: &D20Id,
        effect_instance: EffectInstanceId,
    ) -> Result<ReactionReceipt, D20SessionError> {
        self.ensure_fresh(preview)?;
        let option = preview
            .reactions
            .iter()
            .find(|option| &option.reaction == reaction)
            .ok_or_else(|| D20SessionError::ReactionUnavailable(reaction.clone()))?;
        let definition = self
            .rules
            .reaction(reaction)
            .ok_or_else(|| D20SessionError::ReactionUnavailable(reaction.clone()))?;
        let effect = self
            .rules
            .effect(&definition.effect)
            .expect("compiled reaction references a known effect");
        let before_component = self
            .entities
            .component::<ActionResourcesComponent>(preview.target)?
            .expect("preview requires target resources");
        let before = before_component
            .current(&definition.resource)
            .expect("compiled and admitted character resource exists");
        let after_component = before_component
            .spend(&definition.resource, definition.cost)
            .ok_or_else(|| D20SessionError::ReactionUnavailable(reaction.clone()))?;
        let before_budgets = self.activation_budgets(preview.target)?;
        let after_budgets =
            spend_activation_costs(before_budgets, preview.target, &definition.activation_costs)?;

        let mut staged = self.entities.clone();
        EntityAuthoringService.replace_component(
            &mut staged,
            preview.target_resources_revision.clone(),
            preview.target,
            after_component,
        )?;
        EntityAuthoringService.replace_component(
            &mut staged,
            preview.target_activation_budgets_revision.clone(),
            preview.target,
            after_budgets,
        )?;
        let expires_at_turn = self
            .current_turn
            .checked_add(u64::from(effect.duration_turns))
            .ok_or(D20SessionError::TurnOverflow)?;
        let effect_receipt = apply_or_refresh_scheduled_effect(
            &mut staged,
            &self.rules,
            preview.target,
            &preview.operation,
            &definition.effect,
            effect_instance,
            "reaction",
            &preview.target_scheduled_effects_revision,
            expires_at_turn,
        )?;
        self.entities = staged;
        Ok(ReactionReceipt {
            reaction: reaction.clone(),
            target: preview.target,
            resource: option.resource.clone(),
            before,
            after: before - option.cost,
            effect: effect_receipt,
            expires_at_turn,
        })
    }

    pub fn apply_action(
        &mut self,
        request: ApplyActionRequest,
    ) -> Result<ActionReceipt, D20SessionError> {
        self.ensure_fresh(&request.preview)?;
        let next_roll = self
            .next_roll
            .checked_add(1)
            .ok_or(D20SessionError::RollIndexOverflow)?;
        let action = self
            .rules
            .action(&request.preview.action)
            .expect("preview references a compiled action");
        let mut rng = ScopedRng::new(self.seed, &format!("d20-action-roll/{}", self.next_roll));
        let d20 = u8::try_from(rng.next_bounded_u32(20).expect("fixed nonzero d20 bound") + 1)
            .expect("d20 roll fits u8");
        let total = i32::from(d20) + i32::from(request.preview.ability_modifier);
        let hit = i64::from(total) >= request.preview.defense.value.get();

        let mut rolled_damage = 0_u32;
        if hit {
            for _ in 0..request.preview.damage.dice {
                rolled_damage = rolled_damage
                    .checked_add(
                        rng.next_bounded_u32(u32::from(request.preview.damage.sides))
                            .expect("compiled damage die has a nonzero bound")
                            + 1,
                    )
                    .ok_or(D20SessionError::DamageOverflow)?;
            }
        }
        let adjusted_damage = i64::from(rolled_damage) + i64::from(request.preview.damage.bonus);
        let applied_damage = adjusted_damage.max(0);

        let mut staged = self.entities.clone();
        let actor_budgets = self.activation_budgets(request.preview.actor)?;
        let after_actor_budgets = spend_activation_costs(
            actor_budgets,
            request.preview.actor,
            &action.activation_costs,
        )?;
        EntityAuthoringService.replace_component(
            &mut staged,
            request.preview.actor_activation_budgets_revision.clone(),
            request.preview.actor,
            after_actor_budgets,
        )?;
        let damage = if hit {
            Some(DamageService::apply(
                &mut staged,
                self.rules.mechanics(),
                DamageRequest {
                    operation: request.preview.operation.clone(),
                    source: request_source(&request.preview.operation, "action"),
                    actor: Some(request.preview.actor),
                    target: request.preview.target,
                    target_track: vitality_track_id(),
                    parts: vec![DamagePart {
                        amount: scalar(applied_damage),
                        kind: damage_kind_id(&request.preview.damage.kind),
                    }],
                    request_sources: vec![],
                    expected_tracks_revision: Some(request.preview.target_tracks_revision.clone()),
                },
            )?)
        } else {
            None
        };

        let (effect_receipt, expires_at_turn) = if hit {
            if let Some(effect_id) = &action.effect {
                let instance = request
                    .effect_instance
                    .ok_or_else(|| D20SessionError::MissingEffectInstance(effect_id.clone()))?;
                let effect = self
                    .rules
                    .effect(effect_id)
                    .expect("compiled action effect exists");
                let expires_at = self
                    .current_turn
                    .checked_add(u64::from(effect.duration_turns))
                    .ok_or(D20SessionError::TurnOverflow)?;
                let receipt = apply_or_refresh_scheduled_effect(
                    &mut staged,
                    &self.rules,
                    request.preview.target,
                    &request.preview.operation,
                    effect_id,
                    instance,
                    "action-effect",
                    &request.preview.target_scheduled_effects_revision,
                    expires_at,
                )?;
                (Some(receipt), Some(expires_at))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        self.entities = staged;
        let roll_index = self.next_roll;
        self.next_roll = next_roll;
        Ok(ActionReceipt {
            actor: request.preview.actor,
            target: request.preview.target,
            action: request.preview.action,
            operation: request.preview.operation,
            roll_index,
            d20,
            ability_modifier: request.preview.ability_modifier,
            total,
            defense: request.preview.defense.value.get(),
            hit,
            rolled_damage,
            damage,
            effect: effect_receipt,
            expires_at_turn,
        })
    }

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

    pub fn encode_save(&self) -> Result<String, SessionSaveError> {
        let entity_state = serde_json::from_str(&encode_snapshot(&self.entities)?)?;
        Ok(serde_json::to_string_pretty(&D20SessionSave {
            schema_version: D20_SAVE_SCHEMA_VERSION,
            engine_revision: ENGINE_REVISION.to_owned(),
            ruleset_fingerprint: self.rules.fingerprint().to_owned(),
            seed: self.seed.raw(),
            next_roll: self.next_roll,
            current_turn: self.current_turn,
            entity_state,
        })?)
    }

    pub fn decode_save(rules: D20Ruleset, input: &str) -> Result<Self, SessionSaveError> {
        let save: D20SessionSave = serde_json::from_str(input)?;
        if save.schema_version != D20_SAVE_SCHEMA_VERSION {
            return Err(SessionSaveError::UnsupportedSchema {
                actual: save.schema_version,
            });
        }
        if save.engine_revision != ENGINE_REVISION {
            return Err(SessionSaveError::EngineRevisionMismatch {
                expected: ENGINE_REVISION.to_owned(),
                actual: save.engine_revision,
            });
        }
        if save.ruleset_fingerprint != rules.fingerprint() {
            return Err(SessionSaveError::RulesetMismatch {
                expected: rules.fingerprint().to_owned(),
                actual: save.ruleset_fingerprint,
            });
        }
        let entity_state = serde_json::to_string(&save.entity_state)?;
        let registry = d20_component_registry()?;
        let entities =
            decode_snapshot_with_catalog_and_registry(&entity_state, registry, rules.mechanics())?;
        validate_restored_d20_state(&entities, &rules)?;
        Ok(Self {
            rules,
            entities,
            seed: RngSeed::new(save.seed),
            next_roll: save.next_roll,
            current_turn: save.current_turn,
        })
    }

    fn ensure_fresh(&self, preview: &ActionPreview) -> Result<(), D20SessionError> {
        if preview.turn != self.current_turn || preview.roll_index != self.next_roll {
            return Err(D20SessionError::StalePreview {
                reason: "turn or deterministic roll position changed",
            });
        }
        ensure_component_revision(
            &self.entities,
            &preview.actor_abilities_revision,
            self.entities
                .component_revision::<AbilityScoresComponent>(preview.actor)?,
        )?;
        ensure_component_revision(
            &self.entities,
            &preview.actor_activation_budgets_revision,
            self.entities
                .component_revision::<ActivationBudgetsComponent>(preview.actor)?,
        )?;
        ensure_component_revision(
            &self.entities,
            &preview.actor_equipment_revision,
            self.entities
                .component_revision::<EquipmentComponent>(preview.actor)?,
        )?;
        ensure_component_revision(
            &self.entities,
            &preview.actor_scheduled_effects_revision,
            self.entities
                .component_revision::<ScheduledEffectsComponent>(preview.actor)?,
        )?;
        ensure_component_revision(
            &self.entities,
            &preview.target_resources_revision,
            self.entities
                .component_revision::<ActionResourcesComponent>(preview.target)?,
        )?;
        ensure_component_revision(
            &self.entities,
            &preview.target_activation_budgets_revision,
            self.entities
                .component_revision::<ActivationBudgetsComponent>(preview.target)?,
        )?;
        ensure_component_revision(
            &self.entities,
            &preview.target_tracks_revision,
            self.entities
                .component_revision::<TracksComponent>(preview.target)?,
        )?;
        ensure_component_revision(
            &self.entities,
            &preview.target_scheduled_effects_revision,
            self.entities
                .component_revision::<ScheduledEffectsComponent>(preview.target)?,
        )?;
        for observed in &preview.defense.observed_revisions {
            let actual = mechanics_revision(&self.entities, observed)?;
            if actual != observed.revision {
                return Err(D20SessionError::StalePreview {
                    reason: "an observed mechanics component changed",
                });
            }
        }
        Ok(())
    }
}

pub const fn ability_modifier(score: i16) -> i16 {
    ((score as i32 - 10).div_euclid(2)) as i16
}

#[derive(Debug)]
pub enum D20SessionError {
    UnknownAction(D20Id),
    UnknownArmor(D20Id),
    UnknownEquipment(EquipmentReferenceDefinition),
    UnknownDamageType(D20Id),
    MissingAbility {
        entity: EntityId,
        ability: D20Id,
    },
    MissingResource {
        entity: EntityId,
        resource: D20Id,
    },
    InvalidAbilityScore {
        entity: EntityId,
        ability: D20Id,
        score: i16,
        minimum: i16,
        maximum: i16,
    },
    InvalidResourceValue {
        entity: EntityId,
        resource: D20Id,
        current: u16,
        maximum: u16,
    },
    DuplicateAffinity {
        entity: EntityId,
        damage_type: D20Id,
    },
    MissingComponent {
        entity: EntityId,
        component: &'static str,
    },
    ArmorItemMismatch {
        item: EntityId,
        expected: D20Id,
    },
    EquipmentItemMismatch {
        item: EntityId,
        expected: EquipmentReferenceDefinition,
    },
    RequiredImplementNotEquipped {
        entity: EntityId,
        implement: D20Id,
    },
    ActionForbidden {
        entity: EntityId,
        action: D20Id,
        effect: D20Id,
    },
    ActivationBudgetUnavailable {
        entity: EntityId,
        budget: D20Id,
        required: u16,
        available: u16,
    },
    InvalidEncounterParticipation(String),
    ReactionUnavailable(D20Id),
    MissingEffectInstance(D20Id),
    UnscheduledActiveEffect {
        entity: EntityId,
        instance: String,
    },
    StalePreview {
        reason: &'static str,
    },
    TurnMustAdvance {
        current: u64,
        requested: u64,
    },
    TurnOverflow,
    RollIndexOverflow,
    DamageOverflow,
    EntityDefinition(EntityDefinitionError),
    ComponentRegistration(ComponentRegistrationError),
    ComponentAccess(ComponentAccessError),
    ComponentMutation(EntityAuthoringError),
    ComponentData(D20ComponentDataError),
    Mechanics(MechanicsError),
    MechanicsComponentData(gameplay_mechanics::MechanicsComponentDataError),
}

impl std::fmt::Display for D20SessionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "d20 session operation failed: {self:?}")
    }
}

impl std::error::Error for D20SessionError {}

impl From<EntityDefinitionError> for D20SessionError {
    fn from(value: EntityDefinitionError) -> Self {
        Self::EntityDefinition(value)
    }
}

impl From<ComponentRegistrationError> for D20SessionError {
    fn from(value: ComponentRegistrationError) -> Self {
        Self::ComponentRegistration(value)
    }
}

impl From<ComponentAccessError> for D20SessionError {
    fn from(value: ComponentAccessError) -> Self {
        Self::ComponentAccess(value)
    }
}

impl From<EntityAuthoringError> for D20SessionError {
    fn from(value: EntityAuthoringError) -> Self {
        Self::ComponentMutation(value)
    }
}

impl From<D20ComponentDataError> for D20SessionError {
    fn from(value: D20ComponentDataError) -> Self {
        Self::ComponentData(value)
    }
}

impl From<MechanicsError> for D20SessionError {
    fn from(value: MechanicsError) -> Self {
        Self::Mechanics(value)
    }
}

impl From<gameplay_mechanics::MechanicsComponentDataError> for D20SessionError {
    fn from(value: gameplay_mechanics::MechanicsComponentDataError) -> Self {
        Self::MechanicsComponentData(value)
    }
}

#[derive(Debug)]
pub enum SessionSaveError {
    Json(serde_json::Error),
    Snapshot(entity_state::EntityStateSnapshotError),
    MechanicsSnapshot(MechanicsSnapshotError),
    ComponentRegistration(ComponentRegistrationError),
    UnsupportedSchema { actual: u32 },
    EngineRevisionMismatch { expected: String, actual: String },
    RulesetMismatch { expected: String, actual: String },
    InvalidState(D20SessionError),
}

impl std::fmt::Display for SessionSaveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "d20 session save rejected: {self:?}")
    }
}

impl std::error::Error for SessionSaveError {}

impl From<serde_json::Error> for SessionSaveError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<entity_state::EntityStateSnapshotError> for SessionSaveError {
    fn from(value: entity_state::EntityStateSnapshotError) -> Self {
        Self::Snapshot(value)
    }
}

impl From<MechanicsSnapshotError> for SessionSaveError {
    fn from(value: MechanicsSnapshotError) -> Self {
        Self::MechanicsSnapshot(value)
    }
}

impl From<ComponentRegistrationError> for SessionSaveError {
    fn from(value: ComponentRegistrationError) -> Self {
        Self::ComponentRegistration(value)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct D20SessionSave {
    schema_version: u32,
    engine_revision: String,
    ruleset_fingerprint: String,
    seed: u64,
    next_roll: u64,
    current_turn: u64,
    entity_state: serde_json::Value,
}

fn validate_character_seed(
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

fn spend_activation_costs(
    component: &ActivationBudgetsComponent,
    entity: EntityId,
    costs: &[ActivationCostDefinition],
) -> Result<ActivationBudgetsComponent, D20SessionError> {
    let mut after = component.clone();
    for cost in costs {
        let available = after.current(&cost.budget).unwrap_or(0);
        after = after.spend(&cost.budget, cost.amount).ok_or_else(|| {
            D20SessionError::ActivationBudgetUnavailable {
                entity,
                budget: cost.budget.clone(),
                required: cost.amount,
                available,
            }
        })?;
    }
    Ok(after)
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

fn request_source(operation: &OperationId, label: &str) -> SourceInstanceIdentity {
    SourceInstanceIdentity::Request {
        operation: operation.clone(),
        instance: SourceInstanceId::parse(label).expect("fixed request source identity is valid"),
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_or_refresh_scheduled_effect(
    state: &mut EntityState,
    rules: &D20Ruleset,
    entity: EntityId,
    operation: &OperationId,
    effect: &D20Id,
    proposed_instance: EffectInstanceId,
    source_label: &str,
    expected_schedule_revision: &ComponentRevision,
    expires_at_turn: u64,
) -> Result<EffectMutationReceipt, D20SessionError> {
    let mechanics_definition = mechanics_effect_id(effect);
    let existing = state
        .component::<ActiveEffectsComponent>(entity)?
        .ok_or(D20SessionError::MissingComponent {
            entity,
            component: ActiveEffectsComponent::LABEL,
        })?
        .effects()
        .iter()
        .find(|active| active.definition() == &mechanics_definition)
        .map(|active| active.instance().clone());
    let refreshing = existing.is_some();
    let provenance = request_source(operation, source_label);
    let (receipt, scheduled_instance) = if let Some(existing) = existing {
        (
            EffectService::refresh(
                state,
                rules.mechanics(),
                EffectRefreshRequest {
                    operation: operation.clone(),
                    entity,
                    instance: existing.clone(),
                    provenance,
                    stacks: 1,
                    expected_revision: None,
                },
            )?,
            existing,
        )
    } else {
        (
            EffectService::apply(
                state,
                rules.mechanics(),
                EffectApplyRequest {
                    operation: operation.clone(),
                    entity,
                    instance: proposed_instance.clone(),
                    definition: mechanics_definition,
                    provenance,
                    stacks: 1,
                    expected_revision: None,
                },
            )?,
            proposed_instance,
        )
    };

    let schedule = state
        .component::<ScheduledEffectsComponent>(entity)?
        .ok_or(D20SessionError::MissingComponent {
            entity,
            component: ScheduledEffectsComponent::LABEL,
        })?;
    let schedule = if refreshing {
        schedule
            .without_instances(std::slice::from_ref(&scheduled_instance))?
            .with_added(ScheduledEffect::new(
                scheduled_instance,
                effect.clone(),
                expires_at_turn,
            ))?
    } else {
        schedule.with_added(ScheduledEffect::new(
            scheduled_instance,
            effect.clone(),
            expires_at_turn,
        ))?
    };
    EntityAuthoringService.replace_component(
        state,
        expected_schedule_revision.clone(),
        entity,
        schedule,
    )?;
    Ok(receipt)
}

fn ensure_component_revision(
    _state: &EntityState,
    expected: &ComponentRevision,
    actual: ComponentRevision,
) -> Result<(), D20SessionError> {
    if expected.revision() != actual.revision()
        || expected.entity() != actual.entity()
        || expected.component() != actual.component()
    {
        return Err(D20SessionError::StalePreview {
            reason: "an observed d20 component changed",
        });
    }
    Ok(())
}

fn mechanics_revision(
    state: &EntityState,
    observed: &ObservedComponentRevision,
) -> Result<u64, ComponentAccessError> {
    let revision = match observed.component {
        MechanicsComponentKind::Stats => state
            .component_revision::<StatsComponent>(observed.entity)?
            .revision(),
        MechanicsComponentKind::Tracks => state
            .component_revision::<TracksComponent>(observed.entity)?
            .revision(),
        MechanicsComponentKind::IntrinsicSources => state
            .component_revision::<IntrinsicSourcesComponent>(observed.entity)?
            .revision(),
        MechanicsComponentKind::ActiveEffects => state
            .component_revision::<ActiveEffectsComponent>(observed.entity)?
            .revision(),
        MechanicsComponentKind::Inventory => state
            .component_revision::<gameplay_mechanics::InventoryComponent>(observed.entity)?
            .revision(),
        MechanicsComponentKind::Item => state
            .component_revision::<ItemComponent>(observed.entity)?
            .revision(),
        MechanicsComponentKind::Equipment => state
            .component_revision::<EquipmentComponent>(observed.entity)?
            .revision(),
    };
    Ok(revision)
}

fn validate_restored_d20_state(
    state: &EntityState,
    rules: &D20Ruleset,
) -> Result<(), SessionSaveError> {
    for (entity, abilities) in state
        .components::<AbilityScoresComponent>()
        .map_err(D20SessionError::from)
        .map_err(SessionSaveError::InvalidState)?
    {
        let resources = state
            .component::<ActionResourcesComponent>(entity)
            .map_err(D20SessionError::from)
            .map_err(SessionSaveError::InvalidState)?
            .ok_or({
                SessionSaveError::InvalidState(D20SessionError::MissingComponent {
                    entity,
                    component: ActionResourcesComponent::LABEL,
                })
            })?;
        state
            .component::<ScheduledEffectsComponent>(entity)
            .map_err(D20SessionError::from)
            .map_err(SessionSaveError::InvalidState)?
            .ok_or({
                SessionSaveError::InvalidState(D20SessionError::MissingComponent {
                    entity,
                    component: ScheduledEffectsComponent::LABEL,
                })
            })?;
        state
            .component::<ActiveEffectsComponent>(entity)
            .map_err(D20SessionError::from)
            .map_err(SessionSaveError::InvalidState)?
            .ok_or({
                SessionSaveError::InvalidState(D20SessionError::MissingComponent {
                    entity,
                    component: ActiveEffectsComponent::LABEL,
                })
            })?;
        let budgets = state
            .component::<ActivationBudgetsComponent>(entity)
            .map_err(D20SessionError::from)
            .map_err(SessionSaveError::InvalidState)?
            .ok_or({
                SessionSaveError::InvalidState(D20SessionError::MissingComponent {
                    entity,
                    component: ActivationBudgetsComponent::LABEL,
                })
            })?;
        let mut expected_budgets = rules
            .activation_budgets()
            .map(|definition| (definition.id.clone(), definition.initial))
            .collect::<std::collections::BTreeMap<_, _>>();
        for budget in budgets.budgets() {
            let Some(initial) = expected_budgets.remove(budget.id()) else {
                return Err(SessionSaveError::InvalidState(
                    D20SessionError::ActivationBudgetUnavailable {
                        entity,
                        budget: budget.id().clone(),
                        required: budget.current(),
                        available: 0,
                    },
                ));
            };
            if budget.current() > initial {
                return Err(SessionSaveError::InvalidState(
                    D20SessionError::ActivationBudgetUnavailable {
                        entity,
                        budget: budget.id().clone(),
                        required: budget.current(),
                        available: initial,
                    },
                ));
            }
        }
        if let Some((budget, _)) = expected_budgets.first_key_value() {
            return Err(SessionSaveError::InvalidState(
                D20SessionError::ActivationBudgetUnavailable {
                    entity,
                    budget: budget.clone(),
                    required: 0,
                    available: 0,
                },
            ));
        }
        validate_character_seed(
            rules,
            &CharacterSeed {
                entity,
                name: String::new(),
                vitality: 0,
                abilities: abilities.scores().to_vec(),
                resources: resources.resources().to_vec(),
                affinities: vec![],
            },
        )
        .map_err(SessionSaveError::InvalidState)?;
    }
    for (entity, _) in state
        .components::<ActionResourcesComponent>()
        .map_err(D20SessionError::from)
        .map_err(SessionSaveError::InvalidState)?
    {
        if !state
            .has_component::<AbilityScoresComponent>(entity)
            .map_err(D20SessionError::from)
            .map_err(SessionSaveError::InvalidState)?
        {
            return Err(SessionSaveError::InvalidState(
                D20SessionError::MissingComponent {
                    entity,
                    component: AbilityScoresComponent::LABEL,
                },
            ));
        }
    }
    for (entity, participation) in state
        .components::<EncounterParticipationComponent>()
        .map_err(D20SessionError::from)
        .map_err(SessionSaveError::InvalidState)?
    {
        if !state
            .has_component::<AbilityScoresComponent>(entity)
            .map_err(D20SessionError::from)
            .map_err(SessionSaveError::InvalidState)?
        {
            return Err(SessionSaveError::InvalidState(
                D20SessionError::InvalidEncounterParticipation(format!(
                    "entity {entity} participates in {} without character facts",
                    participation.encounter()
                )),
            ));
        }
        if rules.encounter(participation.encounter()).is_none() {
            return Err(SessionSaveError::InvalidState(
                D20SessionError::InvalidEncounterParticipation(format!(
                    "entity {entity} references unknown encounter {}",
                    participation.encounter()
                )),
            ));
        }
    }
    for (entity, schedule) in state
        .components::<ScheduledEffectsComponent>()
        .map_err(D20SessionError::from)
        .map_err(SessionSaveError::InvalidState)?
    {
        if !state
            .has_component::<AbilityScoresComponent>(entity)
            .map_err(D20SessionError::from)
            .map_err(SessionSaveError::InvalidState)?
        {
            return Err(SessionSaveError::InvalidState(
                D20SessionError::MissingComponent {
                    entity,
                    component: AbilityScoresComponent::LABEL,
                },
            ));
        }
        let active = state
            .component::<ActiveEffectsComponent>(entity)
            .map_err(D20SessionError::from)
            .map_err(SessionSaveError::InvalidState)?
            .ok_or({
                SessionSaveError::InvalidState(D20SessionError::MissingComponent {
                    entity,
                    component: ActiveEffectsComponent::LABEL,
                })
            })?;
        for scheduled in schedule.effects() {
            if rules.effect(scheduled.definition()).is_none()
                || !active
                    .effects()
                    .iter()
                    .any(|effect| effect.instance() == scheduled.instance())
            {
                return Err(SessionSaveError::InvalidState(
                    D20SessionError::MissingEffectInstance(scheduled.definition().clone()),
                ));
            }
        }
        for effect in active.effects() {
            let Some(scheduled) = schedule
                .effects()
                .iter()
                .find(|scheduled| scheduled.instance() == effect.instance())
            else {
                return Err(SessionSaveError::InvalidState(
                    D20SessionError::UnscheduledActiveEffect {
                        entity,
                        instance: effect.instance().to_string(),
                    },
                ));
            };
            if mechanics_effect_id(scheduled.definition()) != *effect.definition() {
                return Err(SessionSaveError::InvalidState(
                    D20SessionError::UnscheduledActiveEffect {
                        entity,
                        instance: effect.instance().to_string(),
                    },
                ));
            }
        }
    }
    Ok(())
}

fn scalar(value: i64) -> MechanicsScalar {
    MechanicsScalar::new(value).expect("validated d20 values fit mechanics scalar")
}
