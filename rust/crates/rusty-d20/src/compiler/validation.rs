use std::collections::{BTreeMap, BTreeSet};

use gameplay_rules::RulePackageIdentity;

use crate::*;

use super::collector::subject;
use super::*;

impl DefinitionCollector {
    pub(super) fn validate_references(&mut self) {
        if self.abilities.is_empty()
            || self.defenses.is_empty()
            || self.activation_budgets.is_empty()
            || self.damage_types.is_empty()
            || self.actions.is_empty()
        {
            self.push_global(
                "D20_INCOMPLETE_RULESET",
                "$/payload",
                "the resolved ruleset requires at least one ability, defense, activation budget, damage type, and action",
            );
        }

        for (id, (definition, package_id)) in self.defenses.clone() {
            for ability in &definition.abilities {
                if !self.abilities.contains_key(ability) {
                    self.push_for_identity(
                        &package_id,
                        Some(&subject("defense", &id)),
                        "D20_UNKNOWN_ABILITY",
                        format!("$/payload/defenses/{id}/abilities"),
                        format!("unknown ability {ability}"),
                    );
                }
            }
        }
        for (id, (definition, package_id)) in self.armors.clone() {
            if !self.defenses.contains_key(&definition.defense) {
                self.push_for_identity(
                    &package_id,
                    Some(&subject("armor", &id)),
                    "D20_UNKNOWN_DEFENSE",
                    format!("$/payload/armors/{id}/defense"),
                    format!("unknown defense {}", definition.defense),
                );
            }
        }
        for (id, (definition, package_id)) in self.implements.clone() {
            let correlation = subject("implement", &id);
            for (known, code, path, value) in [
                (
                    self.abilities.contains_key(&definition.ability),
                    "D20_UNKNOWN_ABILITY",
                    "ability",
                    definition.ability.to_string(),
                ),
                (
                    self.defenses.contains_key(&definition.defense),
                    "D20_UNKNOWN_DEFENSE",
                    "defense",
                    definition.defense.to_string(),
                ),
                (
                    self.damage_types.contains_key(&definition.damage.kind),
                    "D20_UNKNOWN_DAMAGE_TYPE",
                    "damage/kind",
                    definition.damage.kind.to_string(),
                ),
            ] {
                if !known {
                    self.push_for_identity(
                        &package_id,
                        Some(&correlation),
                        code,
                        format!("$/payload/implements/{id}/{path}"),
                        format!("unknown reference {value}"),
                    );
                }
            }
        }
        for (id, (definition, package_id)) in self.effects.clone() {
            if definition
                .defense
                .as_ref()
                .is_some_and(|defense| !self.defenses.contains_key(defense))
            {
                self.push_for_identity(
                    &package_id,
                    Some(&subject("effect", &id)),
                    "D20_UNKNOWN_DEFENSE",
                    format!("$/payload/effects/{id}/defense"),
                    "effect references an unknown defense".to_owned(),
                );
            }
        }
        for (id, (definition, package_id)) in self.reactions.clone() {
            let correlation = subject("reaction", &id);
            if !self.defenses.contains_key(&definition.defense) {
                self.push_for_identity(
                    &package_id,
                    Some(&correlation),
                    "D20_UNKNOWN_DEFENSE",
                    format!("$/payload/reactions/{id}/defense"),
                    format!("unknown defense {}", definition.defense),
                );
            }
            for cost in &definition.activation_costs {
                let Some(budget) = self.activation_budgets.get(&cost.budget) else {
                    self.push_for_identity(
                        &package_id,
                        Some(&correlation),
                        "D20_UNKNOWN_ACTIVATION_BUDGET",
                        format!("$/payload/reactions/{id}/activationCosts"),
                        format!("unknown activation budget {}", cost.budget),
                    );
                    continue;
                };
                if budget.0.timing != ActivationTimingDefinition::Reaction
                    || cost.amount > budget.0.initial
                {
                    self.push_for_identity(
                        &package_id,
                        Some(&correlation),
                        "D20_INCOMPATIBLE_ACTIVATION_COST",
                        format!("$/payload/reactions/{id}/activationCosts"),
                        format!(
                            "reaction activation cost {} must use a reaction budget and not exceed {} initial amount {}",
                            cost.amount, cost.budget, budget.0.initial
                        ),
                    );
                }
            }
            let Some(resource) = self.resources.get(&definition.resource) else {
                self.push_for_identity(
                    &package_id,
                    Some(&correlation),
                    "D20_UNKNOWN_RESOURCE",
                    format!("$/payload/reactions/{id}/resource"),
                    format!("unknown resource {}", definition.resource),
                );
                continue;
            };
            if definition.cost > resource.0.maximum {
                self.push_for_identity(
                    &package_id,
                    Some(&correlation),
                    "D20_INCOMPATIBLE_REACTION_COST",
                    format!("$/payload/reactions/{id}/cost"),
                    format!(
                        "reaction cost {} exceeds resource maximum {}",
                        definition.cost, resource.0.maximum
                    ),
                );
            }
            let Some(effect) = self.effects.get(&definition.effect) else {
                self.push_for_identity(
                    &package_id,
                    Some(&correlation),
                    "D20_UNKNOWN_EFFECT",
                    format!("$/payload/reactions/{id}/effect"),
                    format!("unknown effect {}", definition.effect),
                );
                continue;
            };
            if effect.0.defense.as_ref() != Some(&definition.defense)
                || effect.0.defense_bonus != definition.bonus
            {
                self.push_for_identity(
                    &package_id,
                    Some(&correlation),
                    "D20_INCOMPATIBLE_REACTION_EFFECT",
                    format!("$/payload/reactions/{id}/effect"),
                    "reaction defense and bonus must match its effect".to_owned(),
                );
            }
        }
        for (id, (definition, package_id)) in self.actions.clone() {
            let correlation = subject("action", &id);
            match &definition.attack {
                ActionAttackDefinition::Fixed {
                    ability,
                    defense,
                    damage,
                    ..
                } => {
                    for (known, code, path, value) in [
                        (
                            self.abilities.contains_key(ability),
                            "D20_UNKNOWN_ABILITY",
                            "attack/ability",
                            ability.to_string(),
                        ),
                        (
                            self.defenses.contains_key(defense),
                            "D20_UNKNOWN_DEFENSE",
                            "attack/defense",
                            defense.to_string(),
                        ),
                        (
                            self.damage_types.contains_key(&damage.kind),
                            "D20_UNKNOWN_DAMAGE_TYPE",
                            "attack/damage/kind",
                            damage.kind.to_string(),
                        ),
                    ] {
                        if !known {
                            self.push_for_identity(
                                &package_id,
                                Some(&correlation),
                                code,
                                format!("$/payload/actions/{id}/{path}"),
                                format!("unknown reference {value}"),
                            );
                        }
                    }
                }
                ActionAttackDefinition::Implement { implement } => {
                    if !self.implements.contains_key(implement) {
                        self.push_for_identity(
                            &package_id,
                            Some(&correlation),
                            "D20_UNKNOWN_IMPLEMENT",
                            format!("$/payload/actions/{id}/attack/implement"),
                            format!("unknown implement {implement}"),
                        );
                    }
                }
            }
            for cost in &definition.activation_costs {
                let Some(budget) = self.activation_budgets.get(&cost.budget) else {
                    self.push_for_identity(
                        &package_id,
                        Some(&correlation),
                        "D20_UNKNOWN_ACTIVATION_BUDGET",
                        format!("$/payload/actions/{id}/activationCosts"),
                        format!("unknown activation budget {}", cost.budget),
                    );
                    continue;
                };
                if budget.0.timing != ActivationTimingDefinition::Action
                    || cost.amount > budget.0.initial
                {
                    self.push_for_identity(
                        &package_id,
                        Some(&correlation),
                        "D20_INCOMPATIBLE_ACTIVATION_COST",
                        format!("$/payload/actions/{id}/activationCosts"),
                        format!(
                            "action activation cost {} must use an action budget and not exceed {} initial amount {}",
                            cost.amount, cost.budget, budget.0.initial
                        ),
                    );
                }
            }
            if definition
                .effect
                .as_ref()
                .is_some_and(|effect| !self.effects.contains_key(effect))
            {
                self.push_for_identity(
                    &package_id,
                    Some(&correlation),
                    "D20_UNKNOWN_EFFECT",
                    format!("$/payload/actions/{id}/effect"),
                    "action references an unknown effect".to_owned(),
                );
            }
        }
        self.validate_authored_references();
    }

