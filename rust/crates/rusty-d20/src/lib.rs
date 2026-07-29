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
    admit_d20_candidate, generated_d20_candidate_typescript, AbilityCandidate, ActionCandidate,
    AdventureCandidate, ArmorCandidate, CharacterAbilityCandidate, CharacterAffinityCandidate,
    CharacterAffinityKindCandidate, CharacterResourceCandidate, CharacterTemplateCandidate,
    D20PackageEnvelope, D20RulesCandidate, DamageCandidate, DamageTypeCandidate, DefenseCandidate,
    EffectCandidate, EncounterCandidate, EncounterOutcomeCandidate, ItemInstanceCandidate,
    ItemRarityCandidate, ReactionCandidate, ResourceCandidate, StorageCandidate,
    D20_CANDIDATE_SCHEMA_VERSION,
};
pub use compiler::{
    AbilityDefinition, ActionDefinition, AdventureDefinition, ArmorDefinition,
    CharacterAffinityDefinition, CharacterAffinityKindDefinition, CharacterTemplateDefinition,
    D20CompileError, D20Ruleset, DamageDefinition, DefenseDefinition, EffectDefinition,
    EncounterDefinition, EncounterOutcomeDefinition, ItemInstanceDefinition, ItemRarityDefinition,
    ReactionDefinition, ResourceDefinition, StorageDefinition, MAX_D20_ADVENTURES_PER_PACKAGE,
    MAX_D20_ADVENTURE_ENTRIES, MAX_D20_AUTHORED_TEXT_BYTES, MAX_D20_DAMAGE_DICE,
    MAX_D20_DAMAGE_DIE_SIDES, MAX_D20_DEFINITIONS_PER_KIND, MAX_D20_EFFECT_DURATION_TURNS,
};
pub use component::{
    d20_component_registry, register_d20_components, AbilityScore, AbilityScoresComponent,
    ActionResource, ActionResourcesComponent, D20ComponentDataError, ScheduledEffect,
    ScheduledEffectsComponent, ABILITY_SCORES_COMPONENT_CODEC_ID,
    ABILITY_SCORES_COMPONENT_CODEC_VERSION, ABILITY_SCORES_COMPONENT_TYPE_ID,
    ACTION_RESOURCES_COMPONENT_CODEC_ID, ACTION_RESOURCES_COMPONENT_CODEC_VERSION,
    ACTION_RESOURCES_COMPONENT_TYPE_ID, SCHEDULED_EFFECTS_COMPONENT_CODEC_ID,
    SCHEDULED_EFFECTS_COMPONENT_CODEC_VERSION, SCHEDULED_EFFECTS_COMPONENT_TYPE_ID,
};
pub use game::{
    ActionDto, AdventureChoiceDto, ApiErrorDto, ApiErrorKindDto, ApplyActionRequestDto,
    ApplyReactionRequestDto, CampaignDto, CampaignOutcomeDto, CampaignPhaseDto, CharacterDto,
    DefenseReadoutDto, EncounterChoiceDto, EncounterDto, EncounterOutcomeKindDto,
    EncounterTurnOwnerDto, EnterEncounterRequestDto, EquipItemRequestDto, EquipmentSlotDto,
    ExpectedRevisionDto, GameLogEntryDto, GameLogKindDto, GameRuntime, GameRuntimeError,
    GameSnapshotDto, LoadoutCapacityDto, LoadoutDto, LoadoutItemDto, LoadoutRarityDto,
    NewAdventureRequestDto, PendingActionDto, PreviewActionRequestDto, ReactionDto, ResourceDto,
    TransferItemRequestDto, UnequipItemRequestDto,
};
pub use identity::{D20Id, D20IdentityError, D20_ID_PATTERN, MAX_D20_ID_BYTES};
pub use session::{
    ability_modifier, ActionPreview, ActionReceipt, AdvanceTurnReceipt, AffinitySeed,
    ApplyActionRequest, ArmorItemSeed, CharacterSeed, D20Session, D20SessionError, DamageAffinity,
    InventorySeed, ReactionOption, ReactionReceipt, SessionSaveError, StorageSeed,
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
        EncounterTurnOwnerDto::decl(),
        EncounterDto::decl(),
        CampaignPhaseDto::decl(),
        EncounterOutcomeKindDto::decl(),
        CampaignOutcomeDto::decl(),
        EncounterChoiceDto::decl(),
        AdventureChoiceDto::decl(),
        LoadoutRarityDto::decl(),
        LoadoutItemDto::decl(),
        EquipmentSlotDto::decl(),
        LoadoutCapacityDto::decl(),
        DefenseReadoutDto::decl(),
        LoadoutDto::decl(),
        CampaignDto::decl(),
        GameSnapshotDto::decl(),
        ExpectedRevisionDto::decl(),
        NewAdventureRequestDto::decl(),
        EnterEncounterRequestDto::decl(),
        EquipItemRequestDto::decl(),
        UnequipItemRequestDto::decl(),
        TransferItemRequestDto::decl(),
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
