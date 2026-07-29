use super::*;

impl GameRuntime {
    pub fn decode_save(input: &str) -> Result<Self, GameRuntimeError> {
        if input.len() > MAX_GAME_SAVE_BYTES {
            return Err(GameRuntimeError::InvalidSave(format!(
                "save contains {} bytes; maximum is {MAX_GAME_SAVE_BYTES}",
                input.len()
            )));
        }
        let value: serde_json::Value = serde_json::from_str(input)?;
        let envelope: SaveEnvelope = serde_json::from_value(value.clone())?;
        if envelope.schema_version != GAME_SAVE_SCHEMA_VERSION {
            return Err(GameRuntimeError::UnsupportedSaveSchema {
                actual: envelope.schema_version,
            });
        }
        let save: GameSave = serde_json::from_value(value)?;
        let catalog = AuthoredAdventureCatalog::builtin().map_err(GameRuntimeError::Catalog)?;
        let adventure_id = D20Id::parse(&save.campaign.adventure_id).map_err(|error| {
            GameRuntimeError::InvalidSave(format!("saved adventure identity is invalid: {error}"))
        })?;
        let rules = catalog
            .rules_for(&adventure_id)
            .map_err(GameRuntimeError::InvalidSave)?;
        if save.composition_fingerprint != rules.fingerprint() {
            return Err(GameRuntimeError::CompositionFingerprintMismatch {
                expected: rules.fingerprint().to_owned(),
                actual: save.composition_fingerprint,
            });
        }
        let campaign = validate_campaign_save(&rules, &adventure_id, save.campaign)?;
        Self::restore(
            catalog,
            rules,
            adventure_id,
            RestoreData {
                revision: save.revision,
                next_operation: save.next_operation,
                next_log_id: save.next_log_id,
                log: save.log,
                session: save.session,
            },
            campaign,
        )
    }

