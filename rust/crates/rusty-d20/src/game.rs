use std::collections::BTreeMap;

use core_ids::EntityId;
use entity_state::ComponentAccessError;
use gameplay_mechanics::{
    ActiveEffectsComponent, DecisionOutcome, EffectInstanceId, EquipmentComponent,
    InventoryComponent, ItemComponent, MechanicsError, OperationId, ResponseDecisionKind,
    SourceInstanceIdentity, StatContribution, StatService, TracksComponent,
};
use serde::{Deserialize, Serialize};
use svc_rng::RngSeed;

use crate::adventure::AuthoredAdventureCatalog;
use crate::compiler::defense_stat_id;
use crate::{
    AbilityScore, ActionPreview, ActionResource, ActionResourcesComponent, AdventureDefinition,
    AffinitySeed, ApplyActionRequest, ArmorItemSeed, CharacterAffinityKindDefinition,
    CharacterSeed, CharacterTemplateDefinition, D20CompileError, D20Id, D20Ruleset, D20Session,
    D20SessionError, DamageAffinity, EncounterDefinition, InventorySeed, ItemInstanceDefinition,
    ItemRarityDefinition, ReactionReceipt, ScheduledEffectsComponent, SessionSaveError,
    StorageSeed, ENGINE_REVISION,
};

const GAME_SAVE_SCHEMA_VERSION: u32 = 5;
const MAX_LOG_ENTRIES: usize = 64;
const MAX_LOG_DETAILS: usize = 32;
const MAX_LOG_SOURCE_BYTES: usize = 128;
const MAX_LOG_TEXT_BYTES: usize = 512;
const MAX_LOG_DETAIL_BYTES: usize = 512;
const MAX_GAME_SAVE_BYTES: usize = 1_000_000;

mod content;
mod dto;
mod persistence;
mod projection;

use content::*;
pub use dto::*;

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
    resolved_encounter_id: Option<String>,
    turn_owner: Option<EncounterTurnOwner>,
    outcome: Option<EncounterOutcome>,
}

