use super::*;

impl GameRuntime {
    pub(super) fn push_log(
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

    pub(super) fn session(&self) -> Result<&D20Session, GameRuntimeError> {
        self.session.as_ref().ok_or(GameRuntimeError::NoEncounter)
    }

    pub(super) fn adventure(&self) -> Result<&AdventureDefinition, GameRuntimeError> {
        self.rules.adventure(&self.adventure_id).ok_or_else(|| {
            GameRuntimeError::InvalidState(format!(
                "compiled adventure {} is missing",
                self.adventure_id
            ))
        })
    }

    pub(super) fn session_mut(&mut self) -> Result<&mut D20Session, GameRuntimeError> {
        self.session.as_mut().ok_or(GameRuntimeError::NoEncounter)
    }

    pub(super) fn campaign_mut(&mut self) -> Result<&mut CampaignState, GameRuntimeError> {
        self.campaign.as_mut().ok_or(GameRuntimeError::NoEncounter)
    }

    pub(super) fn character_name(&self, entity: EntityId) -> Result<String, GameRuntimeError> {
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

    pub(super) fn vitality(&self, entity: EntityId) -> Result<i64, GameRuntimeError> {
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

    pub(super) fn require_pending(&self, token: &str) -> Result<&PendingAction, GameRuntimeError> {
        self.pending
            .as_ref()
            .filter(|pending| pending.token == token)
            .ok_or_else(|| {
                GameRuntimeError::StaleCommand(
                    "the selected action preview is no longer current".to_owned(),
                )
            })
    }

    pub(super) fn ensure_encounter_phase(&self) -> Result<(), GameRuntimeError> {
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

    pub(super) fn ensure_camp_phase(&self) -> Result<(), GameRuntimeError> {
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

    pub(super) fn ensure_outcome_phase(&self) -> Result<(), GameRuntimeError> {
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

    pub(super) fn ensure_current_faction(
        &self,
        expected: EncounterFaction,
    ) -> Result<(), GameRuntimeError> {
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

    pub(super) fn ensure_revision(&self, expected: u64) -> Result<(), GameRuntimeError> {
        if expected != self.revision {
            return Err(GameRuntimeError::StaleCommand(format!(
                "expected revision {expected}, current revision is {}",
                self.revision
            )));
        }
        Ok(())
    }

    pub(super) fn ensure_mutation_capacity(
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

    pub(super) fn bump_revision(&mut self) -> Result<(), GameRuntimeError> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(GameRuntimeError::CounterOverflow)?;
        Ok(())
    }
}

pub(super) fn target_team_allows(
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

pub(super) fn entity(raw: u64) -> Result<EntityId, GameRuntimeError> {
    if raw == 0 {
        return Err(GameRuntimeError::InvalidCommand(
            "entity identity must be nonzero".to_owned(),
        ));
    }
    Ok(EntityId::new(raw))
}

pub(super) fn party_member_name(
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

pub(super) fn id(value: &str) -> Result<D20Id, GameRuntimeError> {
    Ok(D20Id::parse(value)?)
}

pub(super) fn operation(value: &str) -> Result<OperationId, GameRuntimeError> {
    OperationId::parse(value).map_err(|error| GameRuntimeError::InvalidCommand(error.to_string()))
}

pub(super) fn effect_instance(value: &str) -> Result<EffectInstanceId, GameRuntimeError> {
    EffectInstanceId::parse(value)
        .map_err(|error| GameRuntimeError::InvalidCommand(error.to_string()))
}

pub(super) fn humanize(value: &str) -> String {
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

pub(super) fn stat_contribution_label(contribution: Option<&StatContribution>) -> String {
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

pub(super) fn damage_decision_label(decision: &ResponseDecisionKind) -> String {
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

pub(super) const fn outcome_label(outcome: DecisionOutcome) -> &'static str {
    match outcome {
        DecisionOutcome::Applied => "applied",
        DecisionOutcome::Suppressed => "suppressed",
        DecisionOutcome::Inapplicable => "inapplicable",
    }
}

pub(super) fn source_label(source: &SourceInstanceIdentity) -> String {
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

pub(super) fn is_unavailable_action_error(error: &D20SessionError) -> bool {
    matches!(
        error,
        D20SessionError::ActionForbidden { .. }
            | D20SessionError::RequiredImplementNotEquipped { .. }
            | D20SessionError::ActivationBudgetUnavailable { .. }
    )
}
