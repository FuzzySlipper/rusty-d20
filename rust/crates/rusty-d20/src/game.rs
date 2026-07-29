use std::collections::BTreeMap;

use core_ids::EntityId;
use entity_state::ComponentAccessError;
use gameplay_mechanics::{
    ActiveEffectsComponent, DecisionOutcome, EffectInstanceId, EquipmentComponent,
    InventoryComponent, ItemComponent, MechanicsError, OperationId, ResponseDecisionKind,
    SourceInstanceIdentity, StatContribution, StatService, TracksComponent,
};
use gameplay_rules::AdmittedRulePackage;
use serde::{Deserialize, Serialize};
use svc_rng::RngSeed;
use ts_rs::TS;

use crate::compiler::defense_stat_id;
use crate::{
    AbilityScore, ActionPreview, ActionResource, ActionResourcesComponent, AffinitySeed,
    ApplyActionRequest, ArmorItemSeed, CharacterSeed, D20CompileError, D20Id, D20Ruleset,
    D20Session, D20SessionError, DamageAffinity, InventorySeed, ReactionReceipt,
    ScheduledEffectsComponent, SessionSaveError, StorageSeed, ENGINE_REVISION,
};

const GAME_SAVE_SCHEMA_VERSION: u32 = 4;
const ADVENTURE_ID: &str = "wardens-gate";
const ADVENTURE_TITLE: &str = "The Warden's Gate";
const ENCOUNTER_ID: &str = "iron-warden";
const ENCOUNTER_TITLE: &str = "The Iron Warden";
const PLAYER: EntityId = EntityId::new(101);
const OPPONENT: EntityId = EntityId::new(102);
const CAMP_STASH: EntityId = EntityId::new(103);
const OPPONENT_ARMOR: EntityId = EntityId::new(201);
const PLAYER_CHAIN_ARMOR: EntityId = EntityId::new(202);
const PLAYER_BUCKLER: EntityId = EntityId::new(203);
const STASH_BUCKLER: EntityId = EntityId::new(204);
const MAX_LOG_ENTRIES: usize = 64;
const MAX_LOG_DETAILS: usize = 32;
const MAX_LOG_SOURCE_BYTES: usize = 128;
const MAX_LOG_TEXT_BYTES: usize = 512;
const MAX_LOG_DETAIL_BYTES: usize = 512;
const MAX_GAME_SAVE_BYTES: usize = 1_000_000;
const STARTING_VITALITY: u32 = 24;
const DEFEAT_RECOVERY_VITALITY: u32 = 12;
const STARTER_CORE: &str = include_str!("../../../../rules/artifacts/starter/starter-core.json");
const STEEL_GUARD: &str = include_str!("../../../../rules/artifacts/starter/steel-guard.json");

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
pub struct CharacterDto {
    #[ts(type = "number")]
    pub id: u64,
    pub name: String,
    pub title: String,
    pub level: u16,
    #[ts(type = "number")]
    pub health_current: i64,
    #[ts(type = "number")]
    pub health_maximum: i64,
    pub resources: Vec<ResourceDto>,
    pub effects: Vec<String>,
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
    pub effect: Option<String>,
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
pub struct PendingActionDto {
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
    pub turn: u64,
    #[ts(type = "number")]
    pub next_roll: u64,
    #[ts(type = "number")]
    pub player_id: u64,
    pub turn_owner: Option<EncounterTurnOwnerDto>,
    pub characters: Vec<CharacterDto>,
    pub actions: Vec<ActionDto>,
    pub pending_action: Option<PendingActionDto>,
    pub log: Vec<GameLogEntryDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum EncounterTurnOwnerDto {
    Player,
    Opposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum CampaignPhaseDto {
    Camp,
    Encounter,
    Outcome,
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
pub struct EncounterChoiceDto {
    pub id: String,
    pub title: String,
    pub summary: String,
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
pub struct LoadoutDto {
    #[ts(type = "number")]
    pub owner_id: u64,
    #[ts(type = "number")]
    pub stash_owner_id: u64,
    pub inventory_slots: Vec<Option<LoadoutItemDto>>,
    pub equipment_slots: Vec<EquipmentSlotDto>,
    pub stash_items: Vec<LoadoutItemDto>,
    pub capacity: LoadoutCapacityDto,
    #[ts(type = "number")]
    pub armor_defense: i64,
    pub armor_defense_sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CampaignDto {
    pub id: String,
    pub title: String,
    pub phase: CampaignPhaseDto,
    pub hero: CharacterDto,
    pub loadout: LoadoutDto,
    pub active_encounter_id: Option<String>,
    pub available_encounters: Vec<EncounterChoiceDto>,
    pub latest_outcome: Option<CampaignOutcomeDto>,
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
    pub campaign: Option<CampaignDto>,
    pub encounter: Option<EncounterDto>,
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
pub struct EnterEncounterRequestDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub encounter_id: String,
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
pub struct PreviewActionRequestDto {
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
pub struct ApplyReactionRequestDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub preview_token: String,
    pub reaction_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ApplyActionRequestDto {
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub preview_token: String,
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

#[derive(Debug, Clone)]
struct PendingAction {
    serial: u64,
    token: String,
    preview: ActionPreview,
}

#[derive(Debug)]
struct RestoreData {
    revision: u64,
    next_operation: u64,
    next_log_id: u64,
    log: Vec<GameLogEntryDto>,
    session: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum CampaignPhase {
    Camp,
    Encounter,
    Outcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum EncounterTurnOwner {
    Player,
    Opposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum EncounterOutcome {
    Victory,
    Defeat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CampaignState {
    phase: CampaignPhase,
    active_encounter_id: Option<String>,
    turn_owner: Option<EncounterTurnOwner>,
    outcome: Option<EncounterOutcome>,
}

#[derive(Debug, Clone)]
pub struct GameRuntime {
    rules: D20Ruleset,
    campaign: Option<CampaignState>,
    session: Option<D20Session>,
    revision: u64,
    saved_revision: Option<u64>,
    next_operation: u64,
    next_log_id: u64,
    pending: Option<PendingAction>,
    log: Vec<GameLogEntryDto>,
}

impl GameRuntime {
    pub fn empty() -> Result<Self, GameRuntimeError> {
        Ok(Self {
            rules: starter_ruleset()?,
            campaign: None,
            session: None,
            revision: 0,
            saved_revision: None,
            next_operation: 1,
            next_log_id: 1,
            pending: None,
            log: Vec::new(),
        })
    }

    pub fn decode_save(input: &str) -> Result<Self, GameRuntimeError> {
        if input.len() > MAX_GAME_SAVE_BYTES {
            return Err(GameRuntimeError::InvalidSave(format!(
                "save contains {} bytes; maximum is {MAX_GAME_SAVE_BYTES}",
                input.len()
            )));
        }
        let value: serde_json::Value = serde_json::from_str(input)?;
        let envelope: SaveEnvelope = serde_json::from_value(value.clone())?;
        let rules = starter_ruleset()?;
        match envelope.schema_version {
            1 => {
                let save: LegacyGameSave = serde_json::from_value(value)?;
                Self::restore(
                    rules,
                    RestoreData {
                        revision: save.revision,
                        next_operation: save.next_operation,
                        next_log_id: save.next_log_id,
                        log: save.log,
                        session: save.session,
                    },
                    CampaignState {
                        phase: CampaignPhase::Encounter,
                        active_encounter_id: Some(ENCOUNTER_ID.to_owned()),
                        turn_owner: Some(EncounterTurnOwner::Player),
                        outcome: None,
                    },
                    true,
                )
            }
            2 | 3 => {
                let save: LegacyCampaignGameSave = serde_json::from_value(value)?;
                let campaign = validate_legacy_campaign_save(save.campaign)?;
                Self::restore(
                    rules,
                    RestoreData {
                        revision: save.revision,
                        next_operation: save.next_operation,
                        next_log_id: save.next_log_id,
                        log: save.log,
                        session: save.session,
                    },
                    campaign,
                    true,
                )
            }
            GAME_SAVE_SCHEMA_VERSION => {
                let save: GameSave = serde_json::from_value(value)?;
                let campaign = validate_campaign_save(save.campaign)?;
                Self::restore(
                    rules,
                    RestoreData {
                        revision: save.revision,
                        next_operation: save.next_operation,
                        next_log_id: save.next_log_id,
                        log: save.log,
                        session: save.session,
                    },
                    campaign,
                    false,
                )
            }
            actual => Err(GameRuntimeError::UnsupportedSaveSchema { actual }),
        }
    }

    fn restore(
        rules: D20Ruleset,
        data: RestoreData,
        campaign: CampaignState,
        legacy: bool,
    ) -> Result<Self, GameRuntimeError> {
        let mut data = data;
        let mut campaign = campaign;
        let session_json = serde_json::to_string(&data.session)?;
        let mut session = D20Session::decode_save(rules.clone(), &session_json)?;
        if legacy
            && session
                .entities()
                .component::<InventoryComponent>(PLAYER)?
                .is_none()
        {
            install_product_loadout(&mut session)?;
        }
        if legacy {
            campaign = migrate_legacy_campaign(&mut session, campaign, &mut data.next_operation)?;
        }
        validate_product_state(&session, &campaign)?;
        if data.next_operation == 0 || data.next_log_id == 0 || data.log.len() > MAX_LOG_ENTRIES {
            return Err(GameRuntimeError::InvalidSave(
                "operation/log counters or bounded log are invalid".to_owned(),
            ));
        }
        if data.log.windows(2).any(|pair| pair[0].id >= pair[1].id) {
            return Err(GameRuntimeError::InvalidSave(
                "log identities are not in strict order".to_owned(),
            ));
        }
        if data.log.iter().any(|entry| {
            entry.id == 0
                || entry.source.len() > MAX_LOG_SOURCE_BYTES
                || entry.text.len() > MAX_LOG_TEXT_BYTES
                || entry.details.len() > MAX_LOG_DETAILS
                || entry
                    .details
                    .iter()
                    .any(|detail| detail.len() > MAX_LOG_DETAIL_BYTES)
        }) || data
            .log
            .last()
            .is_some_and(|entry| data.next_log_id <= entry.id)
        {
            return Err(GameRuntimeError::InvalidSave(
                "log entry bounds or next identity are invalid".to_owned(),
            ));
        }
        let runtime = Self {
            rules,
            campaign: Some(campaign),
            session: Some(session),
            revision: data.revision,
            saved_revision: Some(data.revision),
            next_operation: data.next_operation,
            next_log_id: data.next_log_id,
            pending: None,
            log: data.log,
        };
        runtime.snapshot()?;
        Ok(runtime)
    }

    pub fn encode_save(&self) -> Result<String, GameRuntimeError> {
        if self.pending.is_some() {
            return Err(GameRuntimeError::PendingActionCannotBeSaved);
        }
        let campaign = self
            .campaign
            .as_ref()
            .ok_or(GameRuntimeError::NoEncounter)?;
        let session = self.session.as_ref().ok_or(GameRuntimeError::NoEncounter)?;
        let session = serde_json::from_str(&session.encode_save()?)?;
        Ok(serde_json::to_string_pretty(&GameSave {
            schema_version: GAME_SAVE_SCHEMA_VERSION,
            revision: self.revision,
            next_operation: self.next_operation,
            next_log_id: self.next_log_id,
            log: self.log.clone(),
            campaign: CampaignSave {
                adventure_id: ADVENTURE_ID.to_owned(),
                phase: campaign.phase,
                active_encounter_id: campaign.active_encounter_id.clone(),
                turn_owner: campaign.turn_owner,
                outcome: campaign.outcome,
            },
            session,
        })?)
    }

    pub fn encode_save_at(&self, expected_revision: u64) -> Result<String, GameRuntimeError> {
        self.ensure_revision(expected_revision)?;
        self.encode_save()
    }

    pub fn mark_saved(&mut self, revision: u64) {
        if self.revision == revision {
            self.saved_revision = Some(revision);
        }
    }

    pub fn readout_entity_count(&self) -> usize {
        self.session
            .as_ref()
            .map_or(0, |session| session.entities().total_count())
    }

    pub fn snapshot(&self) -> Result<GameSnapshotDto, GameRuntimeError> {
        let session = self.session.as_ref();
        let campaign = match (&self.campaign, session) {
            (Some(campaign), Some(session)) => Some(self.project_campaign(campaign, session)?),
            (None, None) => None,
            _ => {
                return Err(GameRuntimeError::InvalidState(
                    "campaign and session ownership diverged".to_owned(),
                ));
            }
        };
        let encounter = match (&self.campaign, session) {
            (Some(campaign), Some(session))
                if matches!(
                    campaign.phase,
                    CampaignPhase::Encounter | CampaignPhase::Outcome
                ) =>
            {
                Some(self.project_encounter(campaign, session)?)
            }
            _ => None,
        };
        Ok(GameSnapshotDto {
            product: "Rusty D20".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            engine_revision: ENGINE_REVISION.to_owned(),
            ruleset_fingerprint: self.rules.fingerprint().to_owned(),
            revision: self.revision,
            saved: self.saved_revision == Some(self.revision),
            campaign,
            encounter,
        })
    }

    pub fn new_adventure(
        &mut self,
        expected_revision: u64,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(expected_revision)?;
        if self.campaign.is_some() {
            return Err(GameRuntimeError::InvalidCommand(
                "an adventure is already active".to_owned(),
            ));
        }
        self.ensure_mutation_capacity(false, true)?;
        let mut session = D20Session::new_with_loadout(
            self.rules.clone(),
            RngSeed::new(0xD20_2026),
            vec![
                character_seed(PLAYER, "Mara Venn", 18, 14, vec![]),
                character_seed(
                    OPPONENT,
                    "Iron Warden",
                    14,
                    12,
                    vec![AffinitySeed {
                        damage_type: id("slashing")?,
                        affinity: DamageAffinity::Resistant,
                    }],
                ),
            ],
            vec![
                InventorySeed {
                    owner: PLAYER,
                    maximum_items: 2,
                },
                InventorySeed {
                    owner: OPPONENT,
                    maximum_items: 1,
                },
            ],
            vec![StorageSeed {
                entity: CAMP_STASH,
                name: "Camp stash".to_owned(),
                maximum_items: 8,
            }],
            product_armor_items()?,
        )?;
        session.equip_armor(
            OPPONENT,
            OPPONENT_ARMOR,
            &id("chain-armor")?,
            operation("equip-warden-chain")?,
        )?;
        equip_initial_player_loadout(&mut session)?;
        self.campaign = Some(CampaignState {
            phase: CampaignPhase::Camp,
            active_encounter_id: None,
            turn_owner: None,
            outcome: None,
        });
        self.session = Some(session);
        self.pending = None;
        self.log.clear();
        self.next_log_id = 1;
        self.next_operation = 1;
        self.bump_revision()?;
        self.saved_revision = None;
        self.push_log(
            GameLogKindDto::System,
            "Adventure",
            "Mara Venn prepares at the Warden's Gate camp.",
            vec![
                "Starter Core + Steel Guard authored packages compiled by Rust.".to_owned(),
                "The Iron Warden encounter is available from camp.".to_owned(),
            ],
        )?;
        self.snapshot()
    }

    pub fn enter_encounter(
        &mut self,
        request: EnterEncounterRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_mutation_capacity(false, true)?;
        if request.encounter_id != ENCOUNTER_ID {
            return Err(GameRuntimeError::InvalidCommand(format!(
                "unknown encounter {}",
                request.encounter_id
            )));
        }
        let campaign = self
            .campaign
            .as_ref()
            .ok_or(GameRuntimeError::NoEncounter)?;
        if campaign.phase != CampaignPhase::Camp {
            return Err(GameRuntimeError::InvalidCommand(
                "an encounter can only be entered from camp".to_owned(),
            ));
        }
        if campaign.outcome.is_some() {
            return Err(GameRuntimeError::InvalidCommand(
                "the Iron Warden encounter has already been resolved".to_owned(),
            ));
        }
        let campaign = self
            .campaign
            .as_mut()
            .expect("campaign was validated before mutation");
        campaign.phase = CampaignPhase::Encounter;
        campaign.active_encounter_id = Some(ENCOUNTER_ID.to_owned());
        campaign.turn_owner = Some(EncounterTurnOwner::Player);
        campaign.outcome = None;
        self.bump_revision()?;
        self.saved_revision = None;
        self.push_log(
            GameLogKindDto::System,
            "Encounter",
            "Mara Venn faces the Iron Warden.",
            vec![
                "Iron Warden's chain armor and slashing resistance are active sources.".to_owned(),
            ],
        )?;
        self.snapshot()
    }

    pub fn equip_item(
        &mut self,
        request: EquipItemRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_camp_phase()?;
        self.ensure_mutation_capacity(true, true)?;
        let item = entity(request.item_id)?;
        let armor = product_loadout_armor(item)?;
        let definition = self
            .rules
            .armor(&armor)
            .expect("fixed product armor exists in the starter ruleset")
            .clone();
        if request.slot_id != definition.slot.as_str() {
            return Err(GameRuntimeError::InvalidEquipmentSlot {
                requested: request.slot_id,
                required: definition.slot.to_string(),
            });
        }
        let serial = self.next_operation;
        self.session_mut()?.equip_armor(
            PLAYER,
            item,
            &armor,
            operation(&format!("equip-item-{serial}"))?,
        )?;
        self.next_operation = serial + 1;
        self.bump_revision()?;
        self.saved_revision = None;
        self.push_log(
            GameLogKindDto::System,
            "Loadout",
            &format!("Equipped {}.", humanize(armor.as_str())),
            vec![format!(
                "{} now occupies the {} slot.",
                humanize(armor.as_str()),
                humanize(definition.slot.as_str())
            )],
        )?;
        self.snapshot()
    }

    pub fn unequip_item(
        &mut self,
        request: UnequipItemRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_camp_phase()?;
        self.ensure_mutation_capacity(true, true)?;
        let item = entity(request.item_id)?;
        let armor = product_loadout_armor(item)?;
        let serial = self.next_operation;
        self.session_mut()?.unequip_armor(
            PLAYER,
            item,
            operation(&format!("unequip-item-{serial}"))?,
        )?;
        self.next_operation = serial + 1;
        self.bump_revision()?;
        self.saved_revision = None;
        self.push_log(
            GameLogKindDto::System,
            "Loadout",
            &format!("Unequipped {}.", humanize(armor.as_str())),
            vec!["The item remains in Mara Venn's inventory.".to_owned()],
        )?;
        self.snapshot()
    }

    pub fn transfer_item(
        &mut self,
        request: TransferItemRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_camp_phase()?;
        self.ensure_mutation_capacity(true, true)?;
        let item = entity(request.item_id)?;
        let armor = product_loadout_armor(item)?;
        let from_owner = entity(request.from_owner_id)?;
        let to_owner = entity(request.to_owner_id)?;
        if !matches!(
            (from_owner, to_owner),
            (PLAYER, CAMP_STASH) | (CAMP_STASH, PLAYER)
        ) {
            return Err(GameRuntimeError::InvalidContainment(
                "loadout transfers are limited to Mara Venn and the camp stash".to_owned(),
            ));
        }
        let serial = self.next_operation;
        self.session_mut()?.transfer_armor(
            item,
            from_owner,
            to_owner,
            operation(&format!("transfer-item-{serial}"))?,
        )?;
        self.next_operation = serial + 1;
        self.bump_revision()?;
        self.saved_revision = None;
        let destination = if to_owner == PLAYER {
            "Mara Venn's inventory"
        } else {
            "the camp stash"
        };
        self.push_log(
            GameLogKindDto::System,
            "Loadout",
            &format!("Moved {} to {destination}.", humanize(armor.as_str())),
            vec![format!(
                "Canonical containment now points to entity {}.",
                to_owner.raw()
            )],
        )?;
        self.snapshot()
    }

    pub fn preview_action(
        &mut self,
        request: PreviewActionRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_encounter_phase()?;
        self.ensure_turn_owner(EncounterTurnOwner::Player)?;
        self.ensure_mutation_capacity(true, false)?;
        if self.pending.is_some() {
            return Err(GameRuntimeError::InvalidCommand(
                "resolve the current action preview before choosing another action".to_owned(),
            ));
        }
        let actor = entity(request.actor_id)?;
        let target = entity(request.target_id)?;
        if actor != PLAYER || target != OPPONENT {
            return Err(GameRuntimeError::InvalidCommand(
                "this encounter only permits Mara Venn to target the Iron Warden".to_owned(),
            ));
        }
        let action = id(&request.action_id)?;
        let serial = self.next_operation;
        let operation = operation(&format!("action-{serial}"))?;
        let preview = self
            .session()?
            .preview_action(actor, target, &action, operation)?;
        self.next_operation = self
            .next_operation
            .checked_add(1)
            .ok_or(GameRuntimeError::CounterOverflow)?;
        self.pending = Some(PendingAction {
            serial,
            token: format!("preview-{serial}"),
            preview,
        });
        self.bump_revision()?;
        self.saved_revision = None;
        self.snapshot()
    }

    pub fn apply_reaction(
        &mut self,
        request: ApplyReactionRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        let snapshot = staged.apply_reaction_inner(request)?;
        *self = staged;
        Ok(snapshot)
    }

    fn apply_reaction_inner(
        &mut self,
        request: ApplyReactionRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_encounter_phase()?;
        self.ensure_mutation_capacity(false, true)?;
        let pending = self.require_pending(&request.preview_token)?.clone();
        let reaction = id(&request.reaction_id)?;
        let receipt = self.session_mut()?.apply_reaction(
            &pending.preview,
            &reaction,
            effect_instance(&format!("reaction-{}", pending.serial))?,
        )?;
        let fresh = self.session()?.preview_action(
            pending.preview.actor(),
            pending.preview.target(),
            pending.preview.action(),
            pending.preview.operation().clone(),
        )?;
        self.pending = Some(PendingAction {
            preview: fresh,
            ..pending
        });
        self.bump_revision()?;
        self.saved_revision = None;
        self.log_reaction(&receipt)?;
        self.snapshot()
    }

    pub fn apply_action(
        &mut self,
        request: ApplyActionRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        let snapshot = staged.apply_action_inner(request)?;
        *self = staged;
        Ok(snapshot)
    }

    fn apply_action_inner(
        &mut self,
        request: ApplyActionRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_encounter_phase()?;
        self.ensure_mutation_capacity(false, true)?;
        let pending = self.require_pending(&request.preview_token)?.clone();
        let turn_owner = self
            .campaign
            .as_ref()
            .and_then(|campaign| campaign.turn_owner)
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "active encounter is missing its turn owner".to_owned(),
                )
            })?;
        let expected_actor = match turn_owner {
            EncounterTurnOwner::Player => PLAYER,
            EncounterTurnOwner::Opposition => OPPONENT,
        };
        if pending.preview.actor() != expected_actor {
            return Err(GameRuntimeError::InvalidCommand(
                "the pending action does not belong to the current turn owner".to_owned(),
            ));
        }
        let action_definition = self
            .rules
            .action(pending.preview.action())
            .ok_or_else(|| GameRuntimeError::InvalidCommand("unknown pending action".to_owned()))?
            .clone();
        let effect_instance = action_definition
            .effect
            .as_ref()
            .map(|_| effect_instance(&format!("action-effect-{}", pending.serial)))
            .transpose()?;
        let receipt = self.session_mut()?.apply_action(ApplyActionRequest {
            preview: pending.preview,
            effect_instance,
        })?;
        self.pending = None;

        let mut details = vec![
            format!(
                "d20 {} + modifier {} = {} against defense {}.",
                receipt.d20, receipt.ability_modifier, receipt.total, receipt.defense
            ),
            format!("Deterministic roll index {}.", receipt.roll_index),
        ];
        if let Some(damage) = &receipt.damage {
            for part in &damage.parts {
                details.push(format!(
                    "{} damage requested; {} applied to vitality.",
                    part.original.get(),
                    part.applied.get()
                ));
            }
            for decision in &damage.decisions {
                details.push(format!(
                    "{}: {} ({}).",
                    source_label(&decision.source),
                    damage_decision_label(&decision.kind),
                    outcome_label(decision.outcome)
                ));
            }
        }
        if let Some(expires) = receipt.expires_at_turn {
            details.push(format!(
                "{} applied until turn {expires}.",
                humanize(
                    action_definition
                        .effect
                        .as_ref()
                        .expect("expiry requires an effect")
                        .as_str()
                )
            ));
        }
        let kind = if receipt.hit {
            GameLogKindDto::Hit
        } else {
            GameLogKindDto::Miss
        };
        let outcome = if receipt.hit { "hit" } else { "missed" };
        let actor_name = self.character_name(receipt.actor)?;
        let target_name = self.character_name(receipt.target)?;
        self.push_log(
            kind,
            &humanize(receipt.action.as_str()),
            &format!("{actor_name} {outcome} {target_name}."),
            details,
        )?;

        if self.vitality(receipt.target)? == 0 {
            let encounter_outcome = if receipt.target == OPPONENT {
                EncounterOutcome::Victory
            } else {
                EncounterOutcome::Defeat
            };
            self.complete_encounter(encounter_outcome)?;
        } else {
            match turn_owner {
                EncounterTurnOwner::Player => {
                    self.campaign_mut()?.turn_owner = Some(EncounterTurnOwner::Opposition);
                }
                EncounterTurnOwner::Opposition => {
                    let serial = self.next_operation;
                    let next_turn = self
                        .session()?
                        .current_turn()
                        .checked_add(1)
                        .ok_or(GameRuntimeError::CounterOverflow)?;
                    let turn_receipt = self
                        .session_mut()?
                        .advance_turn(next_turn, operation(&format!("advance-round-{serial}"))?)?;
                    self.next_operation = self
                        .next_operation
                        .checked_add(1)
                        .ok_or(GameRuntimeError::CounterOverflow)?;
                    self.campaign_mut()?.turn_owner = Some(EncounterTurnOwner::Player);
                    self.push_log(
                        GameLogKindDto::Turn,
                        "Round",
                        &format!(
                            "The encounter advanced from round {} to {}.",
                            turn_receipt.before, turn_receipt.after
                        ),
                        vec![format!(
                            "{} scheduled effect(s) expired before Mara's next turn.",
                            turn_receipt.expired.len()
                        )],
                    )?;
                }
            }
        }
        self.bump_revision()?;
        self.saved_revision = None;
        self.snapshot()
    }

    pub fn begin_opposition_turn(
        &mut self,
        expected_revision: u64,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        let snapshot = staged.begin_opposition_turn_inner(expected_revision)?;
        *self = staged;
        Ok(snapshot)
    }

    fn begin_opposition_turn_inner(
        &mut self,
        expected_revision: u64,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(expected_revision)?;
        self.ensure_encounter_phase()?;
        self.ensure_turn_owner(EncounterTurnOwner::Opposition)?;
        self.ensure_mutation_capacity(true, true)?;
        if self.pending.is_some() {
            return Err(GameRuntimeError::InvalidCommand(
                "the opposition action is already pending".to_owned(),
            ));
        }
        let actions = self
            .rules
            .actions()
            .map(|action| action.id.clone())
            .collect::<Vec<_>>();
        let upper = u32::try_from(actions.len()).map_err(|_| {
            GameRuntimeError::InvalidState(
                "the opposition action catalog does not fit deterministic choice".to_owned(),
            )
        })?;
        let index = self
            .session()?
            .deterministic_choice_index("iron-warden-action", upper)
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "the opposition has no admitted action choices".to_owned(),
                )
            })?;
        let index = usize::try_from(index).expect("u32 choice index fits usize");
        let action = actions[index].clone();
        let serial = self.next_operation;
        let operation = operation(&format!("opposition-action-{serial}"))?;
        let preview = self
            .session()?
            .preview_action(OPPONENT, PLAYER, &action, operation)?;
        self.next_operation = self
            .next_operation
            .checked_add(1)
            .ok_or(GameRuntimeError::CounterOverflow)?;
        self.pending = Some(PendingAction {
            serial,
            token: format!("preview-{serial}"),
            preview,
        });
        self.bump_revision()?;
        self.saved_revision = None;
        self.push_log(
            GameLogKindDto::Turn,
            "Opposition",
            &format!(
                "Iron Warden prepares {}.",
                humanize(action.as_str())
            ),
            vec![format!(
                "Deterministic enemy policy selected catalog choice {} of {} from Rust-owned session state.",
                index + 1,
                actions.len()
            )],
        )?;
        self.snapshot()
    }

    pub fn return_to_camp(
        &mut self,
        expected_revision: u64,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        let snapshot = staged.return_to_camp_inner(expected_revision)?;
        *self = staged;
        Ok(snapshot)
    }

    fn return_to_camp_inner(
        &mut self,
        expected_revision: u64,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(expected_revision)?;
        self.ensure_outcome_phase()?;
        self.ensure_mutation_capacity(true, true)?;
        let outcome = self
            .campaign
            .as_ref()
            .and_then(|campaign| campaign.outcome)
            .ok_or_else(|| {
                GameRuntimeError::InvalidState("outcome phase is missing its result".to_owned())
            })?;
        let mut details = Vec::new();
        if outcome == EncounterOutcome::Defeat {
            let serial = self.next_operation;
            let receipt = self.session_mut()?.restore_vitality(
                PLAYER,
                DEFEAT_RECOVERY_VITALITY,
                operation(&format!("camp-recovery-{serial}"))?,
            )?;
            self.next_operation = self
                .next_operation
                .checked_add(1)
                .ok_or(GameRuntimeError::CounterOverflow)?;
            details.push(format!(
                "Camp recovery restored {} vitality; Mara returns with {}/{} vitality.",
                receipt.applied_amount.get(),
                receipt.after.get(),
                STARTING_VITALITY
            ));
        } else {
            details.push("Mara keeps her remaining vitality and resources.".to_owned());
            details.push("Warden chain armor remains in canonical camp storage.".to_owned());
        }
        {
            let campaign = self.campaign_mut()?;
            campaign.phase = CampaignPhase::Camp;
            campaign.active_encounter_id = None;
            campaign.turn_owner = None;
        }
        self.bump_revision()?;
        self.saved_revision = None;
        self.push_log(
            GameLogKindDto::System,
            "Camp",
            "The encounter consequence is now part of the durable camp state.",
            details,
        )?;
        self.snapshot()
    }

    fn project_campaign(
        &self,
        campaign: &CampaignState,
        session: &D20Session,
    ) -> Result<CampaignDto, GameRuntimeError> {
        Ok(CampaignDto {
            id: ADVENTURE_ID.to_owned(),
            title: ADVENTURE_TITLE.to_owned(),
            phase: match campaign.phase {
                CampaignPhase::Camp => CampaignPhaseDto::Camp,
                CampaignPhase::Encounter => CampaignPhaseDto::Encounter,
                CampaignPhase::Outcome => CampaignPhaseDto::Outcome,
            },
            hero: self.project_character(session, PLAYER, "Steel Adept")?,
            loadout: self.project_loadout(session)?,
            active_encounter_id: campaign.active_encounter_id.clone(),
            available_encounters: if campaign.outcome.is_none() {
                vec![EncounterChoiceDto {
                    id: ENCOUNTER_ID.to_owned(),
                    title: ENCOUNTER_TITLE.to_owned(),
                    summary: "Challenge the armored sentinel guarding the mountain pass."
                        .to_owned(),
                }]
            } else {
                Vec::new()
            },
            latest_outcome: campaign.outcome.map(|outcome| match outcome {
                EncounterOutcome::Victory => CampaignOutcomeDto {
                    kind: EncounterOutcomeKindDto::Victory,
                    encounter_id: ENCOUNTER_ID.to_owned(),
                    title: "The Iron Warden defeated".to_owned(),
                    summary: "Mara prevailed; her remaining vitality and resources carry forward."
                        .to_owned(),
                    reward_item_id: Some(OPPONENT_ARMOR.raw()),
                    reward: Some("Warden chain armor".to_owned()),
                },
                EncounterOutcome::Defeat => CampaignOutcomeDto {
                    kind: EncounterOutcomeKindDto::Defeat,
                    encounter_id: ENCOUNTER_ID.to_owned(),
                    title: "Mara was defeated".to_owned(),
                    summary: "No reward was granted; returning to camp applies bounded recovery."
                        .to_owned(),
                    reward_item_id: None,
                    reward: None,
                },
            }),
        })
    }

    fn project_loadout(&self, session: &D20Session) -> Result<LoadoutDto, GameRuntimeError> {
        let inventory = session.inventory_view(PLAYER)?;
        let equipment = session
            .entities()
            .component::<EquipmentComponent>(PLAYER)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState("player equipment component is missing".to_owned())
            })?;
        let equipped_by_item = equipment
            .assignments()
            .iter()
            .map(|assignment| (assignment.item, assignment.slot.to_string()))
            .collect::<BTreeMap<_, _>>();
        let mut inventory_items = inventory
            .unique_items()
            .iter()
            .map(|item| {
                self.project_loadout_item(
                    session,
                    item.entity,
                    equipped_by_item.get(&item.entity).cloned(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let capacity = inventory
            .capacity()
            .iter()
            .find(|usage| usage.metric.as_str() == "carried-items")
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "player carried-items capacity is missing".to_owned(),
                )
            })?;
        let maximum = capacity.maximum.ok_or_else(|| {
            GameRuntimeError::InvalidState("player carried-items maximum is missing".to_owned())
        })?;
        let slot_count = usize::try_from(maximum).map_err(|_| {
            GameRuntimeError::InvalidState(
                "player inventory capacity does not fit memory".to_owned(),
            )
        })?;
        if inventory_items.len() > slot_count {
            return Err(GameRuntimeError::InvalidState(
                "player inventory exceeds its projected slot count".to_owned(),
            ));
        }
        let mut inventory_slots = inventory_items
            .drain(..)
            .map(Some)
            .collect::<Vec<Option<LoadoutItemDto>>>();
        inventory_slots.resize(slot_count, None);

        let mut slot_items = BTreeMap::new();
        for assignment in equipment.assignments() {
            slot_items.insert(
                assignment.slot.to_string(),
                self.project_loadout_item(
                    session,
                    assignment.item,
                    Some(assignment.slot.to_string()),
                )?,
            );
        }
        let equipment_slots = self
            .rules
            .armors()
            .map(|armor| armor.slot.clone())
            .fold(BTreeMap::<String, D20Id>::new(), |mut slots, slot| {
                slots.entry(slot.to_string()).or_insert(slot);
                slots
            })
            .into_iter()
            .map(|(slot, definition)| EquipmentSlotDto {
                id: slot.clone(),
                label: humanize(definition.as_str()),
                equipped: slot_items.remove(&slot),
            })
            .collect::<Vec<_>>();

        let stash = session.inventory_view(CAMP_STASH)?;
        let stash_items = stash
            .unique_items()
            .iter()
            .map(|item| self.project_loadout_item(session, item.entity, None))
            .collect::<Result<Vec<_>, _>>()?;
        let defense = StatService::evaluate(
            session.entities(),
            self.rules.mechanics(),
            PLAYER,
            &defense_stat_id(&id("armor")?),
            &operation("project-loadout")?,
            &[],
        )
        .map_err(D20SessionError::from)?;
        Ok(LoadoutDto {
            owner_id: PLAYER.raw(),
            stash_owner_id: CAMP_STASH.raw(),
            inventory_slots,
            equipment_slots,
            stash_items,
            capacity: LoadoutCapacityDto {
                metric: "carried-items".to_owned(),
                used: capacity.used,
                maximum,
            },
            armor_defense: defense.value.get(),
            armor_defense_sources: defense
                .decisions
                .iter()
                .map(|decision| {
                    format!(
                        "{}: {} ({})",
                        source_label(&decision.source),
                        stat_contribution_label(decision.contribution.as_ref()),
                        outcome_label(decision.outcome)
                    )
                })
                .collect(),
        })
    }

    fn project_loadout_item(
        &self,
        session: &D20Session,
        item: EntityId,
        equipped_slot_id: Option<String>,
    ) -> Result<LoadoutItemDto, GameRuntimeError> {
        let armor = product_loadout_armor(item)?;
        let definition = self.rules.armor(&armor).ok_or_else(|| {
            GameRuntimeError::InvalidState(format!("loadout armor {armor} is missing"))
        })?;
        let component = session
            .entities()
            .component::<ItemComponent>(item)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "loadout item {} is missing ItemComponent",
                    item.raw()
                ))
            })?;
        let expected_definition = format!("armor.{armor}");
        if component.definition().as_str() != expected_definition {
            return Err(GameRuntimeError::InvalidState(format!(
                "loadout item {} definition is inconsistent",
                item.raw()
            )));
        }
        let item_name = session
            .entities()
            .core(item)
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "loadout item {} core facts are missing",
                    item.raw()
                ))
            })?
            .name
            .clone();
        let (icon, rarity) = match armor.as_str() {
            "chain-armor" => ("🛡️", LoadoutRarityDto::Uncommon),
            "buckler" => ("◈", LoadoutRarityDto::Common),
            _ => {
                return Err(GameRuntimeError::InvalidState(format!(
                    "unsupported product armor {armor}"
                )));
            }
        };
        Ok(LoadoutItemDto {
            entity_id: item.raw(),
            definition_id: armor.to_string(),
            name: item_name,
            icon: icon.to_owned(),
            rarity,
            quantity: 1,
            equipment_slot_id: definition.slot.to_string(),
            equipped_slot_id,
        })
    }

    fn project_encounter(
        &self,
        campaign: &CampaignState,
        session: &D20Session,
    ) -> Result<EncounterDto, GameRuntimeError> {
        Ok(EncounterDto {
            turn: session.current_turn(),
            next_roll: session.next_roll_index(),
            player_id: PLAYER.raw(),
            turn_owner: campaign.turn_owner.map(|owner| match owner {
                EncounterTurnOwner::Player => EncounterTurnOwnerDto::Player,
                EncounterTurnOwner::Opposition => EncounterTurnOwnerDto::Opposition,
            }),
            characters: vec![
                self.project_character(session, PLAYER, "Steel Adept")?,
                self.project_character(session, OPPONENT, "Armored Sentinel")?,
            ],
            actions: self
                .rules
                .actions()
                .map(|action| ActionDto {
                    id: action.id.to_string(),
                    label: humanize(action.id.as_str()),
                    ability: humanize(action.ability.as_str()),
                    defense: humanize(action.defense.as_str()),
                    damage: format!(
                        "{}d{}{}{} {}",
                        action.damage.dice,
                        action.damage.sides,
                        if action.damage.bonus >= 0 { "+" } else { "" },
                        action.damage.bonus,
                        humanize(action.damage.kind.as_str())
                    ),
                    effect: action
                        .effect
                        .as_ref()
                        .map(|effect| humanize(effect.as_str())),
                })
                .collect(),
            pending_action: self
                .pending
                .as_ref()
                .map(|pending| self.project_pending(pending)),
            log: self.log.clone(),
        })
    }

    fn project_character(
        &self,
        session: &D20Session,
        entity: EntityId,
        title: &str,
    ) -> Result<CharacterDto, GameRuntimeError> {
        let core = session.entities().core(entity).ok_or_else(|| {
            GameRuntimeError::InvalidState("character entity is missing".to_owned())
        })?;
        let tracks = session
            .entities()
            .component::<TracksComponent>(entity)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState("vitality component is missing".to_owned())
            })?;
        let vitality = tracks
            .values()
            .iter()
            .find(|value| value.track().as_str() == "vitality")
            .ok_or_else(|| {
                GameRuntimeError::InvalidState("vitality track is missing".to_owned())
            })?;
        let resources = session
            .entities()
            .component::<ActionResourcesComponent>(entity)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState("resources component is missing".to_owned())
            })?
            .resources()
            .iter()
            .map(|resource| {
                let maximum = self
                    .rules
                    .resource(resource.id())
                    .ok_or_else(|| {
                        GameRuntimeError::InvalidState(format!(
                            "resource definition {} is missing",
                            resource.id()
                        ))
                    })?
                    .maximum;
                Ok(ResourceDto {
                    id: resource.id().to_string(),
                    label: humanize(resource.id().as_str()),
                    current: resource.current(),
                    maximum,
                })
            })
            .collect::<Result<Vec<_>, GameRuntimeError>>()?;
        let effects: Vec<String> = session
            .entities()
            .component::<ScheduledEffectsComponent>(entity)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState("scheduled effects component is missing".to_owned())
            })?
            .effects()
            .iter()
            .map(|effect| {
                format!(
                    "{} · through turn {}",
                    humanize(effect.definition().as_str()),
                    effect.expires_at_turn()
                )
            })
            .collect();
        let active_count = session
            .entities()
            .component::<ActiveEffectsComponent>(entity)?
            .map_or(0, |active| active.effects().len());
        if active_count != effects.len() {
            return Err(GameRuntimeError::InvalidState(
                "active and scheduled effects diverged".to_owned(),
            ));
        }
        Ok(CharacterDto {
            id: entity.raw(),
            name: core.name.clone(),
            title: title.to_owned(),
            level: 1,
            health_current: vitality.current().get(),
            health_maximum: i64::from(STARTING_VITALITY),
            resources,
            effects,
        })
    }

    fn project_pending(&self, pending: &PendingAction) -> PendingActionDto {
        PendingActionDto {
            token: pending.token.clone(),
            actor_id: pending.preview.actor().raw(),
            target_id: pending.preview.target().raw(),
            action_id: pending.preview.action().to_string(),
            action_label: humanize(pending.preview.action().as_str()),
            ability_score: pending.preview.ability_score(),
            ability_modifier: pending.preview.ability_modifier(),
            defense: pending.preview.defense().value.get(),
            defense_sources: pending
                .preview
                .defense()
                .decisions
                .iter()
                .map(|decision| {
                    format!(
                        "{}: {} ({})",
                        source_label(&decision.source),
                        stat_contribution_label(decision.contribution.as_ref()),
                        outcome_label(decision.outcome)
                    )
                })
                .collect(),
            reactions: pending
                .preview
                .reactions()
                .iter()
                .map(|reaction| ReactionDto {
                    id: reaction.reaction().to_string(),
                    label: humanize(reaction.reaction().as_str()),
                    resource: humanize(reaction.resource().as_str()),
                    cost: reaction.cost(),
                    available: reaction.available(),
                    bonus: reaction.bonus(),
                    effect: humanize(reaction.effect().as_str()),
                })
                .collect(),
        }
    }

    fn log_reaction(&mut self, receipt: &ReactionReceipt) -> Result<(), GameRuntimeError> {
        let defender = self.character_name(receipt.target)?;
        self.push_log(
            GameLogKindDto::Reaction,
            &humanize(receipt.reaction.as_str()),
            &format!("{defender} raised a reaction before the roll."),
            vec![
                format!(
                    "{} {} → {}.",
                    humanize(receipt.resource.as_str()),
                    receipt.before,
                    receipt.after
                ),
                format!(
                    "Defense effect remains through turn {}.",
                    receipt.expires_at_turn
                ),
                format!(
                    "{} attributed source activation(s).",
                    receipt.effect.activated_sources.len()
                ),
            ],
        )
    }

    fn complete_encounter(&mut self, outcome: EncounterOutcome) -> Result<(), GameRuntimeError> {
        let mut details = Vec::new();
        match outcome {
            EncounterOutcome::Victory => {
                let session = self.session.as_mut().ok_or(GameRuntimeError::NoEncounter)?;
                transfer_victory_reward(session, &mut self.next_operation)?;
                details.push(
                    "Warden chain armor was unequipped and transferred into canonical camp storage."
                        .to_owned(),
                );
                details.push(format!(
                    "Reward item entity {} can be inspected after returning to camp.",
                    OPPONENT_ARMOR.raw()
                ));
            }
            EncounterOutcome::Defeat => {
                details.push(
                    "Mara reached zero vitality; no reward or inventory mutation occurred."
                        .to_owned(),
                );
                details.push(
                    "Return to camp applies the explicit bounded recovery consequence.".to_owned(),
                );
            }
        }
        {
            let campaign = self.campaign_mut()?;
            campaign.phase = CampaignPhase::Outcome;
            campaign.turn_owner = None;
            campaign.outcome = Some(outcome);
        }
        let (source, text) = match outcome {
            EncounterOutcome::Victory => (
                "Victory",
                "The Iron Warden falls and yields the Warden chain armor.",
            ),
            EncounterOutcome::Defeat => (
                "Defeat",
                "Mara Venn falls and must recover before continuing.",
            ),
        };
        self.push_log(GameLogKindDto::System, source, text, details)
    }

    fn push_log(
        &mut self,
        kind: GameLogKindDto,
        source: &str,
        text: &str,
        mut details: Vec<String>,
    ) -> Result<(), GameRuntimeError> {
        if details.len() > MAX_LOG_DETAILS {
            let omitted = details.len() - (MAX_LOG_DETAILS - 1);
            details.truncate(MAX_LOG_DETAILS - 1);
            details.push(format!("{omitted} additional receipt decision(s) omitted."));
        }
        let turn = self.session.as_ref().map_or(0, D20Session::current_turn);
        let entry = GameLogEntryDto {
            id: self.next_log_id,
            turn,
            kind,
            source: source.to_owned(),
            text: text.to_owned(),
            details,
        };
        self.next_log_id = self
            .next_log_id
            .checked_add(1)
            .ok_or(GameRuntimeError::CounterOverflow)?;
        self.log.push(entry);
        if self.log.len() > MAX_LOG_ENTRIES {
            self.log.remove(0);
        }
        Ok(())
    }

    fn session(&self) -> Result<&D20Session, GameRuntimeError> {
        self.session.as_ref().ok_or(GameRuntimeError::NoEncounter)
    }

    fn session_mut(&mut self) -> Result<&mut D20Session, GameRuntimeError> {
        self.session.as_mut().ok_or(GameRuntimeError::NoEncounter)
    }

    fn campaign_mut(&mut self) -> Result<&mut CampaignState, GameRuntimeError> {
        self.campaign.as_mut().ok_or(GameRuntimeError::NoEncounter)
    }

    fn character_name(&self, entity: EntityId) -> Result<String, GameRuntimeError> {
        self.session()?
            .entities()
            .core(entity)
            .map(|core| core.name.clone())
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "character entity {} is missing",
                    entity.raw()
                ))
            })
    }

    fn vitality(&self, entity: EntityId) -> Result<i64, GameRuntimeError> {
        self.session()?
            .entities()
            .component::<TracksComponent>(entity)?
            .and_then(|tracks| {
                tracks
                    .values()
                    .iter()
                    .find(|value| value.track().as_str() == "vitality")
                    .map(|value| value.current().get())
            })
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "entity {} vitality is missing",
                    entity.raw()
                ))
            })
    }

    fn require_pending(&self, token: &str) -> Result<&PendingAction, GameRuntimeError> {
        self.pending
            .as_ref()
            .filter(|pending| pending.token == token)
            .ok_or_else(|| {
                GameRuntimeError::StaleCommand(
                    "the selected action preview is no longer current".to_owned(),
                )
            })
    }

    fn ensure_encounter_phase(&self) -> Result<(), GameRuntimeError> {
        match self.campaign.as_ref().map(|campaign| campaign.phase) {
            Some(CampaignPhase::Encounter) => Ok(()),
            Some(CampaignPhase::Camp | CampaignPhase::Outcome) => {
                Err(GameRuntimeError::WrongPhase(
                    "this command is only available during an active encounter".to_owned(),
                ))
            }
            None => Err(GameRuntimeError::NoEncounter),
        }
    }

    fn ensure_camp_phase(&self) -> Result<(), GameRuntimeError> {
        match self.campaign.as_ref().map(|campaign| campaign.phase) {
            Some(CampaignPhase::Camp) => Ok(()),
            Some(CampaignPhase::Encounter | CampaignPhase::Outcome) => {
                Err(GameRuntimeError::WrongPhase(
                    "loadout changes are only available at camp".to_owned(),
                ))
            }
            None => Err(GameRuntimeError::NoEncounter),
        }
    }

    fn ensure_outcome_phase(&self) -> Result<(), GameRuntimeError> {
        match self.campaign.as_ref().map(|campaign| campaign.phase) {
            Some(CampaignPhase::Outcome) => Ok(()),
            Some(CampaignPhase::Camp | CampaignPhase::Encounter) => {
                Err(GameRuntimeError::WrongPhase(
                    "return to camp is only available after an encounter outcome".to_owned(),
                ))
            }
            None => Err(GameRuntimeError::NoEncounter),
        }
    }

    fn ensure_turn_owner(&self, expected: EncounterTurnOwner) -> Result<(), GameRuntimeError> {
        let actual = self
            .campaign
            .as_ref()
            .and_then(|campaign| campaign.turn_owner)
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "active encounter is missing its turn owner".to_owned(),
                )
            })?;
        if actual != expected {
            let owner = match actual {
                EncounterTurnOwner::Player => "player",
                EncounterTurnOwner::Opposition => "opposition",
            };
            return Err(GameRuntimeError::WrongPhase(format!(
                "this command is not legal during the {owner} turn"
            )));
        }
        Ok(())
    }

    fn ensure_revision(&self, expected: u64) -> Result<(), GameRuntimeError> {
        if expected != self.revision {
            return Err(GameRuntimeError::StaleCommand(format!(
                "expected revision {expected}, current revision is {}",
                self.revision
            )));
        }
        Ok(())
    }

    fn ensure_mutation_capacity(
        &self,
        reserves_operation: bool,
        reserves_log: bool,
    ) -> Result<(), GameRuntimeError> {
        if self.revision == u64::MAX
            || (reserves_operation && self.next_operation == u64::MAX)
            || (reserves_log && self.next_log_id == u64::MAX)
        {
            return Err(GameRuntimeError::CounterOverflow);
        }
        Ok(())
    }

    fn bump_revision(&mut self) -> Result<(), GameRuntimeError> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(GameRuntimeError::CounterOverflow)?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveEnvelope {
    schema_version: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct LegacyGameSave {
    #[serde(rename = "schemaVersion")]
    _schema_version: u32,
    revision: u64,
    next_operation: u64,
    next_log_id: u64,
    log: Vec<GameLogEntryDto>,
    session: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CampaignSave {
    adventure_id: String,
    phase: CampaignPhase,
    active_encounter_id: Option<String>,
    turn_owner: Option<EncounterTurnOwner>,
    outcome: Option<EncounterOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct LegacyCampaignSave {
    adventure_id: String,
    phase: LegacyCampaignPhase,
    active_encounter_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum LegacyCampaignPhase {
    Camp,
    Encounter,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct LegacyCampaignGameSave {
    #[serde(rename = "schemaVersion")]
    _schema_version: u32,
    revision: u64,
    next_operation: u64,
    next_log_id: u64,
    log: Vec<GameLogEntryDto>,
    campaign: LegacyCampaignSave,
    session: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct GameSave {
    schema_version: u32,
    revision: u64,
    next_operation: u64,
    next_log_id: u64,
    log: Vec<GameLogEntryDto>,
    campaign: CampaignSave,
    session: serde_json::Value,
}

fn validate_campaign_save(save: CampaignSave) -> Result<CampaignState, GameRuntimeError> {
    if save.adventure_id != ADVENTURE_ID {
        return Err(GameRuntimeError::InvalidSave(format!(
            "unknown adventure {}",
            save.adventure_id
        )));
    }
    let valid_phase = match save.phase {
        CampaignPhase::Camp => save.active_encounter_id.is_none() && save.turn_owner.is_none(),
        CampaignPhase::Encounter => {
            save.active_encounter_id.as_deref() == Some(ENCOUNTER_ID)
                && save.turn_owner.is_some()
                && save.outcome.is_none()
        }
        CampaignPhase::Outcome => {
            save.active_encounter_id.as_deref() == Some(ENCOUNTER_ID)
                && save.turn_owner.is_none()
                && save.outcome.is_some()
        }
    };
    if !valid_phase {
        return Err(GameRuntimeError::InvalidSave(
            "campaign phase and active encounter are inconsistent".to_owned(),
        ));
    }
    Ok(CampaignState {
        phase: save.phase,
        active_encounter_id: save.active_encounter_id,
        turn_owner: save.turn_owner,
        outcome: save.outcome,
    })
}

fn validate_legacy_campaign_save(
    save: LegacyCampaignSave,
) -> Result<CampaignState, GameRuntimeError> {
    let phase = match save.phase {
        LegacyCampaignPhase::Camp => CampaignPhase::Camp,
        LegacyCampaignPhase::Encounter => CampaignPhase::Encounter,
    };
    validate_campaign_save(CampaignSave {
        adventure_id: save.adventure_id,
        phase,
        active_encounter_id: save.active_encounter_id,
        turn_owner: (phase == CampaignPhase::Encounter).then_some(EncounterTurnOwner::Player),
        outcome: None,
    })
}

#[derive(Debug)]
pub enum GameRuntimeError {
    NoEncounter,
    PendingActionCannotBeSaved,
    StaleCommand(String),
    InvalidCommand(String),
    InvalidEquipmentSlot { requested: String, required: String },
    InvalidContainment(String),
    WrongPhase(String),
    InvalidState(String),
    InvalidSave(String),
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
            Self::PendingActionCannotBeSaved | Self::InvalidCommand(_) | Self::D20Identity(_) => {
                (ApiErrorKindDto::Invalid, false)
            }
            Self::InvalidSave(_) | Self::UnsupportedSaveSchema { .. } | Self::Save(_) => {
                (ApiErrorKindDto::Persistence, false)
            }
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
            Self::PendingActionCannotBeSaved => {
                write!(formatter, "resolve the pending action before saving")
            }
            Self::StaleCommand(message)
            | Self::InvalidCommand(message)
            | Self::InvalidContainment(message)
            | Self::WrongPhase(message)
            | Self::InvalidState(message)
            | Self::InvalidSave(message) => formatter.write_str(message),
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

fn starter_ruleset() -> Result<D20Ruleset, GameRuntimeError> {
    D20Ruleset::compile(vec![
        decode_package(STARTER_CORE)?,
        decode_package(STEEL_GUARD)?,
    ])
    .map_err(Into::into)
}

fn decode_package(input: &str) -> Result<AdmittedRulePackage, GameRuntimeError> {
    gameplay_rules::decode_canonical_rule_package(input.as_bytes())
        .map_err(|error| GameRuntimeError::InvalidSave(error.to_string()))
}

fn character_seed(
    entity: EntityId,
    name: &str,
    strength: i16,
    dexterity: i16,
    affinities: Vec<AffinitySeed>,
) -> CharacterSeed {
    CharacterSeed {
        entity,
        name: name.to_owned(),
        vitality: STARTING_VITALITY,
        abilities: vec![
            AbilityScore::new(D20Id::parse("constitution").expect("fixed id"), 14),
            AbilityScore::new(D20Id::parse("dexterity").expect("fixed id"), dexterity),
            AbilityScore::new(D20Id::parse("strength").expect("fixed id"), strength),
            AbilityScore::new(D20Id::parse("wisdom").expect("fixed id"), 12),
        ],
        resources: vec![
            ActionResource::new(D20Id::parse("focus").expect("fixed id"), 3),
            ActionResource::new(D20Id::parse("guard").expect("fixed id"), 2),
            ActionResource::new(D20Id::parse("resolve-points").expect("fixed id"), 2),
        ],
        affinities,
    }
}

fn product_armor_items() -> Result<Vec<ArmorItemSeed>, GameRuntimeError> {
    Ok(vec![
        ArmorItemSeed {
            entity: OPPONENT_ARMOR,
            owner: OPPONENT,
            name: "Warden chain armor".to_owned(),
            armor: id("chain-armor")?,
        },
        ArmorItemSeed {
            entity: PLAYER_CHAIN_ARMOR,
            owner: PLAYER,
            name: "Mara's chain armor".to_owned(),
            armor: id("chain-armor")?,
        },
        ArmorItemSeed {
            entity: PLAYER_BUCKLER,
            owner: PLAYER,
            name: "Mara's buckler".to_owned(),
            armor: id("buckler")?,
        },
        ArmorItemSeed {
            entity: STASH_BUCKLER,
            owner: CAMP_STASH,
            name: "Spare buckler".to_owned(),
            armor: id("buckler")?,
        },
    ])
}

fn install_product_loadout(session: &mut D20Session) -> Result<(), GameRuntimeError> {
    session.install_loadout(
        vec![
            InventorySeed {
                owner: PLAYER,
                maximum_items: 2,
            },
            InventorySeed {
                owner: OPPONENT,
                maximum_items: 1,
            },
        ],
        vec![StorageSeed {
            entity: CAMP_STASH,
            name: "Camp stash".to_owned(),
            maximum_items: 8,
        }],
        product_armor_items()?
            .into_iter()
            .filter(|item| item.entity != OPPONENT_ARMOR)
            .collect(),
    )?;
    equip_initial_player_loadout(session)
}

fn validate_product_state(
    session: &D20Session,
    campaign: &CampaignState,
) -> Result<(), GameRuntimeError> {
    if session.entities().total_count() != 7 {
        return Err(GameRuntimeError::InvalidSave(
            "the Warden's Gate entity set is inconsistent".to_owned(),
        ));
    }
    let inventory_owners = session
        .entities()
        .components::<InventoryComponent>()?
        .map(|(entity, _)| entity)
        .collect::<Vec<_>>();
    if inventory_owners != [PLAYER, OPPONENT, CAMP_STASH] {
        return Err(GameRuntimeError::InvalidSave(
            "the Warden's Gate inventory owners are inconsistent".to_owned(),
        ));
    }
    let item_entities = session
        .entities()
        .components::<ItemComponent>()?
        .map(|(entity, _)| entity)
        .collect::<Vec<_>>();
    if item_entities
        != [
            OPPONENT_ARMOR,
            PLAYER_CHAIN_ARMOR,
            PLAYER_BUCKLER,
            STASH_BUCKLER,
        ]
    {
        return Err(GameRuntimeError::InvalidSave(
            "the Warden's Gate item set is inconsistent".to_owned(),
        ));
    }

    validate_inventory(session, PLAYER, 2)?;
    validate_inventory(session, OPPONENT, 1)?;
    validate_inventory(session, CAMP_STASH, 8)?;
    for item in [
        OPPONENT_ARMOR,
        PLAYER_CHAIN_ARMOR,
        PLAYER_BUCKLER,
        STASH_BUCKLER,
    ] {
        if !matches!(
            session.entities().contained_in(item),
            Some(OPPONENT) | Some(PLAYER) | Some(CAMP_STASH)
        ) {
            return Err(GameRuntimeError::InvalidSave(format!(
                "loadout item {} containment is inconsistent",
                item.raw()
            )));
        }
        let expected = product_loadout_armor(item)?;
        let actual = session
            .entities()
            .component::<ItemComponent>(item)?
            .expect("validated product item component exists");
        if actual.definition().as_str() != format!("armor.{expected}") {
            return Err(GameRuntimeError::InvalidSave(format!(
                "loadout item {} definition is inconsistent",
                item.raw()
            )));
        }
    }
    let opponent_equipment = session
        .entities()
        .component::<EquipmentComponent>(OPPONENT)?
        .ok_or_else(|| GameRuntimeError::InvalidSave("opponent equipment is missing".to_owned()))?;
    let victory = campaign.outcome == Some(EncounterOutcome::Victory);
    let opponent_armor_owner = session.entities().contained_in(OPPONENT_ARMOR);
    let opponent_loadout_is_intact = opponent_armor_owner == Some(OPPONENT)
        && opponent_equipment.assignments().len() == 1
        && opponent_equipment.assignments()[0].item == OPPONENT_ARMOR
        && opponent_equipment.assignments()[0].slot.as_str() == "body";
    let reward_is_claimed = matches!(opponent_armor_owner, Some(PLAYER) | Some(CAMP_STASH))
        && opponent_equipment.assignments().is_empty();
    if (victory && !reward_is_claimed) || (!victory && !opponent_loadout_is_intact) {
        return Err(GameRuntimeError::InvalidSave(
            "the Warden's equipment/reward state is inconsistent with the campaign outcome"
                .to_owned(),
        ));
    }
    validate_campaign_vitality(session, campaign)?;
    Ok(())
}

fn validate_campaign_vitality(
    session: &D20Session,
    campaign: &CampaignState,
) -> Result<(), GameRuntimeError> {
    let player_vitality = saved_vitality(session, PLAYER)?;
    let opponent_vitality = saved_vitality(session, OPPONENT)?;
    let valid = match (campaign.phase, campaign.outcome) {
        (CampaignPhase::Encounter, None) | (CampaignPhase::Camp, None) => {
            player_vitality > 0 && opponent_vitality > 0
        }
        (CampaignPhase::Outcome | CampaignPhase::Camp, Some(EncounterOutcome::Victory)) => {
            player_vitality > 0 && opponent_vitality == 0
        }
        (CampaignPhase::Outcome, Some(EncounterOutcome::Defeat)) => {
            player_vitality == 0 && opponent_vitality > 0
        }
        (CampaignPhase::Camp, Some(EncounterOutcome::Defeat)) => {
            player_vitality > 0 && opponent_vitality > 0
        }
        (CampaignPhase::Encounter, Some(_)) | (CampaignPhase::Outcome, None) => false,
    };
    if !valid {
        return Err(GameRuntimeError::InvalidSave(format!(
            "campaign phase/outcome contradict authoritative vitality: player={player_vitality}, opponent={opponent_vitality}"
        )));
    }
    Ok(())
}

fn saved_vitality(session: &D20Session, entity: EntityId) -> Result<i64, GameRuntimeError> {
    session
        .entities()
        .component::<TracksComponent>(entity)?
        .and_then(|tracks| {
            tracks
                .values()
                .iter()
                .find(|value| value.track().as_str() == "vitality")
                .map(|value| value.current().get())
        })
        .ok_or_else(|| {
            GameRuntimeError::InvalidSave(format!(
                "entity {} authoritative vitality is missing",
                entity.raw()
            ))
        })
}

fn migrate_legacy_campaign(
    session: &mut D20Session,
    mut campaign: CampaignState,
    next_operation: &mut u64,
) -> Result<CampaignState, GameRuntimeError> {
    let player_vitality = saved_vitality(session, PLAYER)?;
    let opponent_vitality = saved_vitality(session, OPPONENT)?;
    match campaign.phase {
        CampaignPhase::Encounter if player_vitality > 0 && opponent_vitality > 0 => {}
        CampaignPhase::Encounter if player_vitality > 0 && opponent_vitality == 0 => {
            transfer_victory_reward(session, next_operation)?;
            campaign.phase = CampaignPhase::Outcome;
            campaign.turn_owner = None;
            campaign.outcome = Some(EncounterOutcome::Victory);
        }
        CampaignPhase::Encounter if player_vitality == 0 && opponent_vitality > 0 => {
            campaign.phase = CampaignPhase::Outcome;
            campaign.turn_owner = None;
            campaign.outcome = Some(EncounterOutcome::Defeat);
        }
        CampaignPhase::Camp if player_vitality > 0 && opponent_vitality > 0 => {}
        CampaignPhase::Encounter | CampaignPhase::Camp | CampaignPhase::Outcome => {
            return Err(GameRuntimeError::InvalidSave(format!(
                "legacy campaign has an impossible phase/vitality combination: player={player_vitality}, opponent={opponent_vitality}"
            )));
        }
    }
    Ok(campaign)
}

fn transfer_victory_reward(
    session: &mut D20Session,
    next_operation: &mut u64,
) -> Result<(), GameRuntimeError> {
    if *next_operation == 0 {
        return Err(GameRuntimeError::InvalidSave(
            "next operation identity must be nonzero".to_owned(),
        ));
    }
    let unequip_serial = *next_operation;
    session.unequip_armor(
        OPPONENT,
        OPPONENT_ARMOR,
        operation(&format!("reward-unequip-{unequip_serial}"))?,
    )?;
    *next_operation = next_operation
        .checked_add(1)
        .ok_or(GameRuntimeError::CounterOverflow)?;
    let transfer_serial = *next_operation;
    session.transfer_armor(
        OPPONENT_ARMOR,
        OPPONENT,
        CAMP_STASH,
        operation(&format!("reward-transfer-{transfer_serial}"))?,
    )?;
    *next_operation = next_operation
        .checked_add(1)
        .ok_or(GameRuntimeError::CounterOverflow)?;
    Ok(())
}

fn validate_inventory(
    session: &D20Session,
    owner: EntityId,
    expected_maximum: u64,
) -> Result<(), GameRuntimeError> {
    let view = session.inventory_view(owner)?;
    let capacity = view
        .capacity()
        .iter()
        .find(|usage| usage.metric.as_str() == "carried-items")
        .ok_or_else(|| {
            GameRuntimeError::InvalidSave(format!(
                "inventory {} carried-items capacity is missing",
                owner.raw()
            ))
        })?;
    if capacity.maximum != Some(expected_maximum) {
        return Err(GameRuntimeError::InvalidSave(format!(
            "inventory {} carried-items maximum is inconsistent",
            owner.raw()
        )));
    }
    Ok(())
}

fn equip_initial_player_loadout(session: &mut D20Session) -> Result<(), GameRuntimeError> {
    session.equip_armor(
        PLAYER,
        PLAYER_CHAIN_ARMOR,
        &id("chain-armor")?,
        operation("equip-mara-chain")?,
    )?;
    session.equip_armor(
        PLAYER,
        PLAYER_BUCKLER,
        &id("buckler")?,
        operation("equip-mara-buckler")?,
    )?;
    Ok(())
}

fn product_loadout_armor(item: EntityId) -> Result<D20Id, GameRuntimeError> {
    match item {
        OPPONENT_ARMOR | PLAYER_CHAIN_ARMOR => id("chain-armor"),
        PLAYER_BUCKLER | STASH_BUCKLER => id("buckler"),
        _ => Err(GameRuntimeError::InvalidCommand(format!(
            "entity {} is not a player loadout item",
            item.raw()
        ))),
    }
}

fn entity(raw: u64) -> Result<EntityId, GameRuntimeError> {
    if raw == 0 {
        return Err(GameRuntimeError::InvalidCommand(
            "entity identity must be nonzero".to_owned(),
        ));
    }
    Ok(EntityId::new(raw))
}

fn id(value: &str) -> Result<D20Id, GameRuntimeError> {
    Ok(D20Id::parse(value)?)
}

fn operation(value: &str) -> Result<OperationId, GameRuntimeError> {
    OperationId::parse(value).map_err(|error| GameRuntimeError::InvalidCommand(error.to_string()))
}

fn effect_instance(value: &str) -> Result<EffectInstanceId, GameRuntimeError> {
    EffectInstanceId::parse(value)
        .map_err(|error| GameRuntimeError::InvalidCommand(error.to_string()))
}

fn humanize(value: &str) -> String {
    value
        .split(['-', '.', '/'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            match characters.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), characters.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn stat_contribution_label(contribution: Option<&StatContribution>) -> String {
    match contribution {
        Some(StatContribution::Add { amount }) => format!("{:+} defense", amount.get()),
        Some(StatContribution::Scale { ratio }) => {
            format!("scale × {}/{}", ratio.numerator(), ratio.denominator())
        }
        Some(StatContribution::Minimum { value }) => format!("minimum {}", value.get()),
        Some(StatContribution::Maximum { value }) => format!("maximum {}", value.get()),
        None => "no matching defense contribution".to_owned(),
    }
}

fn damage_decision_label(decision: &ResponseDecisionKind) -> String {
    match decision {
        ResponseDecisionKind::NoDamageResponse => "no matching damage response".to_owned(),
        ResponseDecisionKind::Prevent => "prevent damage".to_owned(),
        ResponseDecisionKind::FlatReduction { amount } => {
            format!("reduce damage by {}", amount.get())
        }
        ResponseDecisionKind::Scale { ratio } => {
            format!(
                "scale damage × {}/{}",
                ratio.numerator(),
                ratio.denominator()
            )
        }
        ResponseDecisionKind::Absorb { track } => {
            format!("absorb into {}", humanize(track.as_str()))
        }
    }
}

const fn outcome_label(outcome: DecisionOutcome) -> &'static str {
    match outcome {
        DecisionOutcome::Applied => "applied",
        DecisionOutcome::Suppressed => "suppressed",
        DecisionOutcome::Inapplicable => "inapplicable",
    }
}

fn source_label(source: &SourceInstanceIdentity) -> String {
    match source {
        SourceInstanceIdentity::Intrinsic { entity, instance } => {
            format!(
                "Intrinsic {} on entity {}",
                humanize(instance.as_str()),
                entity.raw()
            )
        }
        SourceInstanceIdentity::Effect { effect, source, .. } => {
            format!(
                "Effect {} via {}",
                humanize(effect.as_str()),
                humanize(source.as_str())
            )
        }
        SourceInstanceIdentity::EquippedItem { item, source, .. } => format!(
            "Equipped item {} via {}",
            item.raw(),
            humanize(source.as_str())
        ),
        SourceInstanceIdentity::Request {
            operation,
            instance,
        } => format!(
            "Request {} via {}",
            humanize(operation.as_str()),
            humanize(instance.as_str())
        ),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn start_test_encounter(runtime: &mut GameRuntime) -> GameSnapshotDto {
        let camp = runtime.new_adventure(runtime.revision).unwrap();
        assert_eq!(
            camp.campaign.as_ref().unwrap().phase,
            CampaignPhaseDto::Camp
        );
        runtime
            .enter_encounter(EnterEncounterRequestDto {
                expected_revision: camp.revision,
                encounter_id: ENCOUNTER_ID.to_owned(),
            })
            .unwrap()
    }

    #[test]
    fn camp_loadout_is_engine_backed_typed_atomic_and_persistent() {
        let mut runtime = GameRuntime::empty().unwrap();
        let camp = runtime.new_adventure(0).unwrap();
        let loadout = &camp.campaign.as_ref().unwrap().loadout;
        assert_eq!(loadout.capacity.used, 2);
        assert_eq!(loadout.capacity.maximum, 2);
        assert_eq!(loadout.armor_defense, 16);
        assert_eq!(
            loadout
                .equipment_slots
                .iter()
                .find(|slot| slot.id == "body")
                .unwrap()
                .equipped
                .as_ref()
                .unwrap()
                .entity_id,
            PLAYER_CHAIN_ARMOR.raw()
        );

        let before_invalid = runtime.snapshot().unwrap();
        let invalid_slot = runtime
            .equip_item(EquipItemRequestDto {
                expected_revision: camp.revision,
                item_id: PLAYER_CHAIN_ARMOR.raw(),
                slot_id: "off-hand".to_owned(),
            })
            .unwrap_err();
        assert_eq!(invalid_slot.api_error().kind, ApiErrorKindDto::InvalidSlot);
        assert_eq!(runtime.snapshot().unwrap(), before_invalid);

        let capacity = runtime
            .transfer_item(TransferItemRequestDto {
                expected_revision: camp.revision,
                item_id: STASH_BUCKLER.raw(),
                from_owner_id: CAMP_STASH.raw(),
                to_owner_id: PLAYER.raw(),
            })
            .unwrap_err();
        assert_eq!(capacity.api_error().kind, ApiErrorKindDto::Capacity);
        assert_eq!(runtime.snapshot().unwrap(), before_invalid);

        let containment = runtime
            .transfer_item(TransferItemRequestDto {
                expected_revision: camp.revision,
                item_id: PLAYER_CHAIN_ARMOR.raw(),
                from_owner_id: PLAYER.raw(),
                to_owner_id: CAMP_STASH.raw(),
            })
            .unwrap_err();
        assert_eq!(containment.api_error().kind, ApiErrorKindDto::Containment);
        assert_eq!(runtime.snapshot().unwrap(), before_invalid);

        let chain_removed = runtime
            .unequip_item(UnequipItemRequestDto {
                expected_revision: camp.revision,
                item_id: PLAYER_CHAIN_ARMOR.raw(),
            })
            .unwrap();
        assert_eq!(
            chain_removed
                .campaign
                .as_ref()
                .unwrap()
                .loadout
                .armor_defense,
            14
        );
        let chain_restored = runtime
            .equip_item(EquipItemRequestDto {
                expected_revision: chain_removed.revision,
                item_id: PLAYER_CHAIN_ARMOR.raw(),
                slot_id: "body".to_owned(),
            })
            .unwrap();
        assert_eq!(
            chain_restored
                .campaign
                .as_ref()
                .unwrap()
                .loadout
                .armor_defense,
            16
        );

        let buckler_removed = runtime
            .unequip_item(UnequipItemRequestDto {
                expected_revision: chain_restored.revision,
                item_id: PLAYER_BUCKLER.raw(),
            })
            .unwrap();
        let stored = runtime
            .transfer_item(TransferItemRequestDto {
                expected_revision: buckler_removed.revision,
                item_id: PLAYER_BUCKLER.raw(),
                from_owner_id: PLAYER.raw(),
                to_owner_id: CAMP_STASH.raw(),
            })
            .unwrap();
        let taken = runtime
            .transfer_item(TransferItemRequestDto {
                expected_revision: stored.revision,
                item_id: STASH_BUCKLER.raw(),
                from_owner_id: CAMP_STASH.raw(),
                to_owner_id: PLAYER.raw(),
            })
            .unwrap();
        let equipped = runtime
            .equip_item(EquipItemRequestDto {
                expected_revision: taken.revision,
                item_id: STASH_BUCKLER.raw(),
                slot_id: "off-hand".to_owned(),
            })
            .unwrap();
        let equipped_loadout = &equipped.campaign.as_ref().unwrap().loadout;
        assert_eq!(equipped_loadout.capacity.used, 2);
        assert_eq!(
            equipped_loadout
                .equipment_slots
                .iter()
                .find(|slot| slot.id == "off-hand")
                .unwrap()
                .equipped
                .as_ref()
                .unwrap()
                .entity_id,
            STASH_BUCKLER.raw()
        );

        let stale_before = runtime.snapshot().unwrap();
        let stale = runtime
            .unequip_item(UnequipItemRequestDto {
                expected_revision: taken.revision,
                item_id: STASH_BUCKLER.raw(),
            })
            .unwrap_err();
        assert_eq!(stale.api_error().kind, ApiErrorKindDto::Stale);
        assert_eq!(runtime.snapshot().unwrap(), stale_before);

        let encoded = runtime.encode_save().unwrap();
        let mut reopened = GameRuntime::decode_save(&encoded).unwrap();
        assert_eq!(reopened.encode_save().unwrap(), encoded);
        let reopened_snapshot = reopened.snapshot().unwrap();
        assert_eq!(
            reopened_snapshot.campaign.as_ref().unwrap().loadout,
            equipped_loadout.clone()
        );
        let encounter = reopened
            .enter_encounter(EnterEncounterRequestDto {
                expected_revision: reopened_snapshot.revision,
                encounter_id: ENCOUNTER_ID.to_owned(),
            })
            .unwrap();
        assert_eq!(
            encounter
                .campaign
                .as_ref()
                .unwrap()
                .loadout
                .equipment_slots
                .iter()
                .find(|slot| slot.id == "off-hand")
                .unwrap()
                .equipped
                .as_ref()
                .unwrap()
                .entity_id,
            STASH_BUCKLER.raw()
        );
        let phase_before = reopened.snapshot().unwrap();
        let phase_error = reopened
            .unequip_item(UnequipItemRequestDto {
                expected_revision: encounter.revision,
                item_id: STASH_BUCKLER.raw(),
            })
            .unwrap_err();
        assert_eq!(phase_error.api_error().kind, ApiErrorKindDto::Phase);
        assert_eq!(reopened.snapshot().unwrap(), phase_before);
    }

    #[test]
    fn equipment_track_bound_failure_keeps_its_public_error_identity() {
        let error = GameRuntimeError::Session(D20SessionError::Mechanics(
            MechanicsError::EquipmentWouldInvalidateTrack {
                owner: PLAYER,
                track: gameplay_mechanics::TrackId::parse("vitality").unwrap(),
                current: 100,
                prospective_minimum: 0,
                prospective_maximum: 90,
            },
        ));
        assert_eq!(error.api_error().kind, ApiErrorKindDto::TrackBound);
    }

    #[test]
    fn product_runtime_is_atomic_stale_safe_and_reopens_deterministically() {
        let mut runtime = GameRuntime::empty().unwrap();
        assert!(runtime.snapshot().unwrap().encounter.is_none());
        let started = start_test_encounter(&mut runtime);
        let encounter = started.encounter.unwrap();
        assert_eq!(encounter.characters.len(), 2);
        assert_eq!(encounter.actions.len(), 2);

        let before_stale = runtime.encode_save().unwrap();
        assert!(matches!(
            runtime.preview_action(PreviewActionRequestDto {
                expected_revision: 0,
                actor_id: PLAYER.raw(),
                target_id: OPPONENT.raw(),
                action_id: "longsword-strike".to_owned(),
            }),
            Err(GameRuntimeError::StaleCommand(_))
        ));
        assert_eq!(runtime.encode_save().unwrap(), before_stale);

        let previewed = runtime
            .preview_action(PreviewActionRequestDto {
                expected_revision: started.revision,
                actor_id: PLAYER.raw(),
                target_id: OPPONENT.raw(),
                action_id: "longsword-strike".to_owned(),
            })
            .unwrap();
        let pending = previewed
            .encounter
            .as_ref()
            .unwrap()
            .pending_action
            .as_ref()
            .unwrap();
        assert_eq!(pending.reactions[0].id, "parry");
        assert!(pending
            .defense_sources
            .iter()
            .any(|source| source.contains("Equipped item")));
        let reacted = runtime
            .apply_reaction(ApplyReactionRequestDto {
                expected_revision: previewed.revision,
                preview_token: pending.token.clone(),
                reaction_id: "parry".to_owned(),
            })
            .unwrap();
        let pending = reacted
            .encounter
            .as_ref()
            .unwrap()
            .pending_action
            .as_ref()
            .unwrap();
        assert_eq!(pending.defense, 17);
        let applied = runtime
            .apply_action(ApplyActionRequestDto {
                expected_revision: reacted.revision,
                preview_token: pending.token.clone(),
            })
            .unwrap();
        assert!(applied
            .encounter
            .as_ref()
            .unwrap()
            .log
            .iter()
            .any(|entry| entry.details.iter().any(|detail| detail.contains("d20"))));

        let encoded = runtime.encode_save().unwrap();
        let mut reopened = GameRuntime::decode_save(&encoded).unwrap();
        let mut same_reopened = GameRuntime::decode_save(&encoded).unwrap();
        assert_eq!(reopened.encode_save().unwrap(), encoded);
        let reopened_snapshot = reopened.snapshot().unwrap();
        assert!(reopened_snapshot
            .encounter
            .as_ref()
            .unwrap()
            .pending_action
            .is_none());
        assert_eq!(
            reopened_snapshot.encounter.as_ref().unwrap().turn_owner,
            Some(EncounterTurnOwnerDto::Opposition)
        );
        let opposition = reopened
            .begin_opposition_turn(reopened_snapshot.revision)
            .unwrap();
        let same_opposition = same_reopened
            .begin_opposition_turn(reopened_snapshot.revision)
            .unwrap();
        assert_eq!(
            opposition.encounter.as_ref().unwrap().pending_action,
            same_opposition.encounter.as_ref().unwrap().pending_action,
            "the exact save and Rust-owned RNG position select the same opposition action"
        );
        let pending = opposition
            .encounter
            .as_ref()
            .unwrap()
            .pending_action
            .as_ref()
            .unwrap();
        assert_eq!(pending.actor_id, OPPONENT.raw());
        assert_eq!(pending.target_id, PLAYER.raw());
        assert!(matches!(
            pending.action_id.as_str(),
            "longsword-strike" | "precise-shot"
        ));
        let token = pending.token.clone();
        let advanced = reopened
            .apply_action(ApplyActionRequestDto {
                expected_revision: opposition.revision,
                preview_token: token,
            })
            .unwrap();
        let advanced_encounter = advanced.encounter.as_ref().unwrap();
        assert_eq!(advanced_encounter.turn, 1);
        assert_eq!(
            advanced_encounter.turn_owner,
            Some(EncounterTurnOwnerDto::Player)
        );
        assert!(advanced_encounter
            .log
            .last()
            .is_some_and(|entry| entry.source == "Round"
                && entry.text.contains("round 0 to 1")
                && entry
                    .details
                    .iter()
                    .any(|detail| detail.contains("1 scheduled effect(s) expired"))));
    }

    #[test]
    fn complete_encounter_victory_grants_reward_once_and_reopens_exactly() {
        let mut runtime = GameRuntime::empty().unwrap();
        start_test_encounter(&mut runtime);
        let outcome = play_to_outcome(&mut runtime, "precise-shot", false, true);
        let campaign = outcome.campaign.as_ref().unwrap();
        assert_eq!(campaign.phase, CampaignPhaseDto::Outcome);
        assert_eq!(
            campaign.latest_outcome.as_ref().unwrap().kind,
            EncounterOutcomeKindDto::Victory
        );
        assert_eq!(
            campaign.latest_outcome.as_ref().unwrap().reward_item_id,
            Some(OPPONENT_ARMOR.raw())
        );
        assert!(campaign
            .loadout
            .stash_items
            .iter()
            .any(|item| item.entity_id == OPPONENT_ARMOR.raw()));
        assert_eq!(outcome.encounter.as_ref().unwrap().turn_owner, None);
        assert!(outcome
            .encounter
            .as_ref()
            .unwrap()
            .log
            .iter()
            .any(|entry| entry.text.contains("yields the Warden chain armor")));

        let encoded_outcome = runtime.encode_save().unwrap();
        let mut reopened = GameRuntime::decode_save(&encoded_outcome).unwrap();
        assert_eq!(reopened.encode_save().unwrap(), encoded_outcome);
        let before_late = reopened.snapshot().unwrap();
        let late = reopened
            .begin_opposition_turn(before_late.revision)
            .unwrap_err();
        assert_eq!(late.api_error().kind, ApiErrorKindDto::Phase);
        assert_eq!(reopened.snapshot().unwrap(), before_late);

        let camp = reopened.return_to_camp(before_late.revision).unwrap();
        let campaign = camp.campaign.as_ref().unwrap();
        assert_eq!(campaign.phase, CampaignPhaseDto::Camp);
        assert!(campaign.available_encounters.is_empty());
        assert_eq!(
            campaign
                .loadout
                .stash_items
                .iter()
                .filter(|item| item.entity_id == OPPONENT_ARMOR.raw())
                .count(),
            1
        );
        let before_duplicate = reopened.snapshot().unwrap();
        assert!(matches!(
            reopened.return_to_camp(before_duplicate.revision),
            Err(GameRuntimeError::WrongPhase(_))
        ));
        assert_eq!(reopened.snapshot().unwrap(), before_duplicate);
        let camp_save = reopened.encode_save().unwrap();
        assert_eq!(
            GameRuntime::decode_save(&camp_save)
                .unwrap()
                .encode_save()
                .unwrap(),
            camp_save
        );
    }

    #[test]
    fn complete_encounter_defeat_has_no_reward_and_applies_bounded_recovery() {
        let mut runtime = GameRuntime::empty().unwrap();
        let camp = runtime.new_adventure(0).unwrap();
        let without_chain = runtime
            .unequip_item(UnequipItemRequestDto {
                expected_revision: camp.revision,
                item_id: PLAYER_CHAIN_ARMOR.raw(),
            })
            .unwrap();
        let without_armor = runtime
            .unequip_item(UnequipItemRequestDto {
                expected_revision: without_chain.revision,
                item_id: PLAYER_BUCKLER.raw(),
            })
            .unwrap();
        runtime
            .enter_encounter(EnterEncounterRequestDto {
                expected_revision: without_armor.revision,
                encounter_id: ENCOUNTER_ID.to_owned(),
            })
            .unwrap();
        let outcome = play_to_outcome(&mut runtime, "longsword-strike", true, false);
        let campaign = outcome.campaign.as_ref().unwrap();
        assert_eq!(campaign.phase, CampaignPhaseDto::Outcome);
        assert_eq!(
            campaign.latest_outcome.as_ref().unwrap().kind,
            EncounterOutcomeKindDto::Defeat
        );
        assert_eq!(
            campaign.latest_outcome.as_ref().unwrap().reward_item_id,
            None
        );
        assert!(!campaign
            .loadout
            .stash_items
            .iter()
            .any(|item| item.entity_id == OPPONENT_ARMOR.raw()));
        assert_eq!(
            campaign.hero.health_current, 0,
            "defeat is derived from authoritative vitality"
        );

        let outcome_save = runtime.encode_save().unwrap();
        let mut reopened = GameRuntime::decode_save(&outcome_save).unwrap();
        let camp = reopened
            .return_to_camp(reopened.snapshot().unwrap().revision)
            .unwrap();
        assert_eq!(
            camp.campaign.as_ref().unwrap().hero.health_current,
            i64::from(DEFEAT_RECOVERY_VITALITY)
        );
        assert!(camp
            .campaign
            .as_ref()
            .unwrap()
            .latest_outcome
            .as_ref()
            .is_some_and(|outcome| outcome.kind == EncounterOutcomeKindDto::Defeat));
        let recovered_save = reopened.encode_save().unwrap();
        assert_eq!(
            GameRuntime::decode_save(&recovered_save)
                .unwrap()
                .encode_save()
                .unwrap(),
            recovered_save
        );
    }

    #[test]
    fn schema_four_rejects_outcome_that_disagrees_with_authoritative_vitality() {
        let mut active_runtime = GameRuntime::empty().unwrap();
        start_test_encounter(&mut active_runtime);
        let active_save: serde_json::Value =
            serde_json::from_str(&active_runtime.encode_save().unwrap()).unwrap();

        let mut forged_defeat = active_save.clone();
        forged_defeat["campaign"]["phase"] = json!("outcome");
        forged_defeat["campaign"]["turnOwner"] = serde_json::Value::Null;
        forged_defeat["campaign"]["outcome"] = json!("defeat");
        assert_vitality_mismatch_rejected(&forged_defeat);

        let mut dead_active_encounter = active_save;
        set_saved_vitality(&mut dead_active_encounter, PLAYER, 0);
        assert_vitality_mismatch_rejected(&dead_active_encounter);

        let mut victory_runtime = GameRuntime::empty().unwrap();
        start_test_encounter(&mut victory_runtime);
        let victory = play_to_outcome(&mut victory_runtime, "precise-shot", false, true);
        let victory_save = victory_runtime.encode_save().unwrap();
        GameRuntime::decode_save(&victory_save).unwrap();

        let mut forged_victory: serde_json::Value = serde_json::from_str(&victory_save).unwrap();
        set_saved_vitality(&mut forged_victory, OPPONENT, 1);
        assert_vitality_mismatch_rejected(&forged_victory);

        victory_runtime.return_to_camp(victory.revision).unwrap();
        GameRuntime::decode_save(&victory_runtime.encode_save().unwrap()).unwrap();

        let mut defeat_runtime = GameRuntime::empty().unwrap();
        let camp = defeat_runtime.new_adventure(0).unwrap();
        let without_chain = defeat_runtime
            .unequip_item(UnequipItemRequestDto {
                expected_revision: camp.revision,
                item_id: PLAYER_CHAIN_ARMOR.raw(),
            })
            .unwrap();
        let without_armor = defeat_runtime
            .unequip_item(UnequipItemRequestDto {
                expected_revision: without_chain.revision,
                item_id: PLAYER_BUCKLER.raw(),
            })
            .unwrap();
        defeat_runtime
            .enter_encounter(EnterEncounterRequestDto {
                expected_revision: without_armor.revision,
                encounter_id: ENCOUNTER_ID.to_owned(),
            })
            .unwrap();
        let defeat = play_to_outcome(&mut defeat_runtime, "longsword-strike", true, false);
        GameRuntime::decode_save(&defeat_runtime.encode_save().unwrap()).unwrap();
        defeat_runtime.return_to_camp(defeat.revision).unwrap();
        GameRuntime::decode_save(&defeat_runtime.encode_save().unwrap()).unwrap();
    }

    #[test]
    fn schema_three_terminal_encounter_remains_migratable() {
        let mut encounter_runtime = GameRuntime::empty().unwrap();
        start_test_encounter(&mut encounter_runtime);
        let encounter_save = encounter_runtime.encode_save().unwrap();

        for schema in 1..=3 {
            let live = legacy_product_save(&encounter_save, schema);
            let live_snapshot = GameRuntime::decode_save(&serde_json::to_string(&live).unwrap())
                .unwrap()
                .snapshot()
                .unwrap();
            assert_eq!(
                live_snapshot.campaign.as_ref().unwrap().phase,
                CampaignPhaseDto::Encounter
            );
            assert_eq!(
                live_snapshot.encounter.as_ref().unwrap().turn_owner,
                Some(EncounterTurnOwnerDto::Player)
            );

            let mut legacy_victory = live.clone();
            set_saved_vitality(&mut legacy_victory, OPPONENT, 0);
            let mut migrated_victory =
                GameRuntime::decode_save(&serde_json::to_string(&legacy_victory).unwrap()).unwrap();
            let victory = migrated_victory.snapshot().unwrap();
            let campaign = victory.campaign.as_ref().unwrap();
            assert_eq!(campaign.phase, CampaignPhaseDto::Outcome);
            assert_eq!(
                campaign.latest_outcome.as_ref().unwrap().kind,
                EncounterOutcomeKindDto::Victory
            );
            assert_eq!(
                campaign
                    .loadout
                    .stash_items
                    .iter()
                    .filter(|item| item.entity_id == OPPONENT_ARMOR.raw())
                    .count(),
                1
            );
            let schema_four_victory = migrated_victory.encode_save().unwrap();
            let schema_four_value: serde_json::Value =
                serde_json::from_str(&schema_four_victory).unwrap();
            assert_eq!(
                schema_four_value["schemaVersion"],
                json!(GAME_SAVE_SCHEMA_VERSION)
            );
            assert_eq!(schema_four_value["nextOperation"], json!(3));
            assert_eq!(
                GameRuntime::decode_save(&schema_four_victory)
                    .unwrap()
                    .encode_save()
                    .unwrap(),
                schema_four_victory
            );
            let camp = migrated_victory.return_to_camp(victory.revision).unwrap();
            assert_eq!(
                camp.campaign
                    .as_ref()
                    .unwrap()
                    .loadout
                    .stash_items
                    .iter()
                    .filter(|item| item.entity_id == OPPONENT_ARMOR.raw())
                    .count(),
                1
            );

            let mut legacy_defeat = live.clone();
            set_saved_vitality(&mut legacy_defeat, PLAYER, 0);
            let mut migrated_defeat =
                GameRuntime::decode_save(&serde_json::to_string(&legacy_defeat).unwrap()).unwrap();
            let defeat = migrated_defeat.snapshot().unwrap();
            let campaign = defeat.campaign.as_ref().unwrap();
            assert_eq!(campaign.phase, CampaignPhaseDto::Outcome);
            assert_eq!(
                campaign.latest_outcome.as_ref().unwrap().kind,
                EncounterOutcomeKindDto::Defeat
            );
            assert!(!campaign
                .loadout
                .stash_items
                .iter()
                .any(|item| item.entity_id == OPPONENT_ARMOR.raw()));
            let schema_four_defeat = migrated_defeat.encode_save().unwrap();
            assert_eq!(
                GameRuntime::decode_save(&schema_four_defeat)
                    .unwrap()
                    .encode_save()
                    .unwrap(),
                schema_four_defeat
            );
            let recovered = migrated_defeat.return_to_camp(defeat.revision).unwrap();
            assert_eq!(
                recovered.campaign.as_ref().unwrap().hero.health_current,
                i64::from(DEFEAT_RECOVERY_VITALITY)
            );

            let mut impossible = live;
            set_saved_vitality(&mut impossible, PLAYER, 0);
            set_saved_vitality(&mut impossible, OPPONENT, 0);
            assert_legacy_vitality_rejected(&impossible);
        }

        let mut camp_runtime = GameRuntime::empty().unwrap();
        camp_runtime.new_adventure(0).unwrap();
        let camp_save = camp_runtime.encode_save().unwrap();
        for schema in 2..=3 {
            let live_camp = legacy_product_save(&camp_save, schema);
            let migrated = GameRuntime::decode_save(&serde_json::to_string(&live_camp).unwrap())
                .unwrap()
                .snapshot()
                .unwrap();
            assert_eq!(
                migrated.campaign.as_ref().unwrap().phase,
                CampaignPhaseDto::Camp
            );

            let mut impossible_camp = live_camp;
            set_saved_vitality(&mut impossible_camp, OPPONENT, 0);
            assert_legacy_vitality_rejected(&impossible_camp);
        }
    }

    fn play_to_outcome(
        runtime: &mut GameRuntime,
        player_action: &str,
        opponent_reacts: bool,
        player_reacts: bool,
    ) -> GameSnapshotDto {
        for _ in 0..64 {
            let before_player = runtime.snapshot().unwrap();
            let previewed = runtime
                .preview_action(PreviewActionRequestDto {
                    expected_revision: before_player.revision,
                    actor_id: PLAYER.raw(),
                    target_id: OPPONENT.raw(),
                    action_id: player_action.to_owned(),
                })
                .unwrap();
            let mut pending = previewed
                .encounter
                .as_ref()
                .unwrap()
                .pending_action
                .clone()
                .unwrap();
            let mut current = previewed;
            if opponent_reacts && !pending.reactions.is_empty() {
                current = runtime
                    .apply_reaction(ApplyReactionRequestDto {
                        expected_revision: current.revision,
                        preview_token: pending.token.clone(),
                        reaction_id: pending.reactions[0].id.clone(),
                    })
                    .unwrap();
                pending = current
                    .encounter
                    .as_ref()
                    .unwrap()
                    .pending_action
                    .clone()
                    .unwrap();
            }
            let player_result = runtime
                .apply_action(ApplyActionRequestDto {
                    expected_revision: current.revision,
                    preview_token: pending.token,
                })
                .unwrap();
            if player_result.campaign.as_ref().unwrap().phase == CampaignPhaseDto::Outcome {
                return player_result;
            }

            let opposition = runtime
                .begin_opposition_turn(player_result.revision)
                .unwrap();
            let mut pending = opposition
                .encounter
                .as_ref()
                .unwrap()
                .pending_action
                .clone()
                .unwrap();
            let mut current = opposition;
            if player_reacts && !pending.reactions.is_empty() {
                current = runtime
                    .apply_reaction(ApplyReactionRequestDto {
                        expected_revision: current.revision,
                        preview_token: pending.token.clone(),
                        reaction_id: pending.reactions[0].id.clone(),
                    })
                    .unwrap();
                pending = current
                    .encounter
                    .as_ref()
                    .unwrap()
                    .pending_action
                    .clone()
                    .unwrap();
            }
            let opposition_result = runtime
                .apply_action(ApplyActionRequestDto {
                    expected_revision: current.revision,
                    preview_token: pending.token,
                })
                .unwrap();
            if opposition_result.campaign.as_ref().unwrap().phase == CampaignPhaseDto::Outcome {
                return opposition_result;
            }
        }
        panic!("deterministic encounter did not reach an outcome within 64 rounds");
    }

    fn set_saved_vitality(save: &mut serde_json::Value, entity: EntityId, current: i64) {
        let tracks = save["session"]["entityState"]["registeredComponents"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|registered| registered["typeId"] == "rusty.mechanics.tracks")
            .unwrap();
        let entity_tracks = tracks["values"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["entity"] == json!(entity.raw()))
            .unwrap();
        let vitality = entity_tracks["value"]["values"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|track| track["track"] == "vitality")
            .unwrap();
        vitality["current"] = json!(current);
    }

    fn assert_vitality_mismatch_rejected(save: &serde_json::Value) {
        let error = GameRuntime::decode_save(&serde_json::to_string(save).unwrap()).unwrap_err();
        assert!(
            matches!(
                &error,
                GameRuntimeError::InvalidSave(message)
                    if message.contains("contradict authoritative vitality")
            ),
            "unexpected save rejection: {error:?}"
        );
    }

    fn assert_legacy_vitality_rejected(save: &serde_json::Value) {
        let error = GameRuntime::decode_save(&serde_json::to_string(save).unwrap()).unwrap_err();
        assert!(
            matches!(
                &error,
                GameRuntimeError::InvalidSave(message)
                    if message.contains("impossible phase/vitality combination")
            ),
            "unexpected legacy save rejection: {error:?}"
        );
    }

    fn legacy_product_save(input: &str, schema: u32) -> serde_json::Value {
        let mut save: serde_json::Value = if schema <= 2 {
            serde_json::from_str(&downgrade_to_pre_loadout_v2(input)).unwrap()
        } else {
            serde_json::from_str(input).unwrap()
        };
        save["schemaVersion"] = json!(schema);
        if schema == 1 {
            save.as_object_mut().unwrap().remove("campaign");
        } else {
            save["campaign"]
                .as_object_mut()
                .unwrap()
                .remove("turnOwner");
            save["campaign"].as_object_mut().unwrap().remove("outcome");
        }
        save
    }

    #[test]
    fn campaign_phases_and_legacy_migration_are_strict_and_fail_atomic() {
        let mut runtime = GameRuntime::empty().unwrap();
        assert!(runtime.snapshot().unwrap().campaign.is_none());
        assert!(matches!(
            runtime.new_adventure(1),
            Err(GameRuntimeError::StaleCommand(_))
        ));
        assert!(runtime.snapshot().unwrap().campaign.is_none());

        let camp = runtime.new_adventure(0).unwrap();
        assert_eq!(
            camp.campaign.as_ref().unwrap().phase,
            CampaignPhaseDto::Camp
        );
        assert!(camp.encounter.is_none());
        let camp_save = runtime.encode_save().unwrap();
        assert_eq!(
            GameRuntime::decode_save(&camp_save)
                .unwrap()
                .snapshot()
                .unwrap(),
            {
                let mut saved = camp.clone();
                saved.saved = true;
                saved
            }
        );

        let before_invalid = runtime.snapshot().unwrap();
        assert!(matches!(
            runtime.enter_encounter(EnterEncounterRequestDto {
                expected_revision: camp.revision,
                encounter_id: "unknown".to_owned(),
            }),
            Err(GameRuntimeError::InvalidCommand(_))
        ));
        assert_eq!(runtime.snapshot().unwrap(), before_invalid);
        assert!(matches!(
            runtime.preview_action(PreviewActionRequestDto {
                expected_revision: camp.revision,
                actor_id: PLAYER.raw(),
                target_id: OPPONENT.raw(),
                action_id: "longsword-strike".to_owned(),
            }),
            Err(GameRuntimeError::WrongPhase(_))
        ));
        assert_eq!(runtime.snapshot().unwrap(), before_invalid);

        let encounter = runtime
            .enter_encounter(EnterEncounterRequestDto {
                expected_revision: camp.revision,
                encounter_id: ENCOUNTER_ID.to_owned(),
            })
            .unwrap();
        assert_eq!(
            encounter.campaign.as_ref().unwrap().phase,
            CampaignPhaseDto::Encounter
        );
        assert!(encounter.encounter.is_some());
        let before_duplicate = runtime.snapshot().unwrap();
        assert!(matches!(
            runtime.enter_encounter(EnterEncounterRequestDto {
                expected_revision: encounter.revision,
                encounter_id: ENCOUNTER_ID.to_owned(),
            }),
            Err(GameRuntimeError::InvalidCommand(_))
        ));
        assert_eq!(runtime.snapshot().unwrap(), before_duplicate);

        let legacy_v2 = downgrade_to_pre_loadout_v2(&runtime.encode_save().unwrap());
        let mut legacy: serde_json::Value = serde_json::from_str(&legacy_v2).unwrap();
        legacy["schemaVersion"] = json!(1);
        legacy.as_object_mut().unwrap().remove("campaign");
        let migrated = GameRuntime::decode_save(&serde_json::to_string(&legacy).unwrap()).unwrap();
        assert_eq!(
            migrated.snapshot().unwrap().campaign.unwrap().phase,
            CampaignPhaseDto::Encounter
        );
        let migrated_save: serde_json::Value =
            serde_json::from_str(&migrated.encode_save().unwrap()).unwrap();
        assert_eq!(
            migrated_save["schemaVersion"],
            json!(GAME_SAVE_SCHEMA_VERSION)
        );

        let migrated_v2 = GameRuntime::decode_save(&legacy_v2).unwrap();
        let migrated_loadout = migrated_v2.snapshot().unwrap().campaign.unwrap().loadout;
        assert_eq!(migrated_loadout.capacity.used, 2);
        assert_eq!(migrated_loadout.armor_defense, 16);
        assert_eq!(migrated_loadout.stash_items.len(), 1);

        let mut wrong_legacy_catalog: serde_json::Value = serde_json::from_str(&legacy_v2).unwrap();
        let registered = wrong_legacy_catalog["session"]["entityState"]["registeredComponents"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|registered| registered["typeId"] == "rusty.mechanics.stats")
            .unwrap();
        registered["values"][0]["value"]["catalogVersion"] = json!("rusty-d20.v2");
        assert!(matches!(
            GameRuntime::decode_save(&serde_json::to_string(&wrong_legacy_catalog).unwrap()),
            Err(GameRuntimeError::Save(SessionSaveError::InvalidState(
                D20SessionError::LegacyCatalogVersionMismatch { .. }
            )))
        ));

        let mut invalid: serde_json::Value =
            serde_json::from_str(&runtime.encode_save().unwrap()).unwrap();
        invalid["campaign"]["activeEncounterId"] = serde_json::Value::Null;
        assert!(matches!(
            GameRuntime::decode_save(&serde_json::to_string(&invalid).unwrap()),
            Err(GameRuntimeError::InvalidSave(_))
        ));

        let mut partial_loadout: serde_json::Value =
            serde_json::from_str(&runtime.encode_save().unwrap()).unwrap();
        let inventory = partial_loadout["session"]["entityState"]["registeredComponents"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|registered| registered["typeId"] == "rusty.mechanics.inventory")
            .unwrap();
        inventory["values"]
            .as_array_mut()
            .unwrap()
            .retain(|entry| entry["entity"] != json!(CAMP_STASH.raw()));
        assert!(matches!(
            GameRuntime::decode_save(&serde_json::to_string(&partial_loadout).unwrap()),
            Err(GameRuntimeError::InvalidSave(_))
        ));
    }

    fn downgrade_to_pre_loadout_v2(input: &str) -> String {
        let mut save: serde_json::Value = serde_json::from_str(input).unwrap();
        save["schemaVersion"] = json!(2);
        save["campaign"]
            .as_object_mut()
            .unwrap()
            .remove("turnOwner");
        save["campaign"].as_object_mut().unwrap().remove("outcome");
        save["session"]["schemaVersion"] = json!(1);
        let state = save["session"]["entityState"].as_object_mut().unwrap();
        state
            .get_mut("entities")
            .unwrap()
            .as_array_mut()
            .unwrap()
            .retain(|entity| !matches!(entity["id"].as_u64().unwrap(), 103 | 202 | 203 | 204));
        for registered in state
            .get_mut("registeredComponents")
            .unwrap()
            .as_array_mut()
            .unwrap()
        {
            let type_id = registered["typeId"].as_str().unwrap().to_owned();
            registered
                .get_mut("values")
                .unwrap()
                .as_array_mut()
                .unwrap()
                .retain(|entry| {
                    !matches!(entry["entity"].as_u64().unwrap(), 103 | 202 | 203 | 204)
                });
            if type_id.starts_with("rusty.mechanics.") {
                for entry in registered["values"].as_array_mut().unwrap() {
                    if let Some(value) = entry["value"].as_object_mut() {
                        if value.contains_key("catalogVersion") {
                            value.insert("catalogVersion".to_owned(), json!("rusty-d20.v1"));
                        }
                    }
                }
            }
            if type_id == "rusty.mechanics.equipment" {
                for entry in registered["values"].as_array_mut().unwrap() {
                    if entry["entity"] == json!(PLAYER.raw()) {
                        entry["value"]["assignments"] = json!([]);
                    }
                }
            }
        }
        state
            .get_mut("registeredComponents")
            .unwrap()
            .as_array_mut()
            .unwrap()
            .retain(|registered| registered["typeId"] != "rusty.mechanics.inventory");
        serde_json::to_string(&save).unwrap()
    }

    #[test]
    fn preview_only_and_reacted_pending_saves_reject_without_mutation() {
        let mut runtime = GameRuntime::empty().unwrap();
        let started = start_test_encounter(&mut runtime);
        let previewed = runtime
            .preview_action(PreviewActionRequestDto {
                expected_revision: started.revision,
                actor_id: PLAYER.raw(),
                target_id: OPPONENT.raw(),
                action_id: "longsword-strike".to_owned(),
            })
            .unwrap();

        assert_pending_save_is_unchanged(&runtime, &previewed);

        let pending_token = previewed
            .encounter
            .as_ref()
            .unwrap()
            .pending_action
            .as_ref()
            .unwrap()
            .token
            .clone();
        let reacted = runtime
            .apply_reaction(ApplyReactionRequestDto {
                expected_revision: previewed.revision,
                preview_token: pending_token,
                reaction_id: "parry".to_owned(),
            })
            .unwrap();
        let opponent = reacted
            .encounter
            .as_ref()
            .unwrap()
            .characters
            .iter()
            .find(|character| character.id == OPPONENT.raw())
            .unwrap();
        assert!(opponent
            .resources
            .iter()
            .any(|resource| resource.id == "guard" && resource.current == 1));
        assert!(opponent
            .effects
            .iter()
            .any(|effect| effect.starts_with("Parry Stance")));

        assert_pending_save_is_unchanged(&runtime, &reacted);
    }

    fn assert_pending_save_is_unchanged(runtime: &GameRuntime, before: &GameSnapshotDto) {
        let session_before = runtime.session.as_ref().unwrap().encode_save().unwrap();
        assert!(matches!(
            runtime.encode_save_at(before.revision),
            Err(GameRuntimeError::PendingActionCannotBeSaved)
        ));
        assert_eq!(runtime.snapshot().unwrap(), *before);
        assert_eq!(
            runtime.session.as_ref().unwrap().encode_save().unwrap(),
            session_before
        );
    }

    #[test]
    fn saturated_product_counters_and_oversized_saves_fail_before_mutation() {
        let mut runtime = GameRuntime::empty().unwrap();
        let started = start_test_encounter(&mut runtime);
        let mut save: serde_json::Value =
            serde_json::from_str(&runtime.encode_save().unwrap()).unwrap();
        save["revision"] = json!(u64::MAX);
        let mut saturated =
            GameRuntime::decode_save(&serde_json::to_string(&save).unwrap()).unwrap();
        let before = saturated.encode_save().unwrap();
        assert!(matches!(
            saturated.preview_action(PreviewActionRequestDto {
                expected_revision: u64::MAX,
                actor_id: PLAYER.raw(),
                target_id: OPPONENT.raw(),
                action_id: "longsword-strike".to_owned(),
            }),
            Err(GameRuntimeError::CounterOverflow)
        ));
        assert_eq!(saturated.encode_save().unwrap(), before);
        assert!(matches!(
            GameRuntime::decode_save(&"x".repeat(MAX_GAME_SAVE_BYTES + 1)),
            Err(GameRuntimeError::InvalidSave(_))
        ));
        assert!(started.encounter.is_some());
    }
}
