use super::*;

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

    pub(super) fn empty_with_rules(
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

    pub(super) fn move_loadout_item_inner(
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
}
