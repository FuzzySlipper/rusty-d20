use super::runtime::validate_character_seed;
use super::*;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct D20SessionSave {
    schema_version: u32,
    ruleset_fingerprint: String,
    roll_source: RollSourceConfig,
    next_roll: u64,
    current_turn: u64,
    entity_state: serde_json::Value,
}

impl D20Session {
    pub fn encode_save(&self) -> Result<String, SessionSaveError> {
        let entity_state = serde_json::from_str(&encode_snapshot(&self.entities)?)?;
        Ok(serde_json::to_string_pretty(&D20SessionSave {
            schema_version: D20_SAVE_SCHEMA_VERSION,
            ruleset_fingerprint: self.rules.fingerprint().to_owned(),
            roll_source: self.roll_source.clone(),
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
        save.roll_source
            .validate()
            .map_err(SessionSaveError::InvalidState)?;
        if let RollSourceConfig::Static { rolls } = &save.roll_source {
            let available = u64::try_from(rolls.len()).expect("static roll bound fits u64");
            if save.next_roll > available {
                return Err(SessionSaveError::InvalidState(
                    D20SessionError::InvalidRollSource(format!(
                        "static roll position {} exceeds tape length {available}",
                        save.next_roll
                    )),
                ));
            }
        }
        Ok(Self {
            rules,
            entities,
            roll_source: save.roll_source,
            next_roll: save.next_roll,
            current_turn: save.current_turn,
        })
    }
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
