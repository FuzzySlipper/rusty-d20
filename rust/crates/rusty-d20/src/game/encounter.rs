use super::*;

impl GameRuntime {
    pub fn enter_encounter(
        &mut self,
        request: EnterEncounterRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        let snapshot = staged.enter_encounter_inner(request)?;
        *self = staged;
        Ok(snapshot)
    }

    pub(super) fn enter_encounter_inner(
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

    pub fn choose_action(
        &mut self,
        request: ChooseActionRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        let snapshot = staged.choose_action_inner(request)?;
        *self = staged;
        Ok(snapshot)
    }

    pub(super) fn choose_action_inner(
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

    pub(super) fn move_actor_inner(
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

    pub(super) fn apply_reaction_inner(
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

    pub(super) fn decline_reaction_inner(
        &mut self,
        request: DeclineReactionRequestDto,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.ensure_revision(request.expected_revision)?;
        self.ensure_encounter_phase()?;
        self.ensure_mutation_capacity(false, true)?;
        let pending = self.require_pending(&request.prompt_token)?.clone();
        self.resolve_pending_action(pending)
    }

    pub(super) fn resolve_pending_action(
        &mut self,
        pending: PendingAction,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        self.resolve_pending_action_once(pending)?;
        self.settle_automatic_opposition()?;
        self.snapshot()
    }

    pub(super) fn resolve_pending_action_once(
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

    pub(super) fn advance_opposition_activation(&mut self) -> Result<(), GameRuntimeError> {
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

    pub(super) fn settle_automatic_opposition(&mut self) -> Result<(), GameRuntimeError> {
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

    pub(super) fn log_reaction(
        &mut self,
        receipt: &ReactionReceipt,
    ) -> Result<(), GameRuntimeError> {
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

    pub(super) fn ordered_participants(
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

    pub(super) fn living_participants(
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

    pub(super) fn participant_position(
        &self,
        entity: EntityId,
    ) -> Result<TacticalPosition, GameRuntimeError> {
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

    pub(super) fn occupied_positions(
        &self,
        excluded: Option<EntityId>,
    ) -> Result<BTreeSet<TacticalPosition>, GameRuntimeError> {
        self.ordered_participants()?
            .into_iter()
            .filter(|(entity, _, _)| Some(*entity) != excluded)
            .map(|(entity, _, _)| self.participant_position(entity))
            .collect()
    }

    pub(super) fn tactical_board(&self) -> Result<&TacticalBoardDefinition, GameRuntimeError> {
        Ok(&current_encounter_definition(
            &self.rules,
            self.adventure()?,
            self.campaign
                .as_ref()
                .ok_or(GameRuntimeError::NoEncounter)?,
        )?
        .board)
    }

    pub(super) fn action_range(&self, action: &ActionDefinition) -> Result<u16, GameRuntimeError> {
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

    pub(super) fn action_is_spatially_legal(
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

    pub(super) fn action_target_team_is_legal(
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

    pub(super) fn legal_tactical_routes(
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

    pub(super) fn legal_action_previews(
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

    pub(super) fn opposition_movement_route(
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

    pub(super) fn current_actor(&self) -> Result<(EntityId, EncounterFaction), GameRuntimeError> {
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

    pub(super) fn encounter_outcome(&self) -> Result<Option<EncounterOutcome>, GameRuntimeError> {
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

    pub(super) fn advance_activation(
        &mut self,
        mut details: Vec<String>,
    ) -> Result<(), GameRuntimeError> {
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
}
