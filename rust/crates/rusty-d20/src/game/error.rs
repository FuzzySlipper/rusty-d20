use super::*;

#[derive(Debug)]
pub enum GameRuntimeError {
    NoEncounter,
    ReactionPromptCannotBeSaved,
    StaleCommand(String),
    InvalidCommand(String),
    InvalidEquipmentSlot { requested: String, required: String },
    InvalidContainment(String),
    WrongPhase(String),
    InvalidState(String),
    InvalidSave(String),
    Catalog(String),
    CompositionFingerprintMismatch { expected: String, actual: String },
    UnsupportedSaveSchema { actual: u32 },
    CounterOverflow,
    D20Identity(crate::D20IdentityError),
    Compile(D20CompileError),
    Session(D20SessionError),
    Save(SessionSaveError),
    ComponentAccess(ComponentAccessError),
    Json(serde_json::Error),
}

impl GameRuntimeError {
    pub fn api_error(&self) -> ApiErrorDto {
        let (kind, retryable) = match self {
            Self::StaleCommand(_) => (ApiErrorKindDto::Stale, true),
            Self::NoEncounter => (ApiErrorKindDto::NotFound, false),
            Self::InvalidEquipmentSlot { .. } => (ApiErrorKindDto::InvalidSlot, false),
            Self::InvalidContainment(_) => (ApiErrorKindDto::Containment, false),
            Self::WrongPhase(_) => (ApiErrorKindDto::Phase, false),
            Self::Session(D20SessionError::Mechanics(error)) => {
                let kind = mechanics_api_error_kind(error);
                (kind, kind == ApiErrorKindDto::Stale)
            }
            Self::Session(D20SessionError::StalePreview { .. }) => (ApiErrorKindDto::Stale, true),
            Self::Session(
                D20SessionError::RequiredImplementNotEquipped { .. }
                | D20SessionError::ActionForbidden { .. }
                | D20SessionError::MovementForbidden { .. },
            ) => (ApiErrorKindDto::Invalid, false),
            Self::ReactionPromptCannotBeSaved | Self::InvalidCommand(_) | Self::D20Identity(_) => {
                (ApiErrorKindDto::Invalid, false)
            }
            Self::InvalidSave(_)
            | Self::CompositionFingerprintMismatch { .. }
            | Self::UnsupportedSaveSchema { .. }
            | Self::Save(_) => (ApiErrorKindDto::Persistence, false),
            _ => (ApiErrorKindDto::Internal, false),
        };
        ApiErrorDto {
            kind,
            message: self.to_string(),
            retryable,
        }
    }
}

impl std::fmt::Display for GameRuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoEncounter => write!(formatter, "no encounter is active"),
            Self::ReactionPromptCannotBeSaved => {
                write!(formatter, "choose or decline the reaction before saving")
            }
            Self::StaleCommand(message)
            | Self::InvalidCommand(message)
            | Self::InvalidContainment(message)
            | Self::WrongPhase(message)
            | Self::InvalidState(message)
            | Self::InvalidSave(message)
            | Self::Catalog(message) => formatter.write_str(message),
            Self::CompositionFingerprintMismatch { expected, actual } => write!(
                formatter,
                "save composition fingerprint mismatch: expected {expected}, found {actual}"
            ),
            Self::InvalidEquipmentSlot {
                requested,
                required,
            } => write!(
                formatter,
                "equipment slot {requested} is invalid; this item requires {required}"
            ),
            _ => write!(formatter, "Rusty D20 product operation failed: {self:?}"),
        }
    }
}

fn mechanics_api_error_kind(error: &MechanicsError) -> ApiErrorKindDto {
    match error {
        MechanicsError::StaleComponentRevision { .. } => ApiErrorKindDto::Stale,
        MechanicsError::UnknownEquipmentSlot { .. }
        | MechanicsError::EquipmentSlotOccupied { .. }
        | MechanicsError::EquipmentSlotEmpty { .. }
        | MechanicsError::EquipmentSlotCountMismatch { .. }
        | MechanicsError::EquipmentSlotClassificationMismatch { .. }
        | MechanicsError::EquipmentExclusivityConflict { .. } => ApiErrorKindDto::InvalidSlot,
        MechanicsError::InventoryCapacityExceeded { .. }
        | MechanicsError::InventoryContainmentQuotaExceeded { .. }
        | MechanicsError::CapacityArithmeticOverflow { .. } => ApiErrorKindDto::Capacity,
        MechanicsError::ItemNotContained { .. }
        | MechanicsError::ItemEquipped { .. }
        | MechanicsError::InventoryOwnerConflict { .. } => ApiErrorKindDto::Containment,
        MechanicsError::EquipmentWouldInvalidateTrack { .. } => ApiErrorKindDto::TrackBound,
        _ => ApiErrorKindDto::Invalid,
    }
}

impl std::error::Error for GameRuntimeError {}

impl From<crate::D20IdentityError> for GameRuntimeError {
    fn from(value: crate::D20IdentityError) -> Self {
        Self::D20Identity(value)
    }
}

impl From<D20CompileError> for GameRuntimeError {
    fn from(value: D20CompileError) -> Self {
        Self::Compile(value)
    }
}

impl From<D20SessionError> for GameRuntimeError {
    fn from(value: D20SessionError) -> Self {
        Self::Session(value)
    }
}

impl From<SessionSaveError> for GameRuntimeError {
    fn from(value: SessionSaveError) -> Self {
        Self::Save(value)
    }
}

impl From<ComponentAccessError> for GameRuntimeError {
    fn from(value: ComponentAccessError) -> Self {
        Self::ComponentAccess(value)
    }
}

impl From<serde_json::Error> for GameRuntimeError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
