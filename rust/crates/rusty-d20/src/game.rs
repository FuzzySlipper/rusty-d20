use core_ids::EntityId;
use entity_state::ComponentAccessError;
use gameplay_mechanics::{
    ActiveEffectsComponent, DecisionOutcome, EffectInstanceId, OperationId, ResponseDecisionKind,
    SourceInstanceIdentity, StatContribution, TracksComponent,
};
use gameplay_rules::AdmittedRulePackage;
use serde::{Deserialize, Serialize};
use svc_rng::RngSeed;
use ts_rs::TS;

use crate::{
    AbilityScore, ActionPreview, ActionResource, ActionResourcesComponent, AffinitySeed,
    ApplyActionRequest, ArmorItemSeed, CharacterSeed, D20CompileError, D20Id, D20Ruleset,
    D20Session, D20SessionError, DamageAffinity, ReactionReceipt, ScheduledEffectsComponent,
    SessionSaveError, ENGINE_REVISION,
};

const GAME_SAVE_SCHEMA_VERSION: u32 = 1;
const PLAYER: EntityId = EntityId::new(101);
const OPPONENT: EntityId = EntityId::new(102);
const OPPONENT_ARMOR: EntityId = EntityId::new(201);
const MAX_LOG_ENTRIES: usize = 64;
const MAX_LOG_DETAILS: usize = 32;
const MAX_LOG_SOURCE_BYTES: usize = 128;
const MAX_LOG_TEXT_BYTES: usize = 512;
const MAX_LOG_DETAIL_BYTES: usize = 512;
const MAX_GAME_SAVE_BYTES: usize = 1_000_000;
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
    pub characters: Vec<CharacterDto>,
    pub actions: Vec<ActionDto>,
    pub pending_action: Option<PendingActionDto>,
    pub log: Vec<GameLogEntryDto>,
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
pub struct GameRuntime {
    rules: D20Ruleset,
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
        let save: GameSave = serde_json::from_str(input)?;
        if save.schema_version != GAME_SAVE_SCHEMA_VERSION {
            return Err(GameRuntimeError::UnsupportedSaveSchema {
                actual: save.schema_version,
            });
        }
        let rules = starter_ruleset()?;
        let session_json = serde_json::to_string(&save.session)?;
        let session = D20Session::decode_save(rules.clone(), &session_json)?;
        if save.next_operation == 0 || save.next_log_id == 0 || save.log.len() > MAX_LOG_ENTRIES {
            return Err(GameRuntimeError::InvalidSave(
                "operation/log counters or bounded log are invalid".to_owned(),
            ));
        }
        if save.log.windows(2).any(|pair| pair[0].id >= pair[1].id) {
            return Err(GameRuntimeError::InvalidSave(
                "log identities are not in strict order".to_owned(),
            ));
        }
        if save.log.iter().any(|entry| {
            entry.id == 0
                || entry.source.len() > MAX_LOG_SOURCE_BYTES
                || entry.text.len() > MAX_LOG_TEXT_BYTES
                || entry.details.len() > MAX_LOG_DETAILS
                || entry
                    .details
                    .iter()
                    .any(|detail| detail.len() > MAX_LOG_DETAIL_BYTES)
        }) || save
            .log
            .last()
            .is_some_and(|entry| save.next_log_id <= entry.id)
        {
            return Err(GameRuntimeError::InvalidSave(
                "log entry bounds or next identity are invalid".to_owned(),
            ));
        }
        Ok(Self {
            rules,
            session: Some(session),
            revision: save.revision,
            saved_revision: Some(save.revision),
            next_operation: save.next_operation,
            next_log_id: save.next_log_id,
            pending: None,
            log: save.log,
        })
    }

    pub fn encode_save(&self) -> Result<String, GameRuntimeError> {
        let session = self.session.as_ref().ok_or(GameRuntimeError::NoEncounter)?;
        let session = serde_json::from_str(&session.encode_save()?)?;
        Ok(serde_json::to_string_pretty(&GameSave {
            schema_version: GAME_SAVE_SCHEMA_VERSION,
            revision: self.revision,
            next_operation: self.next_operation,
            next_log_id: self.next_log_id,
            log: self.log.clone(),
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
        Ok(GameSnapshotDto {
            product: "Rusty D20".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            engine_revision: ENGINE_REVISION.to_owned(),
            ruleset_fingerprint: self.rules.fingerprint().to_owned(),
            revision: self.revision,
            saved: self.saved_revision == Some(self.revision),
            encounter: self
                .session
                .as_ref()
                .map(|session| self.project_encounter(session))
                .transpose()?,
        })
    }

    pub fn start_encounter(
        &mut self,
        expected_revision: u64,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(expected_revision)?;
        self.ensure_mutation_capacity(false, false)?;
        let mut session = D20Session::new(
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
            vec![ArmorItemSeed {
                entity: OPPONENT_ARMOR,
                owner: OPPONENT,
                name: "Warden chain armor".to_owned(),
                armor: id("chain-armor")?,
            }],
        )?;
        session.equip_armor(
            OPPONENT,
            OPPONENT_ARMOR,
            &id("chain-armor")?,
            operation("equip-warden-chain")?,
        )?;
        self.session = Some(session);
        self.pending = None;
        self.log.clear();
        self.next_log_id = 1;
        self.next_operation = 1;
        self.bump_revision()?;
        self.saved_revision = None;
        self.push_log(
            GameLogKindDto::System,
            "Encounter",
            "Mara Venn faces the Iron Warden.",
            vec![
                "Starter Core + Steel Guard authored packages compiled by Rust.".to_owned(),
                "Iron Warden's chain armor and slashing resistance are active sources.".to_owned(),
            ],
        )?;
        self.snapshot()
    }

    pub fn preview_action(
        &mut self,
        request: PreviewActionRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_mutation_capacity(true, false)?;
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
        self.ensure_revision(request.expected_revision)?;
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
        self.ensure_revision(request.expected_revision)?;
        self.ensure_mutation_capacity(false, true)?;
        let pending = self.require_pending(&request.preview_token)?.clone();
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
        self.bump_revision()?;
        self.saved_revision = None;

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
        self.push_log(
            kind,
            &humanize(receipt.action.as_str()),
            &format!("Mara Venn {outcome} the Iron Warden."),
            details,
        )?;
        self.snapshot()
    }

    pub fn advance_turn(
        &mut self,
        expected_revision: u64,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(expected_revision)?;
        self.ensure_mutation_capacity(true, true)?;
        let serial = self.next_operation;
        let next_turn = self
            .session()?
            .current_turn()
            .checked_add(1)
            .ok_or(GameRuntimeError::CounterOverflow)?;
        let receipt = self
            .session_mut()?
            .advance_turn(next_turn, operation(&format!("advance-turn-{serial}"))?)?;
        self.next_operation = self
            .next_operation
            .checked_add(1)
            .ok_or(GameRuntimeError::CounterOverflow)?;
        self.pending = None;
        self.bump_revision()?;
        self.saved_revision = None;
        self.push_log(
            GameLogKindDto::Turn,
            "Turn",
            &format!(
                "Advanced from turn {} to {}.",
                receipt.before, receipt.after
            ),
            vec![format!(
                "{} scheduled effect(s) expired.",
                receipt.expired.len()
            )],
        )?;
        self.snapshot()
    }

    fn project_encounter(&self, session: &D20Session) -> Result<EncounterDto, GameRuntimeError> {
        Ok(EncounterDto {
            turn: session.current_turn(),
            next_roll: session.next_roll_index(),
            player_id: PLAYER.raw(),
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
            health_maximum: 100,
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
        self.push_log(
            GameLogKindDto::Reaction,
            &humanize(receipt.reaction.as_str()),
            "Iron Warden raised a reaction before the roll.",
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct GameSave {
    schema_version: u32,
    revision: u64,
    next_operation: u64,
    next_log_id: u64,
    log: Vec<GameLogEntryDto>,
    session: serde_json::Value,
}

#[derive(Debug)]
pub enum GameRuntimeError {
    NoEncounter,
    StaleCommand(String),
    InvalidCommand(String),
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
            Self::InvalidCommand(_) | Self::D20Identity(_) => (ApiErrorKindDto::Invalid, false),
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
            Self::StaleCommand(message)
            | Self::InvalidCommand(message)
            | Self::InvalidState(message)
            | Self::InvalidSave(message) => formatter.write_str(message),
            _ => write!(formatter, "Rusty D20 product operation failed: {self:?}"),
        }
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
        vitality: 100,
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

    #[test]
    fn product_runtime_is_atomic_stale_safe_and_reopens_deterministically() {
        let mut runtime = GameRuntime::empty().unwrap();
        assert!(runtime.snapshot().unwrap().encounter.is_none());
        let started = runtime.start_encounter(0).unwrap();
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
        assert_eq!(reopened.encode_save().unwrap(), encoded);
        let reopened_snapshot = reopened.snapshot().unwrap();
        assert!(reopened_snapshot
            .encounter
            .unwrap()
            .pending_action
            .is_none());
        let advanced = reopened.advance_turn(reopened_snapshot.revision).unwrap();
        assert_eq!(advanced.encounter.unwrap().turn, 1);
    }

    #[test]
    fn saturated_product_counters_and_oversized_saves_fail_before_mutation() {
        let mut runtime = GameRuntime::empty().unwrap();
        let started = runtime.start_encounter(0).unwrap();
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
