use super::*;

#[derive(Debug, Clone)]
pub(super) struct PendingAction {
    pub(super) serial: u64,
    pub(super) token: String,
    pub(super) preview: ActionPreview,
}

pub(super) type LegalActionPreview = (D20Id, EntityId, ActionPreview);

#[derive(Debug)]
pub(super) struct RestoreData {
    pub(super) revision: u64,
    pub(super) next_operation: u64,
    pub(super) next_log_id: u64,
    pub(super) log: Vec<GameLogEntryDto>,
    pub(super) session: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum CampaignPhase {
    Camp,
    Exploration,
    Encounter,
    Outcome,
    AdventureComplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum EncounterOutcome {
    Victory,
    Defeat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct CompletedEncounter {
    pub(super) encounter_id: String,
    pub(super) outcome: EncounterOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct DungeonPosition {
    pub(super) x: u16,
    pub(super) y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ExplorationState {
    pub(super) position: DungeonPosition,
    pub(super) facing: DungeonFacingDefinition,
    pub(super) discovered: BTreeSet<DungeonPosition>,
    pub(super) inspected_landmarks: BTreeSet<String>,
    pub(super) checkpoint_id: String,
    pub(super) opened_doors: BTreeSet<String>,
    pub(super) collected_treasures: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CampaignState {
    pub(super) phase: CampaignPhase,
    pub(super) active_encounter_id: Option<String>,
    pub(super) resolved_encounter_id: Option<String>,
    pub(super) current_actor_id: Option<u64>,
    pub(super) outcome: Option<EncounterOutcome>,
    pub(super) completed_encounters: Vec<CompletedEncounter>,
    pub(super) exploration: Option<ExplorationState>,
}

#[derive(Debug, Clone)]
pub struct GameRuntime {
    pub(super) catalog: AuthoredAdventureCatalog,
    pub(super) rules: D20Ruleset,
    pub(super) adventure_id: D20Id,
    pub(super) roll_source: RollSourceConfig,
    pub(super) campaign: Option<CampaignState>,
    pub(super) session: Option<D20Session>,
    pub(super) revision: u64,
    pub(super) saved_revision: Option<u64>,
    pub(super) next_operation: u64,
    pub(super) next_log_id: u64,
    pub(super) pending: Option<PendingAction>,
    pub(super) log: Vec<GameLogEntryDto>,
}
