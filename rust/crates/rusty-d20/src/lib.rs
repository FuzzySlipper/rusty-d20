//! Concrete downstream state and protocol for the Rusty D20 reference game.
//!
//! Rusty Engine supplies reusable mechanisms. This crate owns Rusty D20's
//! product state, transport projection, and d20 semantics.

#![forbid(unsafe_code)]

mod adventure;
mod candidate;
mod compiler;
mod component;
mod game;
mod identity;
mod session;

pub mod host;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use adventure::MAX_D20_SELECTABLE_ADVENTURES;
pub use candidate::{
    admit_d20_candidate, generated_d20_candidate_typescript, AbilityCandidate,
    ActionAttackCandidate, ActionCandidate, ActionLineOfEffectCandidate, ActionTargetCandidate,
    ActionTargetKindCandidate, ActionTargetTeamCandidate, ActivationBudgetCandidate,
    ActivationCostCandidate, ActivationTimingCandidate, AdventureCandidate, ArmorCandidate,
    CharacterAbilityCandidate, CharacterAffinityCandidate, CharacterAffinityKindCandidate,
    CharacterResourceCandidate, CharacterTemplateCandidate, ConditionClauseCandidate,
    D20PackageEnvelope, D20RulesCandidate, DamageCandidate, DamageTypeCandidate, DefenseCandidate,
    DungeonCandidate, DungeonEncounterCandidate, DungeonFacingCandidate, DungeonLandmarkCandidate,
    EffectCandidate, EncounterCandidate, EncounterFactionCandidate, EncounterOutcomeCandidate,
    EncounterParticipantCandidate, EquipmentReferenceCandidate, ImplementCandidate,
    ItemInstanceCandidate, ItemRarityCandidate, ReactionCandidate, ResourceCandidate,
    StorageCandidate, TacticalBoardCandidate, TacticalPlacementCandidate,
    D20_CANDIDATE_SCHEMA_VERSION,
};
pub use compiler::{
    AbilityDefinition, ActionAttackDefinition, ActionDefinition, ActionLineOfEffectDefinition,
    ActionTargetDefinition, ActionTargetKindDefinition, ActionTargetTeamDefinition,
    ActivationBudgetDefinition, ActivationCostDefinition, ActivationTimingDefinition,
    AdventureDefinition, ArmorDefinition, CharacterAffinityDefinition,
    CharacterAffinityKindDefinition, CharacterTemplateDefinition, ConditionClauseDefinition,
    D20CompileError, D20Ruleset, DamageDefinition, DefenseDefinition, DungeonDefinition,
    DungeonEncounterDefinition, DungeonFacingDefinition, DungeonLandmarkDefinition,
    EffectDefinition, EncounterDefinition, EncounterFactionDefinition, EncounterOutcomeDefinition,
    EncounterParticipantDefinition, EquipmentReferenceDefinition, ImplementDefinition,
    ItemInstanceDefinition, ItemRarityDefinition, ReactionDefinition, ResourceDefinition,
    StorageDefinition, TacticalBoardDefinition, TacticalPlacementDefinition,
    TacticalPositionDefinition, MAX_D20_ACTION_TAGS, MAX_D20_ACTION_TARGETS,
    MAX_D20_ACTIVATION_COSTS, MAX_D20_ADVENTURES_PER_PACKAGE, MAX_D20_ADVENTURE_ENTRIES,
    MAX_D20_AUTHORED_TEXT_BYTES, MAX_D20_CONDITION_CLAUSES, MAX_D20_DAMAGE_DICE,
    MAX_D20_DAMAGE_DIE_SIDES, MAX_D20_DEFINITIONS_PER_KIND, MAX_D20_DUNGEON_CELLS,
    MAX_D20_DUNGEON_HEIGHT, MAX_D20_DUNGEON_WIDTH, MAX_D20_EFFECT_DURATION_TURNS,
    MAX_D20_ENCOUNTER_PARTICIPANTS, MAX_D20_FORCED_MOVEMENT, MAX_D20_IMPLEMENT_TAGS,
    MAX_D20_PARTY_MEMBERS, MAX_D20_TACTICAL_BOARD_CELLS, MAX_D20_TACTICAL_BOARD_HEIGHT,
    MAX_D20_TACTICAL_BOARD_WIDTH, MAX_D20_TACTICAL_RANGE,
};
pub use component::{
    d20_component_registry, register_d20_components, AbilityScore, AbilityScoresComponent,
    ActionResource, ActionResourcesComponent, ActivationBudget, ActivationBudgetsComponent,
    D20ComponentDataError, EncounterFaction, EncounterParticipationComponent, ScheduledEffect,
    ScheduledEffectsComponent, TacticalPosition, ABILITY_SCORES_COMPONENT_CODEC_ID,
    ABILITY_SCORES_COMPONENT_CODEC_VERSION, ABILITY_SCORES_COMPONENT_TYPE_ID,
    ACTION_RESOURCES_COMPONENT_CODEC_ID, ACTION_RESOURCES_COMPONENT_CODEC_VERSION,
    ACTION_RESOURCES_COMPONENT_TYPE_ID, ACTIVATION_BUDGETS_COMPONENT_CODEC_ID,
    ACTIVATION_BUDGETS_COMPONENT_CODEC_VERSION, ACTIVATION_BUDGETS_COMPONENT_TYPE_ID,
    ENCOUNTER_PARTICIPATION_COMPONENT_CODEC_ID, ENCOUNTER_PARTICIPATION_COMPONENT_CODEC_VERSION,
    ENCOUNTER_PARTICIPATION_COMPONENT_TYPE_ID, SCHEDULED_EFFECTS_COMPONENT_CODEC_ID,
    SCHEDULED_EFFECTS_COMPONENT_CODEC_VERSION, SCHEDULED_EFFECTS_COMPONENT_TYPE_ID,
};
pub use game::{
    ActionDto, ActionTargetsDto, AdventureChoiceDto, ApiErrorDto, ApiErrorKindDto,
    ApplyActionRequestDto, ApplyReactionRequestDto, CampaignDto, CampaignOutcomeDto,
    CampaignPhaseDto, CharacterDto, CompletedEncounterDto, DefenseReadoutDto, DiscoveredCellDto,
    EncounterChoiceDto, EncounterDto, EncounterFactionDto, EncounterOutcomeKindDto,
    EncounterParticipantDto, EnterEncounterRequestDto, EquipItemRequestDto, EquipmentSlotDto,
    ExpectedRevisionDto, ExplorationCommandKindDto, ExplorationCommandRequestDto,
    ExplorationDepthDto, ExplorationDto, ExplorationFacingDto, ExplorationLandmarkDto,
    GameLogEntryDto, GameLogKindDto, GameRuntime, GameRuntimeError, GameSnapshotDto,
    LoadoutCapacityDto, LoadoutDto, LoadoutItemDto, LoadoutRarityDto, MoveActorRequestDto,
    NewAdventureRequestDto, PartyMemberDto, PendingActionDto, PreviewActionRequestDto, ReactionDto,
    ResetSessionRequestDto, ResourceDto, SaveStateDto, SaveStatusDto, TacticalBoardDto,
    TacticalCellDto, TacticalMoveDto, TransferItemRequestDto, UnequipItemRequestDto,
};
pub use identity::{D20Id, D20IdentityError, D20_ID_PATTERN, MAX_D20_ID_BYTES};
pub use session::{
    ability_modifier, ActionPreview, ActionReceipt, AdvanceTurnReceipt, AffinitySeed,
    ApplyActionRequest, ArmorItemSeed, CharacterSeed, D20Session, D20SessionError, DamageAffinity,
    EncounterParticipationSeed, EquipmentItemSeed, InventorySeed, ReactionOption, ReactionReceipt,
    SessionSaveError, StorageSeed,
};

