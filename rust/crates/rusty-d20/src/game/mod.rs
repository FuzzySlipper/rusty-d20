mod commands;
mod content;
mod dto;
mod encounter;
mod error;
mod exploration;
mod outcome;
mod persistence;
mod projection;
mod state;
mod support;
mod tactical;

use std::collections::{BTreeMap, BTreeSet};

use rusty_engine::core_ids::EntityId;
use rusty_engine::entity_state::ComponentAccessError;
use rusty_engine::gameplay_mechanics::{
    ActiveEffectsComponent, DecisionOutcome, EffectInstanceId, EquipmentComponent,
    InventoryComponent, ItemComponent, MechanicsError, OperationId, ResponseDecisionKind,
    SourceInstanceIdentity, StatContribution, StatService, TracksComponent,
};
use serde::{Deserialize, Serialize};

use crate::adventure::AuthoredAdventureCatalog;
use crate::compiler::defense_stat_id;
use crate::{
    AbilityScore, AbilityScoresComponent, ActionAttackDefinition, ActionDefinition, ActionPreview,
    ActionResource, ActionResourcesComponent, ActionTargetTeamDefinition, AdventureDefinition,
    AffinitySeed, ApplyActionRequest, CharacterAffinityKindDefinition, CharacterSeed,
    CharacterTemplateDefinition, D20CompileError, D20Id, D20Ruleset, D20Session, D20SessionError,
    DamageAffinity, DungeonFacingDefinition, EncounterDefinition, EncounterFaction,
    EncounterFactionDefinition, EncounterParticipationSeed, EquipmentItemSeed, InventorySeed,
    ItemInstanceDefinition, ItemRarityDefinition, ReactionReceipt, RollSourceConfig,
    ScheduledEffectsComponent, SessionSaveError, StorageSeed, TacticalBoardDefinition,
    TacticalPosition, ENGINE_REVISION, MAX_D20_ENCOUNTER_PARTICIPANTS,
};

const GAME_SAVE_SCHEMA_VERSION: u32 = 11;
const MAX_LOG_ENTRIES: usize = 64;
const MAX_LOG_DETAILS: usize = 32;
const MAX_LOG_SOURCE_BYTES: usize = 128;
const MAX_LOG_TEXT_BYTES: usize = 512;
const MAX_LOG_DETAIL_BYTES: usize = 512;
const MAX_GAME_SAVE_BYTES: usize = 1_000_000;

use content::*;
pub use dto::*;
pub use error::GameRuntimeError;
pub use state::GameRuntime;
use state::*;
use support::*;
use tactical::*;

#[cfg(test)]
mod tests;
