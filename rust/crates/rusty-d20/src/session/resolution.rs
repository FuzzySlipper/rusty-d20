use super::*;

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

impl D20Session {
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
                ..
            } => ResolvedActionDefinition {
                ability: ability.clone(),
                defense: defense.clone(),
                damage: damage.clone(),
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
        let (d20, damage_rolls) = self.action_roll(&request.preview.damage)?;
        let total = i32::from(d20) + i32::from(request.preview.ability_modifier);
        let hit = i64::from(total) >= request.preview.defense.value.get();

        let mut rolled_damage = 0_u32;
        if hit {
            for result in damage_rolls {
                rolled_damage = rolled_damage
                    .checked_add(u32::from(result))
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

    fn action_roll(&self, damage: &DamageDefinition) -> Result<(u8, Vec<u16>), D20SessionError> {
        match &self.roll_source {
            RollSourceConfig::Seeded { seed } => {
                let mut rng = ScopedRng::new(
                    RngSeed::new(*seed),
                    &format!("d20-action-roll/{}", self.next_roll),
                );
                let d20 =
                    u8::try_from(rng.next_bounded_u32(20).expect("fixed nonzero d20 bound") + 1)
                        .expect("d20 roll fits u8");
                let damage = (0..damage.dice)
                    .map(|_| {
                        u16::try_from(
                            rng.next_bounded_u32(u32::from(damage.sides))
                                .expect("compiled damage die has a nonzero bound")
                                + 1,
                        )
                        .expect("compiled damage die result fits u16")
                    })
                    .collect();
                Ok((d20, damage))
            }
            RollSourceConfig::Static { rolls } => {
                let index = usize::try_from(self.next_roll).map_err(|_| {
                    D20SessionError::StaticRollsExhausted {
                        index: self.next_roll,
                        available: rolls.len(),
                    }
                })?;
                let roll = rolls
                    .get(index)
                    .ok_or(D20SessionError::StaticRollsExhausted {
                        index: self.next_roll,
                        available: rolls.len(),
                    })?;
                if roll.damage.len() != usize::from(damage.dice)
                    || roll.damage.iter().any(|result| *result > damage.sides)
                {
                    return Err(D20SessionError::StaticRollMismatch {
                        index: self.next_roll,
                        expected_dice: damage.dice,
                        expected_sides: damage.sides,
                    });
                }
                Ok((roll.d20, roll.damage.clone()))
            }
        }
    }

    fn ensure_fresh(&self, preview: &ActionPreview) -> Result<(), D20SessionError> {
        if preview.turn != self.current_turn || preview.roll_index != self.next_roll {
            return Err(D20SessionError::StalePreview {
                reason: "turn or roll-source position changed",
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
