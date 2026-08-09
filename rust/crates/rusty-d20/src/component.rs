use rusty_engine::entity_state::{
    ComponentCodec, ComponentRegistration, ComponentRegistrationError, ComponentRegistry,
    ComponentTypeId, EntityComponent,
};
use rusty_engine::gameplay_mechanics::{gameplay_component_registry, EffectInstanceId};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::D20Id;

pub const ABILITY_SCORES_COMPONENT_TYPE_ID: &str = "rusty-d20.ability-scores";
pub const ACTION_RESOURCES_COMPONENT_TYPE_ID: &str = "rusty-d20.action-resources";
pub const ACTIVATION_BUDGETS_COMPONENT_TYPE_ID: &str = "rusty-d20.activation-budgets";
pub const ENCOUNTER_PARTICIPATION_COMPONENT_TYPE_ID: &str = "rusty-d20.encounter-participation";
pub const SCHEDULED_EFFECTS_COMPONENT_TYPE_ID: &str = "rusty-d20.scheduled-effects";

pub const ABILITY_SCORES_COMPONENT_CODEC_ID: &str = "rusty-d20.ability-scores-json";
pub const ACTION_RESOURCES_COMPONENT_CODEC_ID: &str = "rusty-d20.action-resources-json";
pub const ACTIVATION_BUDGETS_COMPONENT_CODEC_ID: &str = "rusty-d20.activation-budgets-json";
pub const ENCOUNTER_PARTICIPATION_COMPONENT_CODEC_ID: &str =
    "rusty-d20.encounter-participation-json";
pub const SCHEDULED_EFFECTS_COMPONENT_CODEC_ID: &str = "rusty-d20.scheduled-effects-json";

pub const ABILITY_SCORES_COMPONENT_CODEC_VERSION: u32 = 1;
pub const ACTION_RESOURCES_COMPONENT_CODEC_VERSION: u32 = 1;
pub const ACTIVATION_BUDGETS_COMPONENT_CODEC_VERSION: u32 = 1;
pub const ENCOUNTER_PARTICIPATION_COMPONENT_CODEC_VERSION: u32 = 2;
pub const SCHEDULED_EFFECTS_COMPONENT_CODEC_VERSION: u32 = 1;

pub const MAX_D20_ABILITIES_PER_ENTITY: usize = 64;
pub const MAX_D20_RESOURCES_PER_ENTITY: usize = 64;
pub const MAX_D20_ACTIVATION_BUDGETS_PER_ENTITY: usize = 64;
pub const MAX_D20_SCHEDULED_EFFECTS_PER_ENTITY: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AbilityScore {
    id: D20Id,
    score: i16,
}

impl AbilityScore {
    pub const fn new(id: D20Id, score: i16) -> Self {
        Self { id, score }
    }

    pub const fn id(&self) -> &D20Id {
        &self.id
    }