/// Exact reviewed Rusty Engine revision used by this repository.
pub const ENGINE_REVISION: &str = "fb608e323a8b44a55195f5720101224ff37fd5db";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum RuntimeStatusDto {
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RuntimeReadoutDto {
    pub product: String,
    pub version: String,
    pub engine_revision: String,
    pub status: RuntimeStatusDto,
    pub entity_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct HealthDto {
    pub status: String,
    pub version: String,
}

impl GameRuntime {
    pub fn readout(&self) -> RuntimeReadoutDto {
        RuntimeReadoutDto {
            product: "Rusty D20".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            engine_revision: ENGINE_REVISION.to_owned(),
            status: RuntimeStatusDto::Ready,
            entity_count: u32::try_from(self.readout_entity_count())
                .expect("entity-state bounds fit the protocol count"),
        }
    }
}

pub fn generated_typescript() -> String {
    let declarations = [
        RuntimeStatusDto::decl(),
        RuntimeReadoutDto::decl(),
        HealthDto::decl(),
        GameLogKindDto::decl(),
        GameLogEntryDto::decl(),
        ResourceDto::decl(),
        CharacterDto::decl(),
        ActionDto::decl(),
        ReactionDto::decl(),
        PendingActionDto::decl(),
        ActionTargetsDto::decl(),
        EncounterFactionDto::decl(),
        EncounterParticipantDto::decl(),
        TacticalCellDto::decl(),
        TacticalMoveDto::decl(),
        TacticalBoardDto::decl(),
        EncounterDto::decl(),
        CampaignPhaseDto::decl(),
        ExplorationFacingDto::decl(),
        ExplorationCommandKindDto::decl(),
        ExplorationDepthDto::decl(),
        DiscoveredCellDto::decl(),
        ExplorationLandmarkDto::decl(),
        ExplorationDto::decl(),
        EncounterOutcomeKindDto::decl(),
        CampaignOutcomeDto::decl(),
        CompletedEncounterDto::decl(),
        EncounterChoiceDto::decl(),
        AdventureChoiceDto::decl(),
        LoadoutRarityDto::decl(),
        LoadoutItemDto::decl(),
        EquipmentSlotDto::decl(),
        LoadoutCapacityDto::decl(),
        DefenseReadoutDto::decl(),
        LoadoutDto::decl(),
        PartyMemberDto::decl(),
        CampaignDto::decl(),
        GameSnapshotDto::decl(),
        SaveStateDto::decl(),
        SaveStatusDto::decl(),
        ResetSessionRequestDto::decl(),
        ExpectedRevisionDto::decl(),
        NewAdventureRequestDto::decl(),
        EnterEncounterRequestDto::decl(),
        ExplorationCommandRequestDto::decl(),
        EquipItemRequestDto::decl(),
        UnequipItemRequestDto::decl(),
        TransferItemRequestDto::decl(),
        MoveActorRequestDto::decl(),
        PreviewActionRequestDto::decl(),
        ApplyReactionRequestDto::decl(),
        ApplyActionRequestDto::decl(),
        ApiErrorKindDto::decl(),
        ApiErrorDto::decl(),
    ];
    format!(
        "// GENERATED by `cargo run -p rusty-d20 --bin rusty-d20-protocol`. Do not hand-edit.\n\n\
export const D20_PROTOCOL_LIMITS = Object.freeze({{\n\
  maxAvailableAdventures: {MAX_D20_SELECTABLE_ADVENTURES},\n\
  maxAdventureDetails: {MAX_D20_ADVENTURE_ENTRIES},\n\
  maxCampaignEncounters: {MAX_D20_ADVENTURE_ENTRIES},\n\
  maxPartyMembers: {MAX_D20_PARTY_MEMBERS},\n\
  maxEncounterParticipants: {MAX_D20_ENCOUNTER_PARTICIPANTS},\n\
  maxTacticalBoardWidth: {MAX_D20_TACTICAL_BOARD_WIDTH},\n\
  maxTacticalBoardHeight: {MAX_D20_TACTICAL_BOARD_HEIGHT},\n\
  maxTacticalBoardCells: {MAX_D20_TACTICAL_BOARD_CELLS},\n\
  maxDungeonCells: {MAX_D20_DUNGEON_CELLS},\n\
  maxDungeonViewDepth: 3,\n\
}} as const);\n\n\
export {}\n",
        declarations.join("\n\nexport "),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn empty_readout_is_rust_owned_and_engine_backed() {
        let runtime = GameRuntime::empty().expect("empty product runtime");
        assert_eq!(
            runtime.readout(),
            RuntimeReadoutDto {
                product: "Rusty D20".to_owned(),
                version: "0.1.0".to_owned(),
                engine_revision: ENGINE_REVISION.to_owned(),
                status: RuntimeStatusDto::Ready,
                entity_count: 0,
            }
        );
    }

    #[test]
    fn committed_typescript_protocol_matches_rust_owner() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../libs/protocol/src/generated/api-types.ts");
        let committed = fs::read_to_string(path).expect("committed TypeScript protocol");
        assert_eq!(committed, generated_typescript());
    }

    #[test]
    fn engine_support_crates_are_linked_from_the_reviewed_revision() {
        let _mechanics_limit = gameplay_mechanics::MAX_TRACKS_PER_ENTITY;
        let _rules_limit = gameplay_rules::MAX_RULE_PACKAGES_PER_SET;
        let _rng_seed = svc_rng::RngSeed::new(1);
        assert_eq!(ENGINE_REVISION.len(), 40);
    }
}
