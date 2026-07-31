use std::collections::{BTreeMap, BTreeSet};

use core_ids::EntityId;
use entity_state::ComponentAccessError;
use gameplay_mechanics::{
    ActiveEffectsComponent, DecisionOutcome, EffectInstanceId, EquipmentComponent,
    InventoryComponent, ItemComponent, MechanicsError, OperationId, ResponseDecisionKind,
    SourceInstanceIdentity, StatContribution, StatService, TracksComponent,
};
use serde::{Deserialize, Serialize};

use crate::adventure::AuthoredAdventureCatalog;
use crate::compiler::defense_stat_id;
use crate::{
    AbilityScore, ActionAttackDefinition, ActionDefinition, ActionPreview, ActionResource,
    ActionResourcesComponent, ActionTargetTeamDefinition, AdventureDefinition, AffinitySeed,
    ApplyActionRequest, CharacterAffinityKindDefinition, CharacterSeed,
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

mod content;
mod dto;
mod exploration;
mod persistence;
mod projection;
mod tactical;

use content::*;
pub use dto::*;
use tactical::*;

#[derive(Debug, Clone)]
struct PendingAction {
    serial: u64,
    token: String,
    preview: ActionPreview,
}

type LegalActionPreview = (D20Id, EntityId, ActionPreview);

fn target_team_allows(
    team: ActionTargetTeamDefinition,
    actor: EntityId,
    actor_faction: EncounterFaction,
    target: EntityId,
    target_faction: EncounterFaction,
) -> bool {
    match team {
        ActionTargetTeamDefinition::Hostile => actor_faction != target_faction,
        ActionTargetTeamDefinition::Ally => actor != target && actor_faction == target_faction,
        ActionTargetTeamDefinition::SelfOnly => actor == target,
        ActionTargetTeamDefinition::Any => true,
    }
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
    Exploration,
    Encounter,
    Outcome,
    AdventureComplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum EncounterOutcome {
    Victory,
    Defeat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CompletedEncounter {
    encounter_id: String,
    outcome: EncounterOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct DungeonPosition {
    x: u16,
    y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ExplorationState {
    position: DungeonPosition,
    facing: DungeonFacingDefinition,
    discovered: BTreeSet<DungeonPosition>,
    inspected_landmarks: BTreeSet<String>,
    checkpoint_id: String,
    opened_doors: BTreeSet<String>,
    collected_treasures: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CampaignState {
    phase: CampaignPhase,
    active_encounter_id: Option<String>,
    resolved_encounter_id: Option<String>,
    current_actor_id: Option<u64>,
    outcome: Option<EncounterOutcome>,
    completed_encounters: Vec<CompletedEncounter>,
    exploration: Option<ExplorationState>,
}

#[derive(Debug, Clone)]
pub struct GameRuntime {
    catalog: AuthoredAdventureCatalog,
    rules: D20Ruleset,
    adventure_id: D20Id,
    roll_source: RollSourceConfig,
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
        Self::empty_with_roll_source(RollSourceConfig::default())
    }

    pub fn empty_with_roll_source(roll_source: RollSourceConfig) -> Result<Self, GameRuntimeError> {
        let catalog = AuthoredAdventureCatalog::builtin().map_err(GameRuntimeError::Catalog)?;
        let adventure = catalog.default_adventure().clone();
        let rules = catalog
            .rules_for(&adventure)
            .map_err(GameRuntimeError::Catalog)?;
        Self::empty_with_rules(catalog, rules, adventure, roll_source)
    }

    pub fn empty_for(adventure: &str) -> Result<Self, GameRuntimeError> {
        let adventure = id(adventure)?;
        let catalog = AuthoredAdventureCatalog::builtin().map_err(GameRuntimeError::Catalog)?;
        let rules = catalog
            .rules_for(&adventure)
            .map_err(GameRuntimeError::Catalog)?;
        Self::empty_with_rules(catalog, rules, adventure, RollSourceConfig::default())
    }

    fn empty_with_rules(
        catalog: AuthoredAdventureCatalog,
        rules: D20Ruleset,
        adventure_id: D20Id,
        roll_source: RollSourceConfig,
    ) -> Result<Self, GameRuntimeError> {
        roll_source.validate()?;
        if rules.adventure(&adventure_id).is_none() {
            return Err(GameRuntimeError::Catalog(format!(
                "compiled rules do not define adventure {adventure_id}"
            )));
        }
        Ok(Self {
            catalog,
            rules,
            adventure_id,
            roll_source,
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

    pub fn readout_entity_count(&self) -> usize {
        self.session
            .as_ref()
            .map_or(0, |session| session.entities().total_count())
    }

    pub const fn roll_source(&self) -> &RollSourceConfig {
        &self.roll_source
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
        let exploration = match (&self.campaign, session) {
            (Some(campaign), Some(_))
                if matches!(
                    campaign.phase,
                    CampaignPhase::Exploration | CampaignPhase::Encounter | CampaignPhase::Outcome
                ) =>
            {
                campaign
                    .exploration
                    .as_ref()
                    .map(|state| self.project_exploration(state))
                    .transpose()?
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
            available_adventures: self
                .catalog
                .adventures()
                .filter(|(_, entry)| entry.selectable)
                .map(|(id, entry)| AdventureChoiceDto {
                    id: id.to_string(),
                    title: entry.title.clone(),
                    summary: entry.summary.clone(),
                    details: entry.details.clone(),
                })
                .collect(),
            campaign,
            exploration,
            encounter,
        })
    }

    pub fn new_adventure_for(
        &mut self,
        request: NewAdventureRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        if self.campaign.is_some() {
            return Err(GameRuntimeError::InvalidCommand(
                "an adventure is already active".to_owned(),
            ));
        }
        let adventure_id = id(&request.adventure_id)?;
        let entry = self
            .catalog
            .adventures()
            .find(|(id, _)| **id == adventure_id)
            .map(|(_, entry)| entry)
            .ok_or_else(|| {
                GameRuntimeError::InvalidCommand(format!(
                    "unknown authored adventure {}",
                    request.adventure_id
                ))
            })?;
        if !entry.selectable {
            return Err(GameRuntimeError::InvalidCommand(format!(
                "authored adventure {} is not selectable",
                request.adventure_id
            )));
        }
        let rules = self
            .catalog
            .rules_for(&adventure_id)
            .map_err(GameRuntimeError::InvalidCommand)?;
        let mut staged = Self::empty_with_rules(
            self.catalog.clone(),
            rules,
            adventure_id,
            self.roll_source.clone(),
        )?;
        let snapshot = staged.new_adventure(0)?;
        *self = staged;
        Ok(snapshot)
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
        let adventure = self.adventure()?.clone();
        let characters = adventure
            .characters
            .iter()
            .map(|character| {
                self.rules
                    .character_template(character)
                    .map(character_seed)
                    .ok_or_else(|| {
                        GameRuntimeError::InvalidState(format!(
                            "character template {character} is missing"
                        ))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let inventories = adventure
            .characters
            .iter()
            .map(|character| {
                let definition = self
                    .rules
                    .character_template(character)
                    .expect("compiled adventure character exists");
                InventorySeed {
                    owner: EntityId::new(definition.entity_id),
                    maximum_items: definition.inventory_capacity,
                }
            })
            .collect();
        let storage = adventure
            .storage
            .iter()
            .map(|storage| {
                let definition = self
                    .rules
                    .storage(storage)
                    .expect("compiled adventure storage exists");
                StorageSeed {
                    entity: EntityId::new(definition.entity_id),
                    name: definition.name.clone(),
                    maximum_items: definition.capacity,
                }
            })
            .collect();
        let mut session = D20Session::new_with_roll_source(
            self.rules.clone(),
            self.roll_source.clone(),
            characters,
            inventories,
            storage,
            product_equipment_items(&self.rules, &adventure)?,
        )?;
        equip_initial_loadout(&self.rules, &adventure, &mut session)?;
        self.campaign = Some(CampaignState {
            phase: CampaignPhase::Camp,
            active_encounter_id: None,
            resolved_encounter_id: None,
            current_actor_id: None,
            outcome: None,
            completed_encounters: Vec::new(),
            exploration: None,
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
            &adventure.start_source,
            &adventure.start_text,
            adventure.start_details.clone(),
        )?;
        self.snapshot()
    }

    pub fn enter_encounter(
        &mut self,
        request: EnterEncounterRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        let snapshot = staged.enter_encounter_inner(request)?;
        *self = staged;
        Ok(snapshot)
    }

    fn enter_encounter_inner(
        &mut self,
        request: EnterEncounterRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_mutation_capacity(false, true)?;
        let adventure = self.adventure()?.clone();
        let encounter_id = id(&request.encounter_id)?;
        if !adventure.encounters.contains(&encounter_id) {
            return Err(GameRuntimeError::InvalidCommand(format!(
                "unknown encounter {}",
                request.encounter_id
            )));
        }
        let encounter = self
            .rules
            .encounter(&encounter_id)
            .expect("compiled adventure encounter exists")
            .clone();
        let campaign = self
            .campaign
            .as_ref()
            .ok_or(GameRuntimeError::NoEncounter)?;
        if !matches!(
            campaign.phase,
            CampaignPhase::Camp | CampaignPhase::Exploration
        ) {
            return Err(GameRuntimeError::InvalidCommand(
                "an encounter can only begin from camp or an authored dungeon trigger".to_owned(),
            ));
        }
        if campaign.phase == CampaignPhase::Camp && !encounter.available_from_camp {
            return Err(GameRuntimeError::InvalidCommand(format!(
                "encounter {} is not available from camp",
                request.encounter_id
            )));
        }
        if campaign.phase == CampaignPhase::Exploration {
            let position = campaign
                .exploration
                .as_ref()
                .ok_or_else(|| {
                    GameRuntimeError::InvalidState(
                        "exploration phase is missing its position".to_owned(),
                    )
                })?
                .position;
            if !adventure.dungeon.encounters.iter().any(|trigger| {
                trigger.encounter == encounter.id
                    && trigger.x == position.x
                    && trigger.y == position.y
            }) {
                return Err(GameRuntimeError::InvalidCommand(format!(
                    "encounter {} is not triggered at the current dungeon cell",
                    encounter.id
                )));
            }
        }
        let next = next_available_encounter_definition(&self.rules, &adventure, campaign)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidCommand(
                    "the authored adventure has no incomplete encounter".to_owned(),
                )
            })?;
        if next.id != encounter.id {
            return Err(GameRuntimeError::InvalidCommand(format!(
                "encounter {} is not the next authored encounter; expected {}",
                encounter.id, next.id
            )));
        }
        let mut completed_opposition = BTreeSet::new();
        for completed in &campaign.completed_encounters {
            let completed_id = id(&completed.encounter_id)?;
            let completed_definition = self.rules.encounter(&completed_id).ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "completed encounter {completed_id} is missing"
                ))
            })?;
            completed_opposition.extend(
                completed_definition
                    .roster
                    .iter()
                    .filter(|participant| {
                        participant.faction == EncounterFactionDefinition::Opposition
                    })
                    .map(|participant| participant.character.clone()),
            );
        }
        let mut introduction_details = encounter.introduction_details.clone();
        for participant in encounter.roster.iter().filter(|participant| {
            participant.faction == EncounterFactionDefinition::Opposition
                && completed_opposition.contains(&participant.character)
        }) {
            let opponent = self
                .rules
                .character_template(&participant.character)
                .expect("compiled encounter participant exists")
                .clone();
            let serial = self.next_operation;
            let receipt = self.session_mut()?.restore_vitality(
                EntityId::new(opponent.entity_id),
                opponent.vitality,
                operation(&format!("encounter-recovery-{serial}"))?,
            )?;
            self.next_operation = self
                .next_operation
                .checked_add(1)
                .ok_or(GameRuntimeError::CounterOverflow)?;
            introduction_details.push(format!(
                "{} begins the next authored encounter with {}/{} vitality after {} bounded \
                 recovery; prior resources, effects, and loadout remain authoritative.",
                opponent.name,
                receipt.after.get(),
                opponent.vitality,
                receipt.applied_amount.get()
            ));
        }
        let initiative_ability = id("finesse")?;
        let participants = encounter
            .roster
            .iter()
            .map(|participant| {
                let character = self
                    .rules
                    .character_template(&participant.character)
                    .expect("compiled encounter participant exists");
                let initiative =
                    *character
                        .abilities
                        .get(&initiative_ability)
                        .ok_or_else(|| {
                            GameRuntimeError::InvalidState(format!(
                                "encounter participant {} has no finesse initiative",
                                character.id
                            ))
                        })?;
                Ok(EncounterParticipationSeed {
                    entity: EntityId::new(character.entity_id),
                    faction: match participant.faction {
                        EncounterFactionDefinition::Party => EncounterFaction::Party,
                        EncounterFactionDefinition::Opposition => EncounterFaction::Opposition,
                    },
                    initiative,
                    position: tactical_position(
                        encounter
                            .board
                            .placement(&participant.character)
                            .expect("compiled encounter participant has a placement"),
                    ),
                })
            })
            .collect::<Result<Vec<_>, GameRuntimeError>>()?;
        self.session_mut()?
            .install_encounter_participation(encounter.id.clone(), participants)?;
        let mut ordered = self.session()?.encounter_participants()?;
        ordered.sort_by(|left, right| {
            right
                .1
                .initiative()
                .cmp(&left.1.initiative())
                .then_with(|| left.0.raw().cmp(&right.0.raw()))
        });
        let first_actor = ordered
            .into_iter()
            .find_map(|(entity, _)| (self.vitality(entity).ok()? > 0).then_some(entity))
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "encounter roster has no living participant".to_owned(),
                )
            })?;
        self.session_mut()?.reset_activation_budgets(first_actor)?;
        let campaign = self
            .campaign
            .as_mut()
            .expect("campaign was validated before mutation");
        campaign.phase = CampaignPhase::Encounter;
        campaign.active_encounter_id = Some(encounter.id.to_string());
        campaign.resolved_encounter_id = None;
        campaign.current_actor_id = Some(first_actor.raw());
        campaign.outcome = None;
        self.bump_revision()?;
        self.saved_revision = None;
        self.push_log(
            GameLogKindDto::System,
            &encounter.introduction_source,
            &encounter.introduction_text,
            introduction_details,
        )?;
        self.settle_automatic_opposition()?;
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
        let adventure = self.adventure()?.clone();
        let item_definition = product_loadout_item(&self.rules, &adventure, item)?.clone();
        let equipment = item_definition.equipment;
        let (definition_id, slot) = self
            .rules
            .equipment_definition(&equipment)
            .expect("authored equipment exists in the compiled ruleset");
        let definition_id = definition_id.clone();
        let slot = slot.clone();
        if request.slot_id != slot.as_str() {
            return Err(GameRuntimeError::InvalidEquipmentSlot {
                requested: request.slot_id,
                required: slot.to_string(),
            });
        }
        let serial = self.next_operation;
        let owner = self
            .session()?
            .entities()
            .contained_in(item)
            .ok_or_else(|| {
                GameRuntimeError::InvalidContainment(format!(
                    "item {} has no canonical owner",
                    item.raw()
                ))
            })?;
        let owner_name = party_member_name(&self.rules, &adventure, owner)?;
        self.session_mut()?.equip_item(
            owner,
            item,
            &equipment,
            operation(&format!("equip-item-{serial}"))?,
        )?;
        self.next_operation = serial + 1;
        self.bump_revision()?;
        self.saved_revision = None;
        self.push_log(
            GameLogKindDto::System,
            "Loadout",
            &format!("Equipped {}.", humanize(definition_id.as_str())),
            vec![format!(
                "{} now occupies {}'s {} slot.",
                humanize(definition_id.as_str()),
                owner_name,
                humanize(slot.as_str())
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
        let adventure = self.adventure()?.clone();
        let equipment = product_loadout_item(&self.rules, &adventure, item)?
            .equipment
            .clone();
        let owner = self
            .session()?
            .entities()
            .contained_in(item)
            .ok_or_else(|| {
                GameRuntimeError::InvalidContainment(format!(
                    "item {} has no canonical owner",
                    item.raw()
                ))
            })?;
        let owner_name = party_member_name(&self.rules, &adventure, owner)?;
        let serial = self.next_operation;
        self.session_mut()?.unequip_item(
            owner,
            item,
            operation(&format!("unequip-item-{serial}"))?,
        )?;
        self.next_operation = serial + 1;
        self.bump_revision()?;
        self.saved_revision = None;
        self.push_log(
            GameLogKindDto::System,
            "Loadout",
            &format!("Unequipped {}.", humanize(equipment.id().as_str())),
            vec![format!("The item remains in {owner_name}'s inventory.")],
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
        let adventure = self.adventure()?.clone();
        let equipment = product_loadout_item(&self.rules, &adventure, item)?
            .equipment
            .clone();
        let from_owner = entity(request.from_owner_id)?;
        let to_owner = entity(request.to_owner_id)?;
        let stash = storage_entity(&self.rules, &adventure, &adventure.camp_storage)?;
        let party_entities = adventure
            .party
            .iter()
            .map(|member| character_entity(&self.rules, &adventure, member))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let allowed_owner = |owner: EntityId| owner == stash || party_entities.contains(&owner);
        if from_owner == to_owner || !allowed_owner(from_owner) || !allowed_owner(to_owner) {
            let stash_name = self
                .rules
                .storage(&adventure.camp_storage)
                .expect("compiled camp storage exists")
                .name
                .clone();
            return Err(GameRuntimeError::InvalidContainment(format!(
                "loadout transfers are limited to distinct party inventories and {stash_name}"
            )));
        }
        let serial = self.next_operation;
        self.session_mut()?.transfer_item(
            item,
            from_owner,
            to_owner,
            operation(&format!("transfer-item-{serial}"))?,
        )?;
        self.next_operation = serial + 1;
        self.bump_revision()?;
        self.saved_revision = None;
        let destination = if to_owner == stash {
            "the camp stash".to_owned()
        } else {
            format!(
                "{}'s inventory",
                party_member_name(&self.rules, &adventure, to_owner)?
            )
        };
        self.push_log(
            GameLogKindDto::System,
            "Loadout",
            &format!(
                "Moved {} to {destination}.",
                humanize(equipment.id().as_str())
            ),
            vec![format!(
                "Canonical containment now points to entity {}.",
                to_owner.raw()
            )],
        )?;
        self.snapshot()
    }

    pub fn move_loadout_item(
        &mut self,
        request: MoveLoadoutItemRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        let snapshot = staged.move_loadout_item_inner(request)?;
        *self = staged;
        Ok(snapshot)
    }

    fn move_loadout_item_inner(
        &mut self,
        request: MoveLoadoutItemRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_camp_phase()?;
        self.ensure_mutation_capacity(true, true)?;

        let item = entity(request.item_id)?;
        let from_owner = entity(request.from_owner_id)?;
        let to_owner = entity(request.to_owner_id)?;
        let adventure = self.adventure()?.clone();
        let item_definition = product_loadout_item(&self.rules, &adventure, item)?.clone();
        let equipment = item_definition.equipment.clone();
        let (_, required_slot) = self
            .rules
            .equipment_definition(&equipment)
            .expect("authored equipment exists in the compiled ruleset");
        if let Some(destination_slot) = &request.destination_slot_id {
            if destination_slot != required_slot.as_str() {
                return Err(GameRuntimeError::InvalidEquipmentSlot {
                    requested: destination_slot.clone(),
                    required: required_slot.to_string(),
                });
            }
        }

        let stash = storage_entity(&self.rules, &adventure, &adventure.camp_storage)?;
        let party_entities = adventure
            .party
            .iter()
            .map(|member| character_entity(&self.rules, &adventure, member))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let allowed_owner = |owner: EntityId| owner == stash || party_entities.contains(&owner);
        if !allowed_owner(from_owner) || !allowed_owner(to_owner) {
            return Err(GameRuntimeError::InvalidContainment(
                "loadout placement is limited to party inventories and the camp stash".to_owned(),
            ));
        }
        if request.destination_slot_id.is_some() && !party_entities.contains(&to_owner) {
            return Err(GameRuntimeError::InvalidContainment(
                "only a party member can receive equipped gear".to_owned(),
            ));
        }

        let actual_owner = self
            .session()?
            .entities()
            .contained_in(item)
            .ok_or_else(|| {
                GameRuntimeError::InvalidContainment(format!(
                    "item {} has no canonical owner",
                    item.raw()
                ))
            })?;
        if actual_owner != from_owner {
            return Err(GameRuntimeError::InvalidContainment(format!(
                "item {} belongs to entity {}, not requested source entity {}",
                item.raw(),
                actual_owner.raw(),
                from_owner.raw()
            )));
        }

        let equipped_slot = self
            .session()?
            .entities()
            .component::<EquipmentComponent>(from_owner)?
            .and_then(|equipment| {
                equipment
                    .assignments()
                    .iter()
                    .find(|assignment| assignment.item == item)
                    .map(|assignment| assignment.slot.to_string())
            });
        if from_owner == to_owner {
            match (&equipped_slot, &request.destination_slot_id) {
                (None, None) => {
                    return Err(GameRuntimeError::InvalidContainment(
                        "item is already in the requested inventory".to_owned(),
                    ));
                }
                (Some(current), Some(destination)) if current == destination => {
                    return Err(GameRuntimeError::InvalidEquipmentSlot {
                        requested: destination.clone(),
                        required: "an empty destination or inventory".to_owned(),
                    });
                }
                _ => {}
            }
        }

        let serial = self.next_operation;
        if equipped_slot.is_some() {
            self.session_mut()?.unequip_item(
                from_owner,
                item,
                operation(&format!("move-loadout-{serial}-unequip"))?,
            )?;
        }
        if from_owner != to_owner {
            self.session_mut()?.transfer_item(
                item,
                from_owner,
                to_owner,
                operation(&format!("move-loadout-{serial}-transfer"))?,
            )?;
        }
        if request.destination_slot_id.is_some() {
            self.session_mut()?.equip_item(
                to_owner,
                item,
                &equipment,
                operation(&format!("move-loadout-{serial}-equip"))?,
            )?;
        }

        self.next_operation = serial + 1;
        self.bump_revision()?;
        self.saved_revision = None;
        let destination = match &request.destination_slot_id {
            Some(slot) => format!(
                "{}'s {} slot",
                party_member_name(&self.rules, &adventure, to_owner)?,
                humanize(slot)
            ),
            None if to_owner == stash => "the camp inventory".to_owned(),
            None => format!(
                "{}'s pack",
                party_member_name(&self.rules, &adventure, to_owner)?
            ),
        };
        self.push_log(
            GameLogKindDto::System,
            "Loadout",
            &format!("Placed {} in {destination}.", item_definition.name),
            vec![
                "The transfer and equipment services committed as one Rust-owned operation."
                    .to_owned(),
            ],
        )?;
        self.snapshot()
    }

    pub fn choose_action(
        &mut self,
        request: ChooseActionRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        let snapshot = staged.choose_action_inner(request)?;
        *self = staged;
        Ok(snapshot)
    }

    fn choose_action_inner(
        &mut self,
        request: ChooseActionRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_encounter_phase()?;
        self.ensure_current_faction(EncounterFaction::Party)?;
        self.ensure_mutation_capacity(true, false)?;
        if self.pending.is_some() {
            return Err(GameRuntimeError::InvalidCommand(
                "resolve the current action preview before choosing another action".to_owned(),
            ));
        }
        let actor = entity(request.actor_id)?;
        let target = entity(request.target_id)?;
        let (current_actor, _) = self.current_actor()?;
        if actor != current_actor {
            return Err(GameRuntimeError::InvalidCommand(
                "the selected actor does not own the current activation".to_owned(),
            ));
        }
        let action = id(&request.action_id)?;
        let actor_definition = self
            .rules
            .character_templates()
            .find(|character| character.entity_id == actor.raw())
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "current actor {} has no compiled character template",
                    actor.raw()
                ))
            })?;
        if !actor_definition.actions.contains(&action) {
            return Err(GameRuntimeError::InvalidCommand(format!(
                "action {action} is not available to {}",
                actor_definition.name
            )));
        }
        let action_definition = self
            .rules
            .action(&action)
            .expect("compiled character action exists");
        if !self.action_target_team_is_legal(actor, target, action_definition)? {
            return Err(GameRuntimeError::InvalidCommand(format!(
                "target {} does not match {}'s authored target team",
                target.raw(),
                action
            )));
        }
        if !self.action_is_spatially_legal(actor, target, action_definition)? {
            return Err(GameRuntimeError::InvalidCommand(format!(
                "target {} is outside {} range or line of effect",
                target.raw(),
                action
            )));
        }
        let serial = self.next_operation;
        let operation = operation(&format!("action-{serial}"))?;
        let preview = self
            .session()?
            .preview_action(actor, target, &action, operation)?;
        self.next_operation = self
            .next_operation
            .checked_add(1)
            .ok_or(GameRuntimeError::CounterOverflow)?;
        self.resolve_pending_action(PendingAction {
            serial,
            token: format!("preview-{serial}"),
            preview,
        })
    }

    pub fn move_actor(
        &mut self,
        request: MoveActorRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        let snapshot = staged.move_actor_inner(request)?;
        *self = staged;
        Ok(snapshot)
    }

    fn move_actor_inner(
        &mut self,
        request: MoveActorRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_encounter_phase()?;
        self.ensure_current_faction(EncounterFaction::Party)?;
        self.ensure_mutation_capacity(true, true)?;
        if self.pending.is_some() {
            return Err(GameRuntimeError::InvalidCommand(
                "resolve the pending action before moving".to_owned(),
            ));
        }
        let actor = entity(request.actor_id)?;
        let (current_actor, _) = self.current_actor()?;
        if actor != current_actor {
            return Err(GameRuntimeError::InvalidCommand(
                "the selected actor does not own the current activation".to_owned(),
            ));
        }
        let destination = TacticalPosition::new(request.x, request.y);
        let route = self
            .legal_tactical_routes(actor)?
            .into_iter()
            .find(|route| route.destination == destination)
            .ok_or_else(|| {
                GameRuntimeError::InvalidCommand(format!(
                    "cell ({}, {}) is not a legal destination for the current movement budget",
                    request.x, request.y
                ))
            })?;
        let movement_cost = u16::try_from(route.path.len().saturating_sub(1))
            .map_err(|_| GameRuntimeError::CounterOverflow)?;
        let origin = self.participant_position(actor)?;
        self.session_mut()?
            .relocate_encounter_participant(actor, destination, movement_cost)?;
        self.bump_revision()?;
        self.saved_revision = None;
        self.push_log(
            GameLogKindDto::Turn,
            "Movement",
            &format!(
                "{} moved from ({}, {}) to ({}, {}).",
                self.character_name(actor)?,
                origin.x(),
                origin.y(),
                destination.x(),
                destination.y()
            ),
            vec![format!(
                "Engine pathfinding admitted a {movement_cost}-square route: {}.",
                route
                    .path
                    .iter()
                    .map(|position| format!("({}, {})", position.x(), position.y()))
                    .collect::<Vec<_>>()
                    .join(" → ")
            )],
        )?;
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
        let pending = self.require_pending(&request.prompt_token)?.clone();
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
        let pending = PendingAction {
            preview: fresh,
            ..pending
        };
        self.log_reaction(&receipt)?;
        self.resolve_pending_action(pending)
    }

    pub fn decline_reaction(
        &mut self,
        request: DeclineReactionRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        let snapshot = staged.decline_reaction_inner(request)?;
        *self = staged;
        Ok(snapshot)
    }

    fn decline_reaction_inner(
        &mut self,
        request: DeclineReactionRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_encounter_phase()?;
        self.ensure_mutation_capacity(false, true)?;
        let pending = self.require_pending(&request.prompt_token)?.clone();
        self.resolve_pending_action(pending)
    }

    fn resolve_pending_action(
        &mut self,
        pending: PendingAction,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.resolve_pending_action_once(pending)?;
        self.settle_automatic_opposition()?;
        self.snapshot()
    }

    fn resolve_pending_action_once(
        &mut self,
        pending: PendingAction,
    ) -> Result<(), GameRuntimeError> {
        let (expected_actor, _) = self.current_actor()?;
        if pending.preview.actor() != expected_actor {
            return Err(GameRuntimeError::InvalidCommand(
                "the pending action does not belong to the current actor".to_owned(),
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
            format!("Roll-source position {}.", receipt.roll_index),
        ];
        if receipt.hit && action_definition.forced_movement > 0 {
            let actor_position = self.participant_position(receipt.actor)?;
            let target_position = self.participant_position(receipt.target)?;
            let destination = forced_destination(
                self.tactical_board()?,
                &self.occupied_positions(Some(receipt.target))?,
                actor_position,
                target_position,
                action_definition.forced_movement,
            );
            if destination != target_position {
                self.session_mut()?.relocate_encounter_participant(
                    receipt.target,
                    destination,
                    0,
                )?;
                details.push(format!(
                    "{} was forced from ({}, {}) to ({}, {}) without spending movement.",
                    self.character_name(receipt.target)?,
                    target_position.x(),
                    target_position.y(),
                    destination.x(),
                    destination.y()
                ));
            } else {
                details.push("Forced movement was blocked by terrain or occupancy.".to_owned());
            }
        }
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

        if let Some(encounter_outcome) = self.encounter_outcome()? {
            self.complete_encounter(encounter_outcome)?;
        } else {
            self.advance_activation(Vec::new())?;
        }
        self.bump_revision()?;
        self.saved_revision = None;
        Ok(())
    }

    fn advance_opposition_activation(&mut self) -> Result<(), GameRuntimeError> {
        self.ensure_encounter_phase()?;
        self.ensure_current_faction(EncounterFaction::Opposition)?;
        self.ensure_mutation_capacity(true, true)?;
        if self.pending.is_some() {
            return Err(GameRuntimeError::InvalidCommand(
                "the opposition action is already pending".to_owned(),
            ));
        }
        let encounter = current_encounter_definition(
            &self.rules,
            self.adventure()?,
            self.campaign
                .as_ref()
                .ok_or(GameRuntimeError::NoEncounter)?,
        )?
        .clone();
        let (actor, _) = self.current_actor()?;
        let opponent = self
            .rules
            .character_templates()
            .find(|character| character.entity_id == actor.raw())
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "current opposition actor {} has no compiled character",
                    actor.raw()
                ))
            })?
            .clone();
        let serial = self.next_operation;
        let operation = operation(&format!("opposition-action-{serial}"))?;
        let movement_targets = self
            .living_participants()?
            .into_iter()
            .filter(|(_, faction, _)| *faction == EncounterFaction::Party)
            .map(|(entity, _, _)| entity)
            .collect::<Vec<_>>();
        let targets = self
            .living_participants()?
            .into_iter()
            .map(|(entity, _, _)| entity)
            .collect::<Vec<_>>();
        let (mut legal_actions, mut unavailable) =
            self.legal_action_previews(actor, &opponent.actions, &targets, &operation)?;
        let mut movement_detail = None;
        if legal_actions.is_empty() {
            if let Some(route) = self.opposition_movement_route(actor, &movement_targets)? {
                let origin = self.participant_position(actor)?;
                let movement_cost = u16::try_from(route.path.len().saturating_sub(1))
                    .map_err(|_| GameRuntimeError::CounterOverflow)?;
                match self.session_mut()?.relocate_encounter_participant(
                    actor,
                    route.destination,
                    movement_cost,
                ) {
                    Ok(()) => {
                        movement_detail = Some(format!(
                            "{} moved from ({}, {}) to ({}, {}) along an Engine-admitted {}-square route.",
                            opponent.name,
                            origin.x(),
                            origin.y(),
                            route.destination.x(),
                            route.destination.y(),
                            movement_cost
                        ));
                        (legal_actions, unavailable) = self.legal_action_previews(
                            actor,
                            &opponent.actions,
                            &targets,
                            &operation,
                        )?;
                    }
                    Err(D20SessionError::MovementForbidden { effect, .. }) => {
                        movement_detail = Some(format!(
                            "{} could not move because {} forbids voluntary movement.",
                            opponent.name,
                            humanize(effect.as_str())
                        ));
                    }
                    Err(error) => return Err(GameRuntimeError::Session(error)),
                }
            }
        }
        if legal_actions.is_empty() {
            let mut details = movement_detail.into_iter().collect::<Vec<_>>();
            details.push(format!(
                    "{} had no legal authored action/target pair after tactical movement; {} unavailable choice(s) were skipped.",
                    opponent.name,
                    unavailable
                ));
            self.advance_activation(details)?;
            self.bump_revision()?;
            self.saved_revision = None;
            return Ok(());
        }
        let upper = u32::try_from(legal_actions.len()).map_err(|_| {
            GameRuntimeError::InvalidState(
                "the opposition action catalog does not fit the choice policy".to_owned(),
            )
        })?;
        let index = self
            .session()?
            .choice_index(&format!("{}-{}-action", encounter.id, actor.raw()), upper)
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "the opposition has no admitted action choices".to_owned(),
                )
            })?;
        let index = usize::try_from(index).expect("u32 choice index fits usize");
        let (action, target, preview) = legal_actions[index].clone();
        self.next_operation = self
            .next_operation
            .checked_add(1)
            .ok_or(GameRuntimeError::CounterOverflow)?;
        let pending = PendingAction {
            serial,
            token: format!("preview-{serial}"),
            preview,
        };
        self.push_log(
            GameLogKindDto::Turn,
            "Opposition",
            &format!(
                "{} prepares {} against {}.",
                opponent.name,
                humanize(action.as_str()),
                self.character_name(target)?
            ),
            movement_detail.into_iter().chain([format!(
                "Opposition policy selected legal choice {} of {}; {} unavailable authored choice(s) were excluded.",
                index + 1,
                legal_actions.len(),
                unavailable
            )]).collect(),
        )?;
        if pending.preview.reactions().is_empty() {
            self.resolve_pending_action_once(pending)
        } else {
            self.pending = Some(pending);
            self.bump_revision()?;
            self.saved_revision = None;
            Ok(())
        }
    }

    fn settle_automatic_opposition(&mut self) -> Result<(), GameRuntimeError> {
        for _ in 0..MAX_D20_ENCOUNTER_PARTICIPANTS {
            if self.pending.is_some()
                || !self
                    .campaign
                    .as_ref()
                    .is_some_and(|campaign| campaign.phase == CampaignPhase::Encounter)
            {
                return Ok(());
            }
            let (_, faction) = self.current_actor()?;
            if faction == EncounterFaction::Party {
                return Ok(());
            }
            self.advance_opposition_activation()?;
        }

        if self.pending.is_none()
            && self
                .campaign
                .as_ref()
                .is_some_and(|campaign| campaign.phase == CampaignPhase::Encounter)
            && self.current_actor()?.1 == EncounterFaction::Opposition
        {
            return Err(GameRuntimeError::InvalidState(format!(
                "automatic opposition progression exceeded the admitted {MAX_D20_ENCOUNTER_PARTICIPANTS}-participant bound"
            )));
        }
        Ok(())
    }

    pub fn end_activation(
        &mut self,
        expected_revision: u64,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        staged.ensure_revision(expected_revision)?;
        staged.ensure_encounter_phase()?;
        staged.ensure_current_faction(EncounterFaction::Party)?;
        staged.ensure_mutation_capacity(true, true)?;
        if staged.pending.is_some() {
            return Err(GameRuntimeError::InvalidCommand(
                "resolve the pending action before ending the activation".to_owned(),
            ));
        }
        let (actor, _) = staged.current_actor()?;
        let name = staged.character_name(actor)?;
        staged.advance_activation(vec![format!(
            "{name} ended the activation without spending another action."
        )])?;
        staged.bump_revision()?;
        staged.saved_revision = None;
        staged.settle_automatic_opposition()?;
        let snapshot = staged.snapshot()?;
        *self = staged;
        Ok(snapshot)
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
        let adventure = self.adventure()?.clone();
        let encounter = current_encounter_definition(
            &self.rules,
            &adventure,
            self.campaign
                .as_ref()
                .ok_or(GameRuntimeError::NoEncounter)?,
        )?
        .clone();
        let party = adventure
            .party
            .iter()
            .map(|member| {
                self.rules
                    .character_template(member)
                    .expect("compiled party member exists")
                    .clone()
            })
            .collect::<Vec<_>>();
        let mut details = Vec::new();
        if outcome == EncounterOutcome::Defeat {
            let recovery = encounter.defeat.recovery_vitality.ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "encounter {} has no defeat recovery",
                    encounter.id
                ))
            })?;
            for member in &party {
                let serial = self.next_operation;
                let receipt = self.session_mut()?.restore_vitality(
                    EntityId::new(member.entity_id),
                    recovery,
                    operation(&format!("camp-recovery-{serial}"))?,
                )?;
                self.next_operation = self
                    .next_operation
                    .checked_add(1)
                    .ok_or(GameRuntimeError::CounterOverflow)?;
                details.push(format!(
                    "Camp recovery restored {} vitality; {} returns with {}/{} vitality.",
                    receipt.applied_amount.get(),
                    member.name,
                    receipt.after.get(),
                    member.vitality
                ));
            }
        } else {
            details.push(format!(
                "{} party members keep their remaining vitality and resources.",
                party.len()
            ));
            if let Some(reward) = encounter.victory.reward_label {
                details.push(format!("{reward} remains in canonical camp storage."));
            }
        }
        let campaign = self
            .campaign
            .as_ref()
            .ok_or(GameRuntimeError::NoEncounter)?;
        let adventure_complete =
            next_available_encounter_definition(&self.rules, &adventure, campaign)?.is_none();
        let continue_exploring = outcome == EncounterOutcome::Victory
            && campaign.exploration.is_some()
            && !adventure_complete;
        {
            let campaign = self.campaign_mut()?;
            campaign.phase = if adventure_complete {
                CampaignPhase::AdventureComplete
            } else if continue_exploring {
                CampaignPhase::Exploration
            } else {
                CampaignPhase::Camp
            };
            campaign.active_encounter_id = None;
            campaign.current_actor_id = None;
            if !continue_exploring && !adventure_complete {
                if let Some(exploration) = campaign.exploration.as_mut() {
                    let checkpoint = adventure
                        .dungeon
                        .checkpoint(&exploration.checkpoint_id)
                        .ok_or_else(|| {
                            GameRuntimeError::InvalidState(
                                "active dungeon checkpoint is missing".to_owned(),
                            )
                        })?;
                    exploration.position = DungeonPosition {
                        x: checkpoint.x,
                        y: checkpoint.y,
                    };
                    exploration.facing = adventure.dungeon.start_facing;
                    exploration.discovered.insert(exploration.position);
                }
            }
        }
        self.session_mut()?.clear_encounter_participation()?;
        self.bump_revision()?;
        self.saved_revision = None;
        if adventure_complete {
            details.extend(adventure.completion.details.clone());
        }
        self.push_log(
            GameLogKindDto::System,
            if adventure_complete {
                &adventure.completion.source
            } else if continue_exploring {
                "Expedition"
            } else {
                "Camp"
            },
            if adventure_complete {
                match outcome {
                    EncounterOutcome::Victory => &adventure.completion.victory_text,
                    EncounterOutcome::Defeat => &adventure.completion.defeat_text,
                }
            } else if continue_exploring {
                "The party returns to the exact dungeon location."
            } else {
                "The encounter consequence is now part of the durable camp state."
            },
            details,
        )?;
        self.snapshot()
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
        let adventure = self.adventure()?.clone();
        let encounter = current_encounter_definition(
            &self.rules,
            &adventure,
            self.campaign
                .as_ref()
                .ok_or(GameRuntimeError::NoEncounter)?,
        )?
        .clone();
        let authored_outcome = match outcome {
            EncounterOutcome::Victory => encounter.victory.clone(),
            EncounterOutcome::Defeat => encounter.defeat.clone(),
        };
        let mut details = authored_outcome.log_details.clone();
        match outcome {
            EncounterOutcome::Victory => {
                if let Some(reward) = authored_outcome.reward_item.as_ref() {
                    let reward_entity = self
                        .rules
                        .item_instance(reward)
                        .expect("compiled reward exists")
                        .entity_id;
                    let session = self.session.as_mut().ok_or(GameRuntimeError::NoEncounter)?;
                    transfer_victory_reward(
                        &self.rules,
                        &adventure,
                        &encounter,
                        session,
                        &mut self.next_operation,
                    )?;
                    details.push(format!(
                        "Reward item entity {reward_entity} can be inspected after returning to camp."
                    ));
                }
            }
            EncounterOutcome::Defeat => {}
        }
        {
            let campaign = self.campaign_mut()?;
            if campaign
                .completed_encounters
                .iter()
                .any(|completed| completed.encounter_id == encounter.id.as_str())
            {
                return Err(GameRuntimeError::InvalidState(format!(
                    "encounter {} was already completed",
                    encounter.id
                )));
            }
            campaign.phase = CampaignPhase::Outcome;
            campaign.resolved_encounter_id = campaign.active_encounter_id.clone();
            campaign.current_actor_id = None;
            campaign.outcome = Some(outcome);
            campaign.completed_encounters.push(CompletedEncounter {
                encounter_id: encounter.id.to_string(),
                outcome,
            });
        }
        self.push_log(
            GameLogKindDto::System,
            &authored_outcome.log_source,
            &authored_outcome.log_text,
            details,
        )
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

    fn adventure(&self) -> Result<&AdventureDefinition, GameRuntimeError> {
        self.rules.adventure(&self.adventure_id).ok_or_else(|| {
            GameRuntimeError::InvalidState(format!(
                "compiled adventure {} is missing",
                self.adventure_id
            ))
        })
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

    fn ordered_participants(
        &self,
    ) -> Result<Vec<(EntityId, EncounterFaction, i16)>, GameRuntimeError> {
        let active_encounter = self
            .campaign
            .as_ref()
            .and_then(|campaign| campaign.active_encounter_id.as_deref())
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "active encounter is missing its identity".to_owned(),
                )
            })?;
        let active_encounter = id(active_encounter)?;
        let mut participants = self
            .session()?
            .encounter_participants()?
            .into_iter()
            .filter(|(_, participation)| participation.encounter() == &active_encounter)
            .map(|(entity, participation)| {
                (entity, participation.faction(), participation.initiative())
            })
            .collect::<Vec<_>>();
        participants.sort_by(|left, right| {
            right
                .2
                .cmp(&left.2)
                .then_with(|| left.0.raw().cmp(&right.0.raw()))
        });
        if participants.is_empty() {
            return Err(GameRuntimeError::InvalidState(format!(
                "encounter {active_encounter} has no canonical participants"
            )));
        }
        Ok(participants)
    }

    fn living_participants(
        &self,
    ) -> Result<Vec<(EntityId, EncounterFaction, i16)>, GameRuntimeError> {
        self.ordered_participants()?
            .into_iter()
            .filter_map(|participant| match self.vitality(participant.0) {
                Ok(vitality) if vitality > 0 => Some(Ok(participant)),
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .collect()
    }

    fn participant_position(&self, entity: EntityId) -> Result<TacticalPosition, GameRuntimeError> {
        self.session()?
            .encounter_participation(entity)?
            .map(|participation| participation.position())
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "participant {} has no canonical tactical position",
                    entity.raw()
                ))
            })
    }

    fn occupied_positions(
        &self,
        excluded: Option<EntityId>,
    ) -> Result<BTreeSet<TacticalPosition>, GameRuntimeError> {
        self.ordered_participants()?
            .into_iter()
            .filter(|(entity, _, _)| Some(*entity) != excluded)
            .map(|(entity, _, _)| self.participant_position(entity))
            .collect()
    }

    fn tactical_board(&self) -> Result<&TacticalBoardDefinition, GameRuntimeError> {
        Ok(&current_encounter_definition(
            &self.rules,
            self.adventure()?,
            self.campaign
                .as_ref()
                .ok_or(GameRuntimeError::NoEncounter)?,
        )?
        .board)
    }

    fn action_range(&self, action: &ActionDefinition) -> Result<u16, GameRuntimeError> {
        match &action.attack {
            ActionAttackDefinition::Fixed { range, .. } => Ok(*range),
            ActionAttackDefinition::Implement { implement } => self
                .rules
                .implement(implement)
                .map(|definition| definition.range)
                .ok_or_else(|| {
                    GameRuntimeError::InvalidState(format!(
                        "action {} references missing implement {}",
                        action.id, implement
                    ))
                }),
        }
    }

    fn action_is_spatially_legal(
        &self,
        actor: EntityId,
        target: EntityId,
        action: &ActionDefinition,
    ) -> Result<bool, GameRuntimeError> {
        if actor == target {
            return Ok(matches!(
                action.target.team,
                ActionTargetTeamDefinition::SelfOnly | ActionTargetTeamDefinition::Any
            ));
        }
        action_is_spatially_legal(
            self.tactical_board()?,
            self.participant_position(actor)?,
            self.participant_position(target)?,
            self.action_range(action)?,
            action.target.line_of_effect,
        )
        .map_err(GameRuntimeError::InvalidState)
    }

    fn action_target_team_is_legal(
        &self,
        actor: EntityId,
        target: EntityId,
        action: &ActionDefinition,
    ) -> Result<bool, GameRuntimeError> {
        let session = self.session()?;
        let actor_faction = session
            .encounter_participation(actor)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "actor {} is not an encounter participant",
                    actor.raw()
                ))
            })?
            .faction();
        let target_faction = session
            .encounter_participation(target)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidCommand(
                    "the selected target is not an encounter participant".to_owned(),
                )
            })?
            .faction();
        if self.vitality(target)? == 0 {
            return Ok(false);
        }
        Ok(target_team_allows(
            action.target.team,
            actor,
            actor_faction,
            target,
            target_faction,
        ))
    }

    fn legal_tactical_routes(
        &self,
        actor: EntityId,
    ) -> Result<Vec<TacticalRoute>, GameRuntimeError> {
        if self
            .session()?
            .active_movement_prohibition(actor)?
            .is_some()
        {
            return Ok(Vec::new());
        }
        let movement = id("movement")?;
        let available = self
            .session()?
            .activation_budgets(actor)?
            .current(&movement)
            .unwrap_or(0);
        legal_routes(
            self.tactical_board()?,
            &self.occupied_positions(Some(actor))?,
            self.participant_position(actor)?,
            available,
        )
        .map_err(GameRuntimeError::InvalidState)
    }

    fn legal_action_previews(
        &self,
        actor: EntityId,
        actions: &[D20Id],
        targets: &[EntityId],
        operation: &OperationId,
    ) -> Result<(Vec<LegalActionPreview>, usize), GameRuntimeError> {
        let mut unavailable = 0_usize;
        let previews = actions
            .iter()
            .flat_map(|action| targets.iter().map(move |target| (action, *target)))
            .filter_map(|(action, target)| {
                let definition = self
                    .rules
                    .action(action)
                    .expect("compiled character action exists");
                match self.action_target_team_is_legal(actor, target, definition) {
                    Ok(true) => {}
                    Ok(false) => return None,
                    Err(error) => return Some(Err(error)),
                }
                match self.action_is_spatially_legal(actor, target, definition) {
                    Ok(true) => {}
                    Ok(false) => {
                        unavailable += 1;
                        return None;
                    }
                    Err(error) => return Some(Err(error)),
                }
                match self.session().and_then(|session| {
                    session
                        .preview_action(actor, target, action, operation.clone())
                        .map_err(GameRuntimeError::Session)
                }) {
                    Ok(preview) => Some(Ok((action.clone(), target, preview))),
                    Err(GameRuntimeError::Session(error))
                        if is_unavailable_action_error(&error) =>
                    {
                        unavailable += 1;
                        None
                    }
                    Err(error) => Some(Err(error)),
                }
            })
            .collect::<Result<Vec<_>, GameRuntimeError>>()?;
        Ok((previews, unavailable))
    }

    fn opposition_movement_route(
        &self,
        actor: EntityId,
        targets: &[EntityId],
    ) -> Result<Option<TacticalRoute>, GameRuntimeError> {
        let target_positions = targets
            .iter()
            .map(|target| self.participant_position(*target))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self
            .legal_tactical_routes(actor)?
            .into_iter()
            .min_by_key(|route| {
                let distance = target_positions
                    .iter()
                    .map(|target| {
                        route
                            .destination
                            .x()
                            .abs_diff(target.x())
                            .max(route.destination.y().abs_diff(target.y()))
                    })
                    .min()
                    .unwrap_or(u16::MAX);
                (
                    distance,
                    route.path.len(),
                    route.destination.y(),
                    route.destination.x(),
                )
            }))
    }

    fn current_actor(&self) -> Result<(EntityId, EncounterFaction), GameRuntimeError> {
        let actor = self
            .campaign
            .as_ref()
            .and_then(|campaign| campaign.current_actor_id)
            .map(EntityId::new)
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "active encounter is missing its current actor".to_owned(),
                )
            })?;
        let participation = self
            .session()?
            .encounter_participation(actor)?
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "current actor {} is not an encounter participant",
                    actor.raw()
                ))
            })?;
        if self.vitality(actor)? == 0 {
            return Err(GameRuntimeError::InvalidState(format!(
                "defeated participant {} owns the current activation",
                actor.raw()
            )));
        }
        Ok((actor, participation.faction()))
    }

    fn encounter_outcome(&self) -> Result<Option<EncounterOutcome>, GameRuntimeError> {
        let participants = self.ordered_participants()?;
        let mut party_alive = false;
        let mut opposition_alive = false;
        for (entity, faction, _) in participants {
            if self.vitality(entity)? == 0 {
                continue;
            }
            match faction {
                EncounterFaction::Party => party_alive = true,
                EncounterFaction::Opposition => opposition_alive = true,
            }
        }
        Ok(match (party_alive, opposition_alive) {
            (true, true) => None,
            (true, false) => Some(EncounterOutcome::Victory),
            (false, true) | (false, false) => Some(EncounterOutcome::Defeat),
        })
    }

    fn advance_activation(&mut self, mut details: Vec<String>) -> Result<(), GameRuntimeError> {
        let (current, _) = self.current_actor()?;
        let ordered = self.ordered_participants()?;
        let current_index = ordered
            .iter()
            .position(|participant| participant.0 == current)
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "current actor is absent from canonical initiative order".to_owned(),
                )
            })?;
        let mut selected = None;
        for offset in 1..=ordered.len() {
            let index = (current_index + offset) % ordered.len();
            if self.vitality(ordered[index].0)? > 0 {
                selected = Some((index, ordered[index]));
                break;
            }
        }
        let (next_index, (next_actor, _, _)) = selected.ok_or_else(|| {
            GameRuntimeError::InvalidState(
                "encounter has no living participant after a nonterminal action".to_owned(),
            )
        })?;
        let wrapped = next_index <= current_index;
        if wrapped {
            let serial = self.next_operation;
            let next_round = self
                .session()?
                .current_turn()
                .checked_add(1)
                .ok_or(GameRuntimeError::CounterOverflow)?;
            let receipt = self
                .session_mut()?
                .advance_turn(next_round, operation(&format!("advance-round-{serial}"))?)?;
            self.next_operation = self
                .next_operation
                .checked_add(1)
                .ok_or(GameRuntimeError::CounterOverflow)?;
            details.push(format!(
                "{} scheduled effect(s) expired at the round boundary.",
                receipt.expired.len()
            ));
        }
        self.session_mut()?.reset_activation_budgets(next_actor)?;
        self.campaign_mut()?.current_actor_id = Some(next_actor.raw());
        let name = self.character_name(next_actor)?;
        self.push_log(
            GameLogKindDto::Turn,
            if wrapped { "Round" } else { "Initiative" },
            &format!("{name} begins the next activation."),
            details,
        )
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
            Some(
                CampaignPhase::Camp
                | CampaignPhase::Exploration
                | CampaignPhase::Outcome
                | CampaignPhase::AdventureComplete,
            ) => Err(GameRuntimeError::WrongPhase(
                "this command is only available during an active encounter".to_owned(),
            )),
            None => Err(GameRuntimeError::NoEncounter),
        }
    }

    fn ensure_camp_phase(&self) -> Result<(), GameRuntimeError> {
        match self.campaign.as_ref().map(|campaign| campaign.phase) {
            Some(CampaignPhase::Camp) => Ok(()),
            Some(
                CampaignPhase::Exploration
                | CampaignPhase::Encounter
                | CampaignPhase::Outcome
                | CampaignPhase::AdventureComplete,
            ) => Err(GameRuntimeError::WrongPhase(
                "loadout changes are only available at camp".to_owned(),
            )),
            None => Err(GameRuntimeError::NoEncounter),
        }
    }

    fn ensure_outcome_phase(&self) -> Result<(), GameRuntimeError> {
        match self.campaign.as_ref().map(|campaign| campaign.phase) {
            Some(CampaignPhase::Outcome) => Ok(()),
            Some(
                CampaignPhase::Camp
                | CampaignPhase::Exploration
                | CampaignPhase::Encounter
                | CampaignPhase::AdventureComplete,
            ) => Err(GameRuntimeError::WrongPhase(
                "return to camp is only available after an encounter outcome".to_owned(),
            )),
            None => Err(GameRuntimeError::NoEncounter),
        }
    }

    fn ensure_current_faction(&self, expected: EncounterFaction) -> Result<(), GameRuntimeError> {
        let (_, actual) = self.current_actor()?;
        if actual != expected {
            let owner = match actual {
                EncounterFaction::Party => "party",
                EncounterFaction::Opposition => "opposition",
            };
            return Err(GameRuntimeError::WrongPhase(format!(
                "this command is not legal during the {owner} activation"
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

fn entity(raw: u64) -> Result<EntityId, GameRuntimeError> {
    if raw == 0 {
        return Err(GameRuntimeError::InvalidCommand(
            "entity identity must be nonzero".to_owned(),
        ));
    }
    Ok(EntityId::new(raw))
}

fn party_member_name(
    rules: &D20Ruleset,
    adventure: &AdventureDefinition,
    entity: EntityId,
) -> Result<String, GameRuntimeError> {
    adventure
        .party
        .iter()
        .filter_map(|member| rules.character_template(member))
        .find(|member| member.entity_id == entity.raw())
        .map(|member| member.name.clone())
        .ok_or_else(|| {
            GameRuntimeError::InvalidContainment(format!(
                "entity {} is not an authored party member",
                entity.raw()
            ))
        })
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

fn is_unavailable_action_error(error: &D20SessionError) -> bool {
    matches!(
        error,
        D20SessionError::ActionForbidden { .. }
            | D20SessionError::RequiredImplementNotEquipped { .. }
            | D20SessionError::ActivationBudgetUnavailable { .. }
    )
}

#[cfg(test)]
mod tests;