    pub const fn score(&self) -> i16 {
        self.score
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AbilityScoresComponent {
    scores: Vec<AbilityScore>,
}

impl AbilityScoresComponent {
    pub const LABEL: &'static str = "AbilityScoresComponent";

    pub fn new(mut scores: Vec<AbilityScore>) -> Result<Self, D20ComponentDataError> {
        scores.sort_by(|left, right| left.id.cmp(&right.id));
        validate_unique(
            &scores,
            MAX_D20_ABILITIES_PER_ENTITY,
            "abilities",
            |entry| entry.id.as_str(),
        )?;
        Ok(Self { scores })
    }

    pub fn scores(&self) -> &[AbilityScore] {
        &self.scores
    }

    pub fn score(&self, id: &D20Id) -> Option<i16> {
        self.scores
            .binary_search_by(|entry| entry.id.cmp(id))
            .ok()
            .map(|index| self.scores[index].score)
    }
}

impl EntityComponent for AbilityScoresComponent {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ActionResource {
    id: D20Id,
    current: u16,
}

impl ActionResource {
    pub const fn new(id: D20Id, current: u16) -> Self {
        Self { id, current }
    }

    pub const fn id(&self) -> &D20Id {
        &self.id
    }

    pub const fn current(&self) -> u16 {
        self.current
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ActionResourcesComponent {
    resources: Vec<ActionResource>,
}

impl ActionResourcesComponent {
    pub const LABEL: &'static str = "ActionResourcesComponent";

    pub fn new(mut resources: Vec<ActionResource>) -> Result<Self, D20ComponentDataError> {
        resources.sort_by(|left, right| left.id.cmp(&right.id));
        validate_unique(
            &resources,
            MAX_D20_RESOURCES_PER_ENTITY,
            "resources",
            |entry| entry.id.as_str(),
        )?;
        Ok(Self { resources })
    }

    pub fn resources(&self) -> &[ActionResource] {
        &self.resources
    }

    pub fn current(&self, id: &D20Id) -> Option<u16> {
        self.resources
            .binary_search_by(|entry| entry.id.cmp(id))
            .ok()
            .map(|index| self.resources[index].current)
    }

    pub(crate) fn spend(&self, id: &D20Id, amount: u16) -> Option<Self> {
        let index = self
            .resources
            .binary_search_by(|entry| entry.id.cmp(id))
            .ok()?;
        let after = self.resources[index].current.checked_sub(amount)?;
        let mut candidate = self.clone();
        candidate.resources[index].current = after;
        Some(candidate)
    }
}

impl EntityComponent for ActionResourcesComponent {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ActivationBudget {
    id: D20Id,
    current: u16,
}

impl ActivationBudget {
    pub const fn new(id: D20Id, current: u16) -> Self {
        Self { id, current }
    }

    pub const fn id(&self) -> &D20Id {
        &self.id
    }

    pub const fn current(&self) -> u16 {
        self.current
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ActivationBudgetsComponent {
    budgets: Vec<ActivationBudget>,
}

impl ActivationBudgetsComponent {
    pub const LABEL: &'static str = "ActivationBudgetsComponent";

    pub fn new(mut budgets: Vec<ActivationBudget>) -> Result<Self, D20ComponentDataError> {
        budgets.sort_by(|left, right| left.id.cmp(&right.id));
        validate_unique(
            &budgets,
            MAX_D20_ACTIVATION_BUDGETS_PER_ENTITY,
            "activationBudgets",
            |entry| entry.id.as_str(),
        )?;
        Ok(Self { budgets })
    }

    pub fn budgets(&self) -> &[ActivationBudget] {
        &self.budgets
    }

    pub fn current(&self, id: &D20Id) -> Option<u16> {
        self.budgets
            .binary_search_by(|entry| entry.id.cmp(id))
            .ok()
            .map(|index| self.budgets[index].current)
    }

    pub(crate) fn spend(&self, id: &D20Id, amount: u16) -> Option<Self> {
        let index = self
            .budgets
            .binary_search_by(|entry| entry.id.cmp(id))
            .ok()?;
        let after = self.budgets[index].current.checked_sub(amount)?;
        let mut candidate = self.clone();
        candidate.budgets[index].current = after;
        Some(candidate)
    }
}

impl EntityComponent for ActivationBudgetsComponent {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EncounterFaction {
    Party,
    Opposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TacticalPosition {
    x: u16,
    y: u16,
}

impl TacticalPosition {
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }

    pub const fn x(self) -> u16 {
        self.x
    }

    pub const fn y(self) -> u16 {
        self.y
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EncounterParticipationComponent {
    encounter: D20Id,
    faction: EncounterFaction,
    initiative: i16,
    position: TacticalPosition,
}

impl EncounterParticipationComponent {
    pub const LABEL: &'static str = "EncounterParticipationComponent";

    pub const fn new(
        encounter: D20Id,
        faction: EncounterFaction,
        initiative: i16,
        position: TacticalPosition,
    ) -> Self {
        Self {
            encounter,
            faction,
            initiative,
            position,
        }
    }

    pub const fn encounter(&self) -> &D20Id {
        &self.encounter
    }

    pub const fn faction(&self) -> EncounterFaction {
        self.faction
    }

    pub const fn initiative(&self) -> i16 {
        self.initiative
    }

    pub const fn position(&self) -> TacticalPosition {
        self.position
    }

    pub(crate) fn with_position(&self, position: TacticalPosition) -> Self {
        Self {
            position,
            ..self.clone()
        }
    }
}

impl EntityComponent for EncounterParticipationComponent {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScheduledEffect {
    instance: EffectInstanceId,
    definition: D20Id,
    expires_at_turn: u64,
}

impl ScheduledEffect {
    pub const fn new(instance: EffectInstanceId, definition: D20Id, expires_at_turn: u64) -> Self {
        Self {
            instance,
            definition,
            expires_at_turn,
        }
    }

    pub const fn instance(&self) -> &EffectInstanceId {
        &self.instance
    }

    pub const fn definition(&self) -> &D20Id {
        &self.definition
    }

    pub const fn expires_at_turn(&self) -> u64 {
        self.expires_at_turn
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScheduledEffectsComponent {
    effects: Vec<ScheduledEffect>,
}

impl ScheduledEffectsComponent {
    pub const LABEL: &'static str = "ScheduledEffectsComponent";

    pub fn new(mut effects: Vec<ScheduledEffect>) -> Result<Self, D20ComponentDataError> {
        effects.sort_by(|left, right| left.instance.cmp(&right.instance));
        validate_unique(
            &effects,
            MAX_D20_SCHEDULED_EFFECTS_PER_ENTITY,
            "scheduledEffects",
            |entry| entry.instance.as_str(),
        )?;
        Ok(Self { effects })
    }

    pub fn effects(&self) -> &[ScheduledEffect] {
        &self.effects
    }

    pub(crate) fn with_added(
        &self,
        effect: ScheduledEffect,
    ) -> Result<Self, D20ComponentDataError> {
        let mut effects = self.effects.clone();
        effects.push(effect);
        Self::new(effects)
    }

    pub(crate) fn without_instances(
        &self,
        expired: &[EffectInstanceId],
    ) -> Result<Self, D20ComponentDataError> {
        Self::new(
            self.effects
                .iter()
                .filter(|effect| !expired.contains(&effect.instance))
                .cloned()
                .collect(),
        )
    }
}

impl EntityComponent for ScheduledEffectsComponent {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum D20ComponentDataError {
    QuotaExceeded {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    DuplicateIdentity {
        field: &'static str,
        identity: String,
    },
}

impl std::fmt::Display for D20ComponentDataError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid d20 component data: {self:?}")
    }
}

impl std::error::Error for D20ComponentDataError {}

pub fn d20_component_registry() -> Result<ComponentRegistry, ComponentRegistrationError> {
    let mut registry = gameplay_component_registry()?;
    register_d20_components(&mut registry)?;
    Ok(registry)
}

pub fn register_d20_components(
    registry: &mut ComponentRegistry,
) -> Result<(), ComponentRegistrationError> {
    let mut staged = registry.clone();
    staged.register(durable_registration::<AbilityScoresComponent>(
        ABILITY_SCORES_COMPONENT_TYPE_ID,
        ABILITY_SCORES_COMPONENT_CODEC_ID,
        ABILITY_SCORES_COMPONENT_CODEC_VERSION,
        |value| {
            validate_canonical(value.scores(), MAX_D20_ABILITIES_PER_ENTITY, |entry| {
                entry.id.as_str()
            })
        },
    ))?;
    staged.register(durable_registration::<ActionResourcesComponent>(
        ACTION_RESOURCES_COMPONENT_TYPE_ID,
        ACTION_RESOURCES_COMPONENT_CODEC_ID,
        ACTION_RESOURCES_COMPONENT_CODEC_VERSION,
        |value| {
            validate_canonical(value.resources(), MAX_D20_RESOURCES_PER_ENTITY, |entry| {
                entry.id.as_str()
            })
        },
    ))?;
    staged.register(durable_registration::<ActivationBudgetsComponent>(
        ACTIVATION_BUDGETS_COMPONENT_TYPE_ID,
        ACTIVATION_BUDGETS_COMPONENT_CODEC_ID,
        ACTIVATION_BUDGETS_COMPONENT_CODEC_VERSION,
        |value| {
            validate_canonical(
                value.budgets(),
                MAX_D20_ACTIVATION_BUDGETS_PER_ENTITY,
                |entry| entry.id.as_str(),
            )
        },
    ))?;
    staged.register(durable_registration::<EncounterParticipationComponent>(
        ENCOUNTER_PARTICIPATION_COMPONENT_TYPE_ID,
        ENCOUNTER_PARTICIPATION_COMPONENT_CODEC_ID,
        ENCOUNTER_PARTICIPATION_COMPONENT_CODEC_VERSION,
        |_| Ok(()),
    ))?;
    staged.register(durable_registration::<ScheduledEffectsComponent>(
        SCHEDULED_EFFECTS_COMPONENT_TYPE_ID,
        SCHEDULED_EFFECTS_COMPONENT_CODEC_ID,
        SCHEDULED_EFFECTS_COMPONENT_CODEC_VERSION,
        |value| {
            validate_canonical(
                value.effects(),
                MAX_D20_SCHEDULED_EFFECTS_PER_ENTITY,
                |entry| entry.instance.as_str(),
            )
        },
    ))?;
    *registry = staged;
    Ok(())
}

fn durable_registration<T>(
    type_id: &'static str,
    codec_id: &'static str,
    codec_version: u32,
    validator: fn(&T) -> Result<(), String>,
) -> ComponentRegistration<T>
where
    T: EntityComponent + Serialize + DeserializeOwned,
{
    let codec = ComponentCodec::new(
        codec_id,
        codec_version,
        |value| serde_json::to_value(value).expect("d20 component codec is infallible"),
        |value| serde_json::from_value(value).map_err(|error| error.to_string()),
    )
    .expect("fixed d20 component codec is valid");
    ComponentRegistration::durable(
        ComponentTypeId::parse(type_id).expect("fixed d20 component identity is valid"),
        codec,
    )
    .with_validator(validator)
}

fn validate_unique<T>(
    values: &[T],
    maximum: usize,
    field: &'static str,
    identity: impl Fn(&T) -> &str,
) -> Result<(), D20ComponentDataError> {
    if values.len() > maximum {
        return Err(D20ComponentDataError::QuotaExceeded {
            field,
            actual: values.len(),
            maximum,
        });
    }
    for pair in values.windows(2) {
        if identity(&pair[0]) == identity(&pair[1]) {
            return Err(D20ComponentDataError::DuplicateIdentity {
                field,
                identity: identity(&pair[0]).to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_canonical<T>(
    values: &[T],
    maximum: usize,
    identity: impl Fn(&T) -> &str,
) -> Result<(), String> {
    if values.len() > maximum {
        return Err(format!("component entry count exceeds {maximum}"));
    }
    if values
        .windows(2)
        .any(|pair| identity(&pair[0]) >= identity(&pair[1]))
    {
        return Err("component entries are not in strict canonical order".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusty_engine::entity_state::{ComponentPersistence, EntityState};

    use super::*;

    #[test]
    fn d20_registrations_are_durable_stable_and_fail_atomic() {
        let mut registry = d20_component_registry().unwrap();
        let before = format!("{registry:?}");
        assert!(register_d20_components(&mut registry).is_err());
        assert_eq!(format!("{registry:?}"), before);

        let state = EntityState::with_registry(registry);
        let inspection = state.component_inspection();
        for (type_id, version) in [
            (ABILITY_SCORES_COMPONENT_TYPE_ID, 1),
            (ACTION_RESOURCES_COMPONENT_TYPE_ID, 1),
            (ACTIVATION_BUDGETS_COMPONENT_TYPE_ID, 1),
            (ENCOUNTER_PARTICIPATION_COMPONENT_TYPE_ID, 2),
            (SCHEDULED_EFFECTS_COMPONENT_TYPE_ID, 1),
        ] {
            let kind = inspection
                .kinds
                .iter()
                .find(|kind| kind.type_id.as_str() == type_id)
                .unwrap();
            assert_eq!(kind.persistence, ComponentPersistence::Durable { version });
        }
    }
}
