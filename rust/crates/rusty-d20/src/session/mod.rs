mod error;
mod persistence;
mod resolution;
mod runtime;
mod seed;

use rusty_engine::core_ids::EntityId;
use rusty_engine::entity_state::{
    encode_snapshot, ComponentAccessError, ComponentRegistrationError, ComponentRevision,
    EntityAuthoringError, EntityAuthoringService, EntityComponent, EntityDefinition,
    EntityDefinitionError, EntityState,
};
use rusty_engine::gameplay_mechanics::{
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
use rusty_engine::svc_rng::{RngSeed, ScopedRng};
use serde::{Deserialize, Serialize};

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
    ScheduledEffectsComponent, TacticalPosition, MAX_D20_DAMAGE_DICE, MAX_D20_DAMAGE_DIE_SIDES,
};

const D20_SAVE_SCHEMA_VERSION: u32 = 6;

pub use error::{D20SessionError, SessionSaveError};
pub use resolution::{
    ability_modifier, ActionPreview, ActionReceipt, AdvanceTurnReceipt, ApplyActionRequest,
    ReactionOption, ReactionReceipt,
};
pub use runtime::D20Session;
pub use seed::{
    AffinitySeed, ArmorItemSeed, CharacterSeed, DamageAffinity, EncounterParticipationSeed,
    EquipmentItemSeed, InventorySeed, RollSourceConfig, StaticActionRoll, StorageSeed,
    DEFAULT_ROLL_SEED, MAX_STATIC_ACTION_ROLLS,
};

fn request_source(operation: &OperationId, label: &str) -> SourceInstanceIdentity {
    SourceInstanceIdentity::Request {
        operation: operation.clone(),
        instance: SourceInstanceId::parse(label).expect("fixed request source identity is valid"),
    }
}

fn scalar(value: i64) -> MechanicsScalar {
    MechanicsScalar::new(value).expect("validated d20 values fit mechanics scalar")
}
