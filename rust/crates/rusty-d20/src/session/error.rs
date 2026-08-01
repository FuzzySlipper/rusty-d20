use super::*;

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
    MovementForbidden {
        entity: EntityId,
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
    InvalidRollSource(String),
    StaticRollsExhausted {
        index: u64,
        available: usize,
    },
    StaticRollMismatch {
        index: u64,
        expected_dice: u8,
        expected_sides: u16,
    },
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
