use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum GameLogKindDto {
    System,
    Reaction,
    Hit,
    Miss,
    Turn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct GameLogEntryDto {
    #[ts(type = "number")]
    pub id: u64,
    #[ts(type = "number")]
    pub turn: u64,
    pub kind: GameLogKindDto,
    pub source: String,
    pub text: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ResourceDto {
    pub id: String,
    pub label: String,
    pub current: u16,
    pub maximum: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AbilityReadoutDto {
    pub id: String,
    pub label: String,
    pub score: i16,
    pub modifier: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FeatureReadoutDto {
    pub id: String,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AffinityReadoutDto {
    pub damage_type: String,
    pub label: String,
    pub affinity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CharacterReactionDto {
    pub id: String,
    pub label: String,
    pub defense: String,
    pub bonus: i16,
    pub resource: String,
    pub cost: u16,
    pub available: u16,
    pub activation: Vec<String>,
    pub effect: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CharacterDto {
    #[ts(type = "number")]
    pub id: u64,
    pub name: String,
    pub title: String,
    pub level: u16,
    pub experience: u32,
    #[ts(type = "number")]
    pub health_current: i64,
    #[ts(type = "number")]
    pub health_maximum: i64,
    pub abilities: Vec<AbilityReadoutDto>,
    pub defenses: Vec<DefenseReadoutDto>,
    pub resources: Vec<ResourceDto>,
    pub effects: Vec<String>,
    pub features: Vec<FeatureReadoutDto>,
    pub actions: Vec<ActionDto>,
    pub reactions: Vec<CharacterReactionDto>,
    pub affinities: Vec<AffinityReadoutDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ActionDto {
    pub id: String,
    pub label: String,
    pub ability: String,
    pub defense: String,
    pub damage: String,
    pub activation: Vec<String>,
    pub target: String,
    pub range: u16,
    pub implement: Option<String>,
    pub tags: Vec<String>,
    pub effect: Option<String>,
    pub forced_movement: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ReactionDto {
    pub id: String,
    pub label: String,
    pub resource: String,
    pub cost: u16,
    pub available: u16,
    pub bonus: i16,
    pub effect: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ReactionPromptDto {
    pub token: String,
    #[ts(type = "number")]
    pub actor_id: u64,
    #[ts(type = "number")]
    pub target_id: u64,
    pub action_id: String,
    pub action_label: String,
    pub ability_score: i16,
    pub ability_modifier: i16,
    #[ts(type = "number")]
    pub defense: i64,
    pub defense_sources: Vec<String>,
    pub reactions: Vec<ReactionDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EncounterDto {
    #[ts(type = "number")]
    pub round: u64,
    #[ts(type = "number | null")]
    pub current_actor_id: Option<u64>,
    pub board: TacticalBoardDto,
    pub participants: Vec<EncounterParticipantDto>,
    pub actions: Vec<ActionDto>,
    pub legal_targets: Vec<ActionTargetsDto>,
    pub reaction_prompt: Option<ReactionPromptDto>,
    pub log: Vec<GameLogEntryDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TacticalBoardDto {
    pub width: u16,
    pub height: u16,
    pub rows: Vec<String>,
    pub legal_moves: Vec<TacticalMoveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TacticalMoveDto {
    pub x: u16,
    pub y: u16,
    pub cost: u16,
    pub route: Vec<TacticalCellDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TacticalCellDto {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ActionTargetsDto {
    pub action_id: String,
    #[ts(type = "number[]")]
    pub target_ids: Vec<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum EncounterFactionDto {
    Party,
    Opposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EncounterParticipantDto {
    pub character: CharacterDto,
    pub faction: EncounterFactionDto,
    pub initiative: i16,
    pub defeated: bool,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum CampaignPhaseDto {
    Camp,
    Exploration,
    Encounter,
    Outcome,
    AdventureComplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum ExplorationFacingDto {
    North,
    East,
    South,
    West,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum ExplorationCommandKindDto {
    TurnLeft,
    TurnRight,
    StepForward,
    StepBackward,
    Interact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ExplorationDepthDto {
    pub depth: u16,
    pub front_blocked: bool,
    pub left_blocked: bool,
    pub right_blocked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DiscoveredCellDto {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ExplorationLandmarkDto {
    pub id: String,
    pub title: String,
    pub text: String,
    pub inspected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ExplorationDoorDto {
    pub id: String,
    pub title: String,
    pub text: String,
    pub opened: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ExplorationTreasureDto {
    pub id: String,
    pub title: String,
    pub text: String,
    pub collected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ExplorationCheckpointDto {
    pub id: String,
    pub title: String,
    pub text: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ExplorationDto {
    pub dungeon_title: String,
    pub wall_style: String,
    pub width: u16,
    pub height: u16,
    pub x: u16,
    pub y: u16,
    pub facing: ExplorationFacingDto,
    pub can_step_forward: bool,
    pub can_step_backward: bool,
    pub view: Vec<ExplorationDepthDto>,
    pub discovered_cells: Vec<DiscoveredCellDto>,
    pub landmark: Option<ExplorationLandmarkDto>,
    pub door_ahead: Option<ExplorationDoorDto>,
    pub treasure: Option<ExplorationTreasureDto>,
    pub checkpoint: Option<ExplorationCheckpointDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum EncounterOutcomeKindDto {
    Victory,
    Defeat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CampaignOutcomeDto {
    pub kind: EncounterOutcomeKindDto,
    pub encounter_id: String,
    pub title: String,
    pub summary: String,
    #[ts(type = "number | null")]
    pub reward_item_id: Option<u64>,
    pub reward: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AdventureCompletionDto {
    pub kind: EncounterOutcomeKindDto,
    pub source: String,
    pub title: String,
    pub text: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CompletedEncounterDto {
    pub encounter_id: String,
    pub title: String,
    pub outcome: EncounterOutcomeKindDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EncounterChoiceDto {
    pub id: String,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AdventureChoiceDto {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum LoadoutRarityDto {
    Common,
    Uncommon,
    Rare,
    Epic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LoadoutItemDto {
    #[ts(type = "number")]
    pub entity_id: u64,
    pub definition_id: String,
    pub name: String,
    pub icon: String,
    pub rarity: LoadoutRarityDto,
    #[ts(type = "number")]
    pub quantity: u64,
    pub equipment_slot_id: String,
    pub equipped_slot_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EquipmentSlotDto {
    pub id: String,
    pub label: String,
    pub equipped: Option<LoadoutItemDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LoadoutCapacityDto {
    pub metric: String,
    #[ts(type = "number")]
    pub used: u64,
    #[ts(type = "number")]
    pub maximum: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DefenseReadoutDto {
    pub id: String,
    pub label: String,
    #[ts(type = "number")]
    pub value: i64,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LoadoutDto {
    #[ts(type = "number")]
    pub owner_id: u64,
    #[ts(type = "number")]
    pub stash_owner_id: u64,
    pub inventory_slots: Vec<Option<LoadoutItemDto>>,
    pub equipment_slots: Vec<EquipmentSlotDto>,
    pub stash_items: Vec<LoadoutItemDto>,
    pub stash_capacity: LoadoutCapacityDto,
    pub capacity: LoadoutCapacityDto,
    pub defenses: Vec<DefenseReadoutDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CampaignDto {
    pub id: String,
    pub title: String,
    pub phase: CampaignPhaseDto,
    pub party: Vec<PartyMemberDto>,
    pub active_encounter_id: Option<String>,
    pub available_encounters: Vec<EncounterChoiceDto>,
    pub latest_outcome: Option<CampaignOutcomeDto>,
    pub completed_encounters: Vec<CompletedEncounterDto>,
    pub completion: Option<AdventureCompletionDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PartyMemberDto {
    pub character: CharacterDto,
    pub loadout: LoadoutDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct GameSnapshotDto {
    pub product: String,
    pub version: String,
    pub engine_revision: String,
    pub ruleset_fingerprint: String,
    #[ts(type = "number")]
    pub revision: u64,
    pub saved: bool,
    pub available_adventures: Vec<AdventureChoiceDto>,
    pub campaign: Option<CampaignDto>,
    pub exploration: Option<ExplorationDto>,
    pub encounter: Option<EncounterDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum SaveStateDto {
    Empty,
    Ready,
    RecoveryRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SaveStatusDto {
    pub save_identity: String,
    pub state: SaveStateDto,
    pub campaign_id: Option<String>,
    pub campaign_title: Option<String>,
    #[ts(type = "number | null")]
    pub revision: Option<u64>,
    pub persistence_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ResetSessionRequestDto {
    pub expected_save_identity: String,
    #[ts(type = "number | null")]
    pub expected_revision: Option<u64>,
    pub expected_adventure_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ExpectedRevisionDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct NewAdventureRequestDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub adventure_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EnterEncounterRequestDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub encounter_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ExplorationCommandRequestDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub command: ExplorationCommandKindDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EquipItemRequestDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[ts(type = "number")]
    pub item_id: u64,
    pub slot_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UnequipItemRequestDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[ts(type = "number")]
    pub item_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TransferItemRequestDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[ts(type = "number")]
    pub item_id: u64,
    #[ts(type = "number")]
    pub from_owner_id: u64,
    #[ts(type = "number")]
    pub to_owner_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct MoveLoadoutItemRequestDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[ts(type = "number")]
    pub item_id: u64,
    #[ts(type = "number")]
    pub from_owner_id: u64,
    #[ts(type = "number")]
    pub to_owner_id: u64,
    pub destination_slot_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ChooseActionRequestDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[ts(type = "number")]
    pub actor_id: u64,
    #[ts(type = "number")]
    pub target_id: u64,
    pub action_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct MoveActorRequestDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
    #[ts(type = "number")]
    pub actor_id: u64,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ApplyReactionRequestDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub prompt_token: String,
    pub reaction_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DeclineReactionRequestDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub prompt_token: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum ApiErrorKindDto {
    Stale,
    Invalid,
    InvalidSlot,
    Capacity,
    Containment,
    TrackBound,
    Phase,
    NotFound,
    Persistence,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ApiErrorDto {
    pub kind: ApiErrorKindDto,
    pub message: String,
    pub retryable: bool,
}
