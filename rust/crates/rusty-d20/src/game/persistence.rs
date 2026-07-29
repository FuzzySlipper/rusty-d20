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
        let catalog = AuthoredAdventureCatalog::builtin().map_err(GameRuntimeError::Catalog)?;
        if envelope.schema_version == GAME_SAVE_SCHEMA_VERSION {
            let save: GameSave = serde_json::from_value(value)?;
            let adventure_id = D20Id::parse(&save.campaign.adventure_id).map_err(|error| {
                GameRuntimeError::InvalidSave(format!(
                    "saved adventure identity is invalid: {error}"
                ))
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
            return Self::restore(
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
                None,
                false,
            );
        }
        let adventure_id = catalog.default_adventure().clone();
        let rules = catalog
            .rules_for(&adventure_id)
            .map_err(GameRuntimeError::Catalog)?;
        let legacy_fingerprint = catalog
            .rules_for_package("steel-guard")
            .map_err(GameRuntimeError::Catalog)?
            .fingerprint()
            .to_owned();
        let legacy_encounter_id = rules
            .adventure(&adventure_id)
            .and_then(|adventure| adventure.encounters.first())
            .ok_or_else(|| {
                GameRuntimeError::Catalog(
                    "default adventure does not define a legacy encounter".to_owned(),
                )
            })?
            .to_string();
        match envelope.schema_version {
            1 => {
                let save: LegacyGameSave = serde_json::from_value(value)?;
                Self::restore(
                    rules,
                    adventure_id,
                    RestoreData {
                        revision: save.revision,
                        next_operation: save.next_operation,
                        next_log_id: save.next_log_id,
                        log: save.log,
                        session: save.session,
                    },
                    CampaignState {
                        phase: CampaignPhase::Encounter,
                        active_encounter_id: Some(legacy_encounter_id),
                        resolved_encounter_id: None,
                        turn_owner: Some(EncounterTurnOwner::Player),
                        outcome: None,
                    },
                    Some(&legacy_fingerprint),
                    true,
                )
            }
            2 | 3 => {
                let save: LegacyCampaignGameSave = serde_json::from_value(value)?;
                let campaign = validate_legacy_campaign_save(&rules, &adventure_id, save.campaign)?;
                Self::restore(
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
                    Some(&legacy_fingerprint),
                    true,
                )
            }
            4 => {
                let save: SchemaFourGameSave = serde_json::from_value(value)?;
                let campaign =
                    validate_schema_four_campaign_save(&rules, &adventure_id, save.campaign)?;
                Self::restore(
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
                    Some(&legacy_fingerprint),
                    false,
                )
            }
            actual => Err(GameRuntimeError::UnsupportedSaveSchema { actual }),
        }
    }

    fn restore(
        rules: D20Ruleset,
        adventure_id: D20Id,
        mut data: RestoreData,
        campaign: CampaignState,
        legacy_session_fingerprint: Option<&str>,
        migrate_campaign: bool,
    ) -> Result<Self, GameRuntimeError> {
        let mut campaign = campaign;
        if let Some(legacy_fingerprint) = legacy_session_fingerprint {
            migrate_session_fingerprint(
                &mut data.session,
                legacy_fingerprint,
                rules.fingerprint(),
            )?;
        }
        let session_json = serde_json::to_string(&data.session)?;
        let mut session = D20Session::decode_save(rules.clone(), &session_json)?;
        let adventure = rules
            .adventure(&adventure_id)
            .ok_or_else(|| {
                GameRuntimeError::InvalidSave(format!(
                    "compiled adventure {adventure_id} is missing"
                ))
            })?
            .clone();
        let hero = character_entity(&rules, &adventure, &adventure.hero)?;
        if legacy_session_fingerprint.is_some()
            && session
                .entities()
                .component::<InventoryComponent>(hero)?
                .is_none()
        {
            install_product_loadout(&rules, &adventure, &mut session)?;
        }
        if migrate_campaign {
            campaign = migrate_legacy_campaign(
                &rules,
                &adventure,
                &mut session,
                campaign,
                &mut data.next_operation,
            )?;
        }
        validate_product_state(&rules, &adventure, &session, &campaign)?;
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
    resolved_encounter_id: Option<String>,
    turn_owner: Option<EncounterTurnOwner>,
    outcome: Option<EncounterOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SchemaFourCampaignSave {
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
    composition_fingerprint: String,
    revision: u64,
    next_operation: u64,
    next_log_id: u64,
    log: Vec<GameLogEntryDto>,
    campaign: CampaignSave,
    session: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SchemaFourGameSave {
    #[serde(rename = "schemaVersion")]
    _schema_version: u32,
    revision: u64,
    next_operation: u64,
    next_log_id: u64,
    log: Vec<GameLogEntryDto>,
    campaign: SchemaFourCampaignSave,
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
    let valid_phase = match save.phase {
        CampaignPhase::Camp => {
            save.active_encounter_id.is_none()
                && save.turn_owner.is_none()
                && match save.outcome {
                    None => save.resolved_encounter_id.is_none(),
                    Some(_) => authored_encounter(&save.resolved_encounter_id),
                }
        }
        CampaignPhase::Encounter => {
            authored_encounter(&save.active_encounter_id)
                && save.resolved_encounter_id.is_none()
                && save.turn_owner.is_some()
                && save.outcome.is_none()
        }
        CampaignPhase::Outcome => {
            authored_encounter(&save.active_encounter_id)
                && save.active_encounter_id == save.resolved_encounter_id
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
        resolved_encounter_id: save.resolved_encounter_id,
        turn_owner: save.turn_owner,
        outcome: save.outcome,
    })
}

fn validate_schema_four_campaign_save(
    rules: &D20Ruleset,
    adventure_id: &D20Id,
    save: SchemaFourCampaignSave,
) -> Result<CampaignState, GameRuntimeError> {
    let adventure = rules.adventure(adventure_id).ok_or_else(|| {
        GameRuntimeError::InvalidSave(format!("compiled adventure {adventure_id} is missing"))
    })?;
    let resolved_encounter_id = if save.outcome.is_some() {
        save.active_encounter_id
            .clone()
            .or_else(|| adventure.encounters.first().map(ToString::to_string))
    } else {
        None
    };
    validate_campaign_save(
        rules,
        adventure_id,
        CampaignSave {
            adventure_id: save.adventure_id,
            phase: save.phase,
            active_encounter_id: save.active_encounter_id,
            resolved_encounter_id,
            turn_owner: save.turn_owner,
            outcome: save.outcome,
        },
    )
}

fn validate_legacy_campaign_save(
    rules: &D20Ruleset,
    adventure_id: &D20Id,
    save: LegacyCampaignSave,
) -> Result<CampaignState, GameRuntimeError> {
    let phase = match save.phase {
        LegacyCampaignPhase::Camp => CampaignPhase::Camp,
        LegacyCampaignPhase::Encounter => CampaignPhase::Encounter,
    };
    validate_campaign_save(
        rules,
        adventure_id,
        CampaignSave {
            adventure_id: save.adventure_id,
            phase,
            active_encounter_id: save.active_encounter_id,
            resolved_encounter_id: None,
            turn_owner: (phase == CampaignPhase::Encounter).then_some(EncounterTurnOwner::Player),
            outcome: None,
        },
    )
}

fn migrate_session_fingerprint(
    session: &mut serde_json::Value,
    legacy: &str,
    current: &str,
) -> Result<(), GameRuntimeError> {
    let fingerprint = session
        .as_object_mut()
        .and_then(|session| session.get_mut("rulesetFingerprint"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            GameRuntimeError::InvalidSave(
                "legacy session ruleset fingerprint is missing".to_owned(),
            )
        })?;
    if fingerprint != legacy {
        return Err(GameRuntimeError::CompositionFingerprintMismatch {
            expected: legacy.to_owned(),
            actual: fingerprint.to_owned(),
        });
    }
    session["rulesetFingerprint"] = serde_json::Value::String(current.to_owned());
    Ok(())
}