#[derive(Debug, Clone)]
pub struct GameRuntime {
    catalog: AuthoredAdventureCatalog,
    rules: D20Ruleset,
    adventure_id: D20Id,
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
        let catalog = AuthoredAdventureCatalog::builtin().map_err(GameRuntimeError::Catalog)?;
        let adventure = catalog.default_adventure().clone();
        let rules = catalog
            .rules_for(&adventure)
            .map_err(GameRuntimeError::Catalog)?;
        Self::empty_with_rules(catalog, rules, adventure)
    }

    pub fn empty_for(adventure: &str) -> Result<Self, GameRuntimeError> {
        let adventure = id(adventure)?;
        let catalog = AuthoredAdventureCatalog::builtin().map_err(GameRuntimeError::Catalog)?;
        let rules = catalog
            .rules_for(&adventure)
            .map_err(GameRuntimeError::Catalog)?;
        Self::empty_with_rules(catalog, rules, adventure)
    }

    fn empty_with_rules(
        catalog: AuthoredAdventureCatalog,
        rules: D20Ruleset,
        adventure_id: D20Id,
    ) -> Result<Self, GameRuntimeError> {
        if rules.adventure(&adventure_id).is_none() {
            return Err(GameRuntimeError::Catalog(format!(
                "compiled rules do not define adventure {adventure_id}"
            )));
        }
        Ok(Self {
            catalog,
            rules,
            adventure_id,
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
        let mut staged = Self::empty_with_rules(self.catalog.clone(), rules, adventure_id)?;
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
        let mut session = D20Session::new_with_loadout(
            self.rules.clone(),
            RngSeed::new(0xD20_2026),
            characters,
            inventories,
            storage,
            product_armor_items(&self.rules, &adventure)?,
        )?;
        equip_initial_loadout(&self.rules, &adventure, &mut session)?;
        self.campaign = Some(CampaignState {
            phase: CampaignPhase::Camp,
            active_encounter_id: None,
            resolved_encounter_id: None,
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
        if !encounter.available_from_camp {
            return Err(GameRuntimeError::InvalidCommand(format!(
                "encounter {} is not available from camp",
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
                "the adventure encounter has already been resolved".to_owned(),
            ));
        }
        let campaign = self
            .campaign
            .as_mut()
            .expect("campaign was validated before mutation");
        campaign.phase = CampaignPhase::Encounter;
        campaign.active_encounter_id = Some(encounter.id.to_string());
        campaign.resolved_encounter_id = None;
        campaign.turn_owner = Some(EncounterTurnOwner::Player);
        campaign.outcome = None;
        self.bump_revision()?;
        self.saved_revision = None;
        self.push_log(
            GameLogKindDto::System,
            &encounter.introduction_source,
            &encounter.introduction_text,
            encounter.introduction_details.clone(),
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
        let adventure = self.adventure()?.clone();
        let item_definition = product_loadout_item(&self.rules, &adventure, item)?.clone();
        let armor = item_definition.armor;
        let definition = self
            .rules
            .armor(&armor)
            .expect("authored product armor exists in the compiled ruleset")
            .clone();
        if request.slot_id != definition.slot.as_str() {
            return Err(GameRuntimeError::InvalidEquipmentSlot {
                requested: request.slot_id,
                required: definition.slot.to_string(),
            });
        }
        let serial = self.next_operation;
        let hero = character_entity(&self.rules, &adventure, &adventure.hero)?;
        self.session_mut()?.equip_armor(
            hero,
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
        let adventure = self.adventure()?.clone();
        let armor = product_loadout_item(&self.rules, &adventure, item)?
            .armor
            .clone();
        let hero_name = self
            .rules
            .character_template(&adventure.hero)
            .expect("compiled hero exists")
            .name
            .clone();
        let hero = character_entity(&self.rules, &adventure, &adventure.hero)?;
        let serial = self.next_operation;
        self.session_mut()?.unequip_armor(
            hero,
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
            vec![format!("The item remains in {hero_name}'s inventory.")],
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
        let armor = product_loadout_item(&self.rules, &adventure, item)?
            .armor
            .clone();
        let from_owner = entity(request.from_owner_id)?;
        let to_owner = entity(request.to_owner_id)?;
        let hero = character_entity(&self.rules, &adventure, &adventure.hero)?;
        let stash = storage_entity(&self.rules, &adventure, &adventure.camp_storage)?;
        if !((from_owner == hero && to_owner == stash) || (from_owner == stash && to_owner == hero))
        {
            let hero_name = self
                .rules
                .character_template(&adventure.hero)
                .expect("compiled hero exists")
                .name
                .clone();
            let stash_name = self
                .rules
                .storage(&adventure.camp_storage)
                .expect("compiled camp storage exists")
                .name
                .clone();
            return Err(GameRuntimeError::InvalidContainment(format!(
                "loadout transfers are limited to {hero_name} and {stash_name}"
            )));
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
        let hero_name = self
            .rules
            .character_template(&adventure.hero)
            .expect("compiled hero exists")
            .name
            .clone();
        let destination = if to_owner == hero {
            format!("{hero_name}'s inventory")
        } else {
            "the camp stash".to_owned()
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
        let adventure = self.adventure()?.clone();
        let campaign = self
            .campaign
            .as_ref()
            .ok_or(GameRuntimeError::NoEncounter)?;
        let encounter = current_encounter_definition(&self.rules, &adventure, campaign)?;
        let hero = character_entity(&self.rules, &adventure, &adventure.hero)?;
        let opponent = character_entity(&self.rules, &adventure, &encounter.opponent)?;
        if actor != hero || target != opponent {
            return Err(GameRuntimeError::InvalidCommand(
                "the player action actor/target do not match the authored encounter".to_owned(),
            ));
        }
        let action = id(&request.action_id)?;
        let hero_definition = self
            .rules
            .character_template(&adventure.hero)
            .expect("compiled hero exists");
        if !hero_definition.actions.contains(&action) {
            return Err(GameRuntimeError::InvalidCommand(format!(
                "action {action} is not available to {}",
                hero_definition.name
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
        let adventure = self.adventure()?.clone();
        let encounter = current_encounter_definition(
            &self.rules,
            &adventure,
            self.campaign
                .as_ref()
                .ok_or(GameRuntimeError::NoEncounter)?,
        )?
        .clone();
        let expected_actor = match turn_owner {
            EncounterTurnOwner::Player => {
                character_entity(&self.rules, &adventure, &adventure.hero)?
            }
            EncounterTurnOwner::Opposition => {
                character_entity(&self.rules, &adventure, &encounter.opponent)?
            }
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
            let opponent = character_entity(&self.rules, &adventure, &encounter.opponent)?;
            let encounter_outcome = if receipt.target == opponent {
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
                            "{} scheduled effect(s) expired before {}'s next turn.",
                            turn_receipt.expired.len(),
                            self.rules
                                .character_template(&adventure.hero)
                                .expect("compiled hero exists")
                                .name
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
        let adventure = self.adventure()?.clone();
        let encounter = current_encounter_definition(
            &self.rules,
            &adventure,
            self.campaign
                .as_ref()
                .ok_or(GameRuntimeError::NoEncounter)?,
        )?
        .clone();
        let opponent = self
            .rules
            .character_template(&encounter.opponent)
            .expect("compiled opponent exists")
            .clone();
        let actions = opponent.actions.clone();
        let upper = u32::try_from(actions.len()).map_err(|_| {
            GameRuntimeError::InvalidState(
                "the opposition action catalog does not fit deterministic choice".to_owned(),
            )
        })?;
        let index = self
            .session()?
            .deterministic_choice_index(&format!("{}-{}-action", adventure.id, encounter.id), upper)
            .ok_or_else(|| {
                GameRuntimeError::InvalidState(
                    "the opposition has no admitted action choices".to_owned(),
                )
            })?;
        let index = usize::try_from(index).expect("u32 choice index fits usize");
        let action = actions[index].clone();
        let serial = self.next_operation;
        let operation = operation(&format!("opposition-action-{serial}"))?;
        let preview = self.session()?.preview_action(
            EntityId::new(opponent.entity_id),
            character_entity(&self.rules, &adventure, &adventure.hero)?,
            &action,
            operation,
        )?;
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
            &format!("{} prepares {}.", opponent.name, humanize(action.as_str())),
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
        let adventure = self.adventure()?.clone();
        let encounter = current_encounter_definition(
            &self.rules,
            &adventure,
            self.campaign
                .as_ref()
                .ok_or(GameRuntimeError::NoEncounter)?,
        )?
        .clone();
        let hero = self
            .rules
            .character_template(&adventure.hero)
            .expect("compiled hero exists")
            .clone();
        let mut details = Vec::new();
        if outcome == EncounterOutcome::Defeat {
            let recovery = encounter.defeat.recovery_vitality.ok_or_else(|| {
                GameRuntimeError::InvalidState(format!(
                    "encounter {} has no defeat recovery",
                    encounter.id
                ))
            })?;
            let serial = self.next_operation;
            let receipt = self.session_mut()?.restore_vitality(
                EntityId::new(hero.entity_id),
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
                hero.name,
                receipt.after.get(),
                hero.vitality
            ));
        } else {
            details.push(format!(
                "{} keeps remaining vitality and resources.",
                hero.name
            ));
            if let Some(reward) = encounter.victory.reward_label {
                details.push(format!("{reward} remains in canonical camp storage."));
            }
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
            campaign.phase = CampaignPhase::Outcome;
            campaign.resolved_encounter_id = campaign.active_encounter_id.clone();
            campaign.turn_owner = None;
            campaign.outcome = Some(outcome);
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
            Self::PendingActionCannotBeSaved | Self::InvalidCommand(_) | Self::D20Identity(_) => {
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
            Self::PendingActionCannotBeSaved => {
                write!(formatter, "resolve the pending action before saving")
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
mod tests;