    fn validate_authored_references(&mut self) {
        let mut entity_owners = BTreeMap::<u64, (String, RulePackageIdentity, String)>::new();
        for (id, (definition, package)) in self.character_templates.clone() {
            self.validate_unique_entity(
                &mut entity_owners,
                definition.entity_id,
                format!("character template {id}"),
                package.clone(),
                subject("character-template", &id),
            );
            let correlation = subject("character-template", &id);
            self.validate_unique_ids(
                &package,
                &correlation,
                "characterTemplates",
                &id,
                "actions",
                &definition.actions,
            );
            self.validate_unique_ids(
                &package,
                &correlation,
                "characterTemplates",
                &id,
                "reactions",
                &definition.reactions,
            );
            self.validate_unique_ids(
                &package,
                &correlation,
                "characterTemplates",
                &id,
                "features",
                &definition.features,
            );
            if definition
                .features
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            {
                self.push_for_identity(
                    &package,
                    Some(&correlation),
                    "D20_NONCANONICAL_CHARACTER_FEATURES",
                    format!("$/payload/characterTemplates/{id}/features"),
                    "selected feature identities must be unique and sorted".to_owned(),
                );
            }
            if definition.abilities.len() != self.abilities.len()
                || definition.abilities.iter().any(|(ability, score)| {
                    self.abilities.get(ability).is_none_or(|definition| {
                        *score < definition.0.minimum || *score > definition.0.maximum
                    })
                })
            {
                self.push_for_identity(
                    &package,
                    Some(&correlation),
                    "D20_INVALID_CHARACTER_ABILITIES",
                    format!("$/payload/characterTemplates/{id}/abilities"),
                    "character abilities must define every admitted ability exactly once within its bounds"
                        .to_owned(),
                );
            }
            if definition.resources.len() != self.resources.len()
                || definition.resources.iter().any(|(resource, current)| {
                    self.resources
                        .get(resource)
                        .is_none_or(|definition| *current > definition.0.maximum)
                })
            {
                self.push_for_identity(
                    &package,
                    Some(&correlation),
                    "D20_INVALID_CHARACTER_RESOURCES",
                    format!("$/payload/characterTemplates/{id}/resources"),
                    "character resources must define every admitted resource exactly once within its maximum"
                        .to_owned(),
                );
            }
            for action in &definition.actions {
                if !self.actions.contains_key(action) {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_UNKNOWN_ACTION",
                        format!("$/payload/characterTemplates/{id}/actions"),
                        format!("unknown action {action}"),
                    );
                }
            }
            for reaction in &definition.reactions {
                if !self.reactions.contains_key(reaction) {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_UNKNOWN_REACTION",
                        format!("$/payload/characterTemplates/{id}/reactions"),
                        format!("unknown reaction {reaction}"),
                    );
                }
            }
            for feature in &definition.features {
                if !self.features.contains_key(feature) {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_UNKNOWN_FEATURE",
                        format!("$/payload/characterTemplates/{id}/features"),
                        format!("unknown feature {feature}"),
                    );
                }
            }
            let mut affinities = BTreeSet::new();
            for affinity in &definition.affinities {
                if !affinities.insert(affinity.damage_type.clone()) {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_DUPLICATE_CHARACTER_AFFINITY",
                        format!("$/payload/characterTemplates/{id}/affinities"),
                        format!("duplicate affinity for {}", affinity.damage_type),
                    );
                }
                if !self.damage_types.contains_key(&affinity.damage_type) {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_UNKNOWN_DAMAGE_TYPE",
                        format!("$/payload/characterTemplates/{id}/affinities"),
                        format!("unknown damage type {}", affinity.damage_type),
                    );
                }
            }
        }
        for (id, (definition, package)) in self.storage.clone() {
            self.validate_unique_entity(
                &mut entity_owners,
                definition.entity_id,
                format!("storage {id}"),
                package,
                subject("storage", &id),
            );
        }
        for (id, (definition, package)) in self.item_instances.clone() {
            self.validate_unique_entity(
                &mut entity_owners,
                definition.entity_id,
                format!("item instance {id}"),
                package.clone(),
                subject("item-instance", &id),
            );
            let correlation = subject("item-instance", &id);
            match &definition.equipment {
                EquipmentReferenceDefinition::Armor { armor } => {
                    if !self.armors.contains_key(armor) {
                        self.push_for_identity(
                            &package,
                            Some(&correlation),
                            "D20_UNKNOWN_ARMOR",
                            format!("$/payload/itemInstances/{id}/equipment/armor"),
                            format!("unknown armor {armor}"),
                        );
                    }
                }
                EquipmentReferenceDefinition::Implement { implement } => {
                    if !self.implements.contains_key(implement) {
                        self.push_for_identity(
                            &package,
                            Some(&correlation),
                            "D20_UNKNOWN_IMPLEMENT",
                            format!("$/payload/itemInstances/{id}/equipment/implement"),
                            format!("unknown implement {implement}"),
                        );
                    }
                }
            }
            let owner_is_character = self.character_templates.contains_key(&definition.owner);
            let owner_is_storage = self.storage.contains_key(&definition.owner);
            if !owner_is_character && !owner_is_storage {
                self.push_for_identity(
                    &package,
                    Some(&correlation),
                    "D20_UNKNOWN_ITEM_OWNER",
                    format!("$/payload/itemInstances/{id}/owner"),
                    format!("unknown character or storage owner {}", definition.owner),
                );
            } else if definition.equipped && !owner_is_character {
                self.push_for_identity(
                    &package,
                    Some(&correlation),
                    "D20_INCOMPATIBLE_EQUIPPED_OWNER",
                    format!("$/payload/itemInstances/{id}/equipped"),
                    "an equipped item must be owned by a character".to_owned(),
                );
            }
        }
        for (id, (definition, package)) in self.encounters.clone() {
            let correlation = subject("encounter", &id);
            for participant in &definition.roster {
                if let Some((character, _)) = self.character_templates.get(&participant.character) {
                    if character.actions.is_empty() {
                        self.push_for_identity(
                            &package,
                            Some(&correlation),
                            "D20_ACTIONLESS_ENCOUNTER_PARTICIPANT",
                            format!("$/payload/encounters/{id}/roster"),
                            format!(
                                "encounter participant {} must define at least one action",
                                participant.character
                            ),
                        );
                    }
                } else {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_UNKNOWN_ENCOUNTER_PARTICIPANT",
                        format!("$/payload/encounters/{id}/roster"),
                        format!("unknown character template {}", participant.character),
                    );
                }
            }
            if let Some(item) = definition.victory.reward_item.as_ref() {
                if !self.item_instances.contains_key(item) {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_UNKNOWN_REWARD_ITEM",
                        format!("$/payload/encounters/{id}/victory/rewardItem"),
                        format!("unknown item instance {item}"),
                    );
                }
            }
        }
        let default_count = self
            .adventures
            .values()
            .filter(|(definition, _)| definition.default)
            .count();
        if default_count > 1 {
            self.push_global(
                "D20_MULTIPLE_DEFAULT_ADVENTURES",
                "$/payload/adventures",
                "a resolved package set may define at most one default adventure",
            );
        }
        for (id, (definition, package)) in self.adventures.clone() {
            let correlation = subject("adventure", &id);
            for (field, values) in [
                ("party", definition.party.as_slice()),
                ("characters", definition.characters.as_slice()),
                ("storage", definition.storage.as_slice()),
                ("items", definition.items.as_slice()),
                ("encounters", definition.encounters.as_slice()),
            ] {
                self.validate_unique_ids(&package, &correlation, "adventures", &id, field, values);
            }
            let dungeon_encounters = definition
                .dungeon
                .encounters
                .iter()
                .map(|placement| placement.encounter.clone())
                .collect::<Vec<_>>();
            self.validate_unique_ids(
                &package,
                &correlation,
                "adventures",
                &id,
                "dungeon/encounters",
                &dungeon_encounters,
            );
            let landmark_ids = definition
                .dungeon
                .landmarks
                .iter()
                .map(|landmark| landmark.id.clone())
                .collect::<Vec<_>>();
            self.validate_unique_ids(
                &package,
                &correlation,
                "adventures",
                &id,
                "dungeon/landmarks",
                &landmark_ids,
            );
            let door_ids = definition
                .dungeon
                .doors
                .iter()
                .map(|door| door.id.clone())
                .collect::<Vec<_>>();
            self.validate_unique_ids(
                &package,
                &correlation,
                "adventures",
                &id,
                "dungeon/doors",
                &door_ids,
            );
            let treasure_ids = definition
                .dungeon
                .treasures
                .iter()
                .map(|treasure| treasure.id.clone())
                .collect::<Vec<_>>();
            self.validate_unique_ids(
                &package,
                &correlation,
                "adventures",
                &id,
                "dungeon/treasures",
                &treasure_ids,
            );
            let checkpoint_ids = definition
                .dungeon
                .checkpoints
                .iter()
                .map(|checkpoint| checkpoint.id.clone())
                .collect::<Vec<_>>();
            self.validate_unique_ids(
                &package,
                &correlation,
                "adventures",
                &id,
                "dungeon/checkpoints",
                &checkpoint_ids,
            );
            if dungeon_encounters != definition.encounters {
                self.push_for_identity(
                    &package,
                    Some(&correlation),
                    "D20_INVALID_DUNGEON_ENCOUNTERS",
                    format!("$/payload/adventures/{id}/dungeon/encounters"),
                    "dungeon encounter placements must name every adventure encounter exactly once in authored order"
                        .to_owned(),
                );
            }
            if definition.characters.is_empty()
                || definition.encounters.is_empty()
                || definition
                    .party
                    .iter()
                    .any(|member| !definition.characters.contains(member))
                || !definition.storage.contains(&definition.camp_storage)
                || checkpoint_ids.is_empty()
                || !checkpoint_ids.contains(&definition.dungeon.start_checkpoint)
            {
                self.push_for_identity(
                    &package,
                    Some(&correlation),
                    "D20_INVALID_ADVENTURE_ROOTS",
                    format!("$/payload/adventures/{id}"),
                    "adventure requires characters, encounters, a listed party, listed camp storage, and a valid start checkpoint"
                        .to_owned(),
                );
            }
            if definition.default && !definition.selectable {
                self.push_for_identity(
                    &package,
                    Some(&correlation),
                    "D20_INVALID_DEFAULT_ADVENTURE",
                    format!("$/payload/adventures/{id}/selectable"),
                    "the default adventure must be selectable".to_owned(),
                );
            }
            for party_member in &definition.party {
                if let Some((member, _)) = self.character_templates.get(party_member) {
                    if member.actions.is_empty() {
                        self.push_for_identity(
                            &package,
                            Some(&correlation),
                            "D20_ACTIONLESS_PARTY_MEMBER",
                            format!("$/payload/adventures/{id}/party"),
                            format!("party member {party_member} must define at least one action"),
                        );
                    }
                } else {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_UNKNOWN_PARTY_MEMBER",
                        format!("$/payload/adventures/{id}/party"),
                        format!("unknown character template {party_member}"),
                    );
                }
            }
            for character in &definition.characters {
                if !self.character_templates.contains_key(character) {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_UNKNOWN_CHARACTER_TEMPLATE",
                        format!("$/payload/adventures/{id}/characters"),
                        format!("unknown character template {character}"),
                    );
                }
            }
            for storage in &definition.storage {
                if !self.storage.contains_key(storage) {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_UNKNOWN_STORAGE",
                        format!("$/payload/adventures/{id}/storage"),
                        format!("unknown storage {storage}"),
                    );
                }
            }
            for item in &definition.items {
                let Some((item_definition, _)) = self.item_instances.get(item) else {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_UNKNOWN_ITEM_INSTANCE",
                        format!("$/payload/adventures/{id}/items"),
                        format!("unknown item instance {item}"),
                    );
                    continue;
                };
                if !definition.characters.contains(&item_definition.owner)
                    && !definition.storage.contains(&item_definition.owner)
                {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_ITEM_OWNER_OUTSIDE_ADVENTURE",
                        format!("$/payload/adventures/{id}/items"),
                        format!(
                            "item {item} owner {} is not included in the adventure",
                            item_definition.owner
                        ),
                    );
                }
            }
            for treasure in &definition.dungeon.treasures {
                let Some((item, _)) = self.item_instances.get(&treasure.item) else {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_UNKNOWN_TREASURE_ITEM",
                        format!("$/payload/adventures/{id}/dungeon/treasures"),
                        format!(
                            "treasure {} references unknown item {}",
                            treasure.id, treasure.item
                        ),
                    );
                    continue;
                };
                if !definition.items.contains(&treasure.item)
                    || !definition.storage.contains(&item.owner)
                    || item.owner == definition.camp_storage
                    || item.equipped
                {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_INVALID_TREASURE_ITEM",
                        format!("$/payload/adventures/{id}/dungeon/treasures"),
                        format!(
                            "treasure {} item {} must be an unequipped adventure item owned by listed non-camp storage",
                            treasure.id, treasure.item
                        ),
                    );
                }
            }
            for door in &definition.dungeon.doors {
                if door
                    .requires_treasure
                    .as_ref()
                    .is_some_and(|required| !treasure_ids.contains(required))
                {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_UNKNOWN_DOOR_TREASURE",
                        format!("$/payload/adventures/{id}/dungeon/doors"),
                        format!("door {} requires an unknown dungeon treasure", door.id),
                    );
                }
            }
            for encounter in &definition.encounters {
                let Some((encounter_definition, _)) = self.encounters.get(encounter).cloned()
                else {
                    self.push_for_identity(
                        &package,
                        Some(&correlation),
                        "D20_UNKNOWN_ENCOUNTER",
                        format!("$/payload/adventures/{id}/encounters"),
                        format!("unknown encounter {encounter}"),
                    );
                    continue;
                };
                for participant in &encounter_definition.roster {
                    if !definition.characters.contains(&participant.character) {
                        self.push_for_identity(
                            &package,
                            Some(&correlation),
                            "D20_ENCOUNTER_PARTICIPANT_OUTSIDE_ADVENTURE",
                            format!("$/payload/adventures/{id}/encounters"),
                            format!(
                                "encounter {encounter} participant {} is not included in the adventure",
                                participant.character
                            ),
                        );
                    }
                    if participant.faction == EncounterFactionDefinition::Party
                        && !definition.party.contains(&participant.character)
                    {
                        self.push_for_identity(
                            &package,
                            Some(&correlation),
                            "D20_ENCOUNTER_PARTY_MISMATCH",
                            format!("$/payload/adventures/{id}/encounters"),
                            format!(
                                "encounter {encounter} party participant {} is not in the adventure party",
                                participant.character
                            ),
                        );
                    }
                }
                if let Some(reward) = encounter_definition.victory.reward_item.as_ref() {
                    if !definition.items.contains(reward) {
                        self.push_for_identity(
                            &package,
                            Some(&correlation),
                            "D20_REWARD_OUTSIDE_ADVENTURE",
                            format!("$/payload/adventures/{id}/encounters"),
                            format!("encounter {encounter} reward {reward} is not included"),
                        );
                    }
                }
            }
        }
    }

    fn validate_unique_entity(
        &mut self,
        owners: &mut BTreeMap<u64, (String, RulePackageIdentity, String)>,
        entity: u64,
        label: String,
        package: RulePackageIdentity,
        correlation: String,
    ) {
        if let Some((existing, _, _)) = owners.get(&entity) {
            self.push_for_identity(
                &package,
                Some(&correlation),
                "D20_DUPLICATE_ENTITY_ID",
                "$/payload".to_owned(),
                format!("entity identity {entity} is shared by {existing} and {label}"),
            );
        } else {
            owners.insert(entity, (label, package, correlation));
        }
    }

    fn validate_unique_ids(
        &mut self,
        package: &RulePackageIdentity,
        correlation: &str,
        kind: &str,
        id: &D20Id,
        field: &str,
        values: &[D20Id],
    ) {
        let mut seen = BTreeSet::new();
        if let Some(duplicate) = values.iter().find(|value| !seen.insert((*value).clone())) {
            self.push_for_identity(
                package,
                Some(correlation),
                "D20_DUPLICATE_ADVENTURE_REFERENCE",
                format!("$/payload/{kind}/{id}/{field}"),
                format!("duplicate {field} reference {duplicate}"),
            );
        }
    }
}

pub(super) fn dungeon_offset(x: u16, y: u16, facing: DungeonFacingCandidate) -> Option<(u16, u16)> {
    match facing {
        DungeonFacingCandidate::North => y.checked_sub(1).map(|y| (x, y)),
        DungeonFacingCandidate::East => x.checked_add(1).map(|x| (x, y)),
        DungeonFacingCandidate::South => y.checked_add(1).map(|y| (x, y)),
        DungeonFacingCandidate::West => x.checked_sub(1).map(|x| (x, y)),
    }
}