    fn restore(
        catalog: AuthoredAdventureCatalog,
        rules: D20Ruleset,
        adventure_id: D20Id,
        data: RestoreData,
        campaign: CampaignState,
    ) -> Result<Self, GameRuntimeError> {
        let session_json = serde_json::to_string(&data.session)?;
        let session = D20Session::decode_save(rules.clone(), &session_json)?;
        let adventure = rules
            .adventure(&adventure_id)
            .ok_or_else(|| {
                GameRuntimeError::InvalidSave(format!(
                    "compiled adventure {adventure_id} is missing"
                ))
            })?
            .clone();
        validate_product_state(&rules, &adventure, &session, &campaign)?;
        validate_log_state(&data)?;
        let runtime = Self {
            catalog,
            rules,
            adventure_id,
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
            composition_fingerprint: self.rules.fingerprint().to_owned(),
            revision: self.revision,
            next_operation: self.next_operation,
            next_log_id: self.next_log_id,
            log: self.log.clone(),
            campaign: CampaignSave {
                adventure_id: self.adventure_id.to_string(),
                phase: campaign.phase,
                active_encounter_id: campaign.active_encounter_id.clone(),
                resolved_encounter_id: campaign.resolved_encounter_id.clone(),
                current_actor_id: campaign.current_actor_id,
                outcome: campaign.outcome,
                completed_encounters: campaign.completed_encounters.clone(),
                exploration: campaign.exploration.clone(),
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
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveEnvelope {
    schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CampaignSave {
    adventure_id: String,
    phase: CampaignPhase,
    active_encounter_id: Option<String>,
    resolved_encounter_id: Option<String>,
    current_actor_id: Option<u64>,
    outcome: Option<EncounterOutcome>,
    completed_encounters: Vec<CompletedEncounter>,
    exploration: Option<ExplorationState>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct GameSave {
    schema_version: u32,
    composition_fingerprint: String,
    revision: u64,
    next_operation: u64,
    next_log_id: u64,
    log: Vec<GameLogEntryDto>,
    campaign: CampaignSave,
    session: serde_json::Value,
}

fn validate_campaign_save(
    rules: &D20Ruleset,
    adventure_id: &D20Id,
    save: CampaignSave,
) -> Result<CampaignState, GameRuntimeError> {
    if save.adventure_id != adventure_id.as_str() {
        return Err(GameRuntimeError::InvalidSave(format!(
            "unknown adventure {}",
            save.adventure_id
        )));
    }
    let adventure = rules.adventure(adventure_id).ok_or_else(|| {
        GameRuntimeError::InvalidSave(format!("compiled adventure {adventure_id} is missing"))
    })?;
    let authored_encounter = |value: &Option<String>| {
        value.as_deref().is_some_and(|encounter| {
            adventure
                .encounters
                .iter()
                .any(|candidate| candidate.as_str() == encounter)
        })
    };
    if save.completed_encounters.len() > adventure.encounters.len()
        || save
            .completed_encounters
            .iter()
            .zip(adventure.encounters.iter())
            .any(|(completed, expected)| completed.encounter_id != expected.as_str())
    {
        return Err(GameRuntimeError::InvalidSave(
            "completed encounters are not the authored campaign prefix".to_owned(),
        ));
    }
    let latest_completed = save.completed_encounters.last();
    if let Some(exploration) = save.exploration.as_ref() {
        validate_exploration_state(adventure, exploration)?;
    }
    let valid_phase = match save.phase {
        CampaignPhase::Camp => {
            save.active_encounter_id.is_none()
                && save.current_actor_id.is_none()
                && match (save.outcome, save.resolved_encounter_id.as_deref()) {
                    (None, None) => save.completed_encounters.is_empty(),
                    (Some(outcome), Some(resolved)) => latest_completed.is_some_and(|completed| {
                        completed.encounter_id == resolved && completed.outcome == outcome
                    }),
                    _ => false,
                }
        }
        CampaignPhase::Exploration => {
            save.exploration.is_some()
                && save.active_encounter_id.is_none()
                && save.current_actor_id.is_none()
                && match (save.outcome, save.resolved_encounter_id.as_deref()) {
                    (None, None) => save.completed_encounters.is_empty(),
                    (Some(EncounterOutcome::Victory), Some(resolved)) => latest_completed
                        .is_some_and(|completed| {
                            completed.encounter_id == resolved
                                && completed.outcome == EncounterOutcome::Victory
                        }),
                    _ => false,
                }
        }
        CampaignPhase::Encounter => {
            authored_encounter(&save.active_encounter_id)
                && save.resolved_encounter_id.is_none()
                && save.current_actor_id.is_some_and(|actor| actor != 0)
                && save.outcome.is_none()
                && adventure
                    .encounters
                    .get(save.completed_encounters.len())
                    .is_some_and(|expected| {
                        save.active_encounter_id.as_deref() == Some(expected.as_str())
                    })
                && save.exploration.as_ref().is_none_or(|exploration| {
                    adventure.dungeon.encounters.iter().any(|trigger| {
                        Some(trigger.encounter.as_str()) == save.active_encounter_id.as_deref()
                            && trigger.x == exploration.position.x
                            && trigger.y == exploration.position.y
                    })
                })
        }
        CampaignPhase::Outcome => {
            authored_encounter(&save.active_encounter_id)
                && save.active_encounter_id == save.resolved_encounter_id
                && save.current_actor_id.is_none()
                && save.outcome.is_some_and(|outcome| {
                    latest_completed.is_some_and(|completed| {
                        Some(completed.encounter_id.as_str()) == save.active_encounter_id.as_deref()
                            && completed.outcome == outcome
                    })
                })
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
        resolved_encounter_id: save.resolved_encounter_id,
        current_actor_id: save.current_actor_id,
        outcome: save.outcome,
        completed_encounters: save.completed_encounters,
        exploration: save.exploration,
    })
}

fn validate_exploration_state(
    adventure: &AdventureDefinition,
    exploration: &ExplorationState,
) -> Result<(), GameRuntimeError> {
    let dungeon = &adventure.dungeon;
    if !dungeon.is_floor(exploration.position.x, exploration.position.y)
        || !exploration.discovered.contains(&exploration.position)
        || exploration.discovered.is_empty()
        || exploration.discovered.len() > usize::from(dungeon.width) * usize::from(dungeon.height)
        || exploration
            .discovered
            .iter()
            .any(|position| !dungeon.is_floor(position.x, position.y))
        || exploration.inspected_landmarks.iter().any(|id| {
            !dungeon
                .landmarks
                .iter()
                .any(|landmark| landmark.id.as_str() == id)
        })
    {
        return Err(GameRuntimeError::InvalidSave(
            "exploration position, discoveries, or inspected landmarks are invalid".to_owned(),
        ));
    }
    Ok(())
}

fn validate_log_state(data: &RestoreData) -> Result<(), GameRuntimeError> {
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
    Ok(())
}
