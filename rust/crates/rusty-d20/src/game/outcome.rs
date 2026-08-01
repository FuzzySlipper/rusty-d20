use super::*;

impl GameRuntime {
    pub fn return_to_camp(
        &mut self,
        expected_revision: u64,
    ) -> Result<GameSnapshotDto, GameRuntimeError> {
        let mut staged = self.clone();
        let snapshot = staged.return_to_camp_inner(expected_revision)?;
        *self = staged;
        Ok(snapshot)
    }

    pub(super) fn return_to_camp_inner(
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

    pub(super) fn complete_encounter(
        &mut self,
        outcome: EncounterOutcome,
    ) -> Result<(), GameRuntimeError> {
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
}
