use std::collections::{BTreeMap, BTreeSet, VecDeque};

use gameplay_rules::{
    AdmittedRulePackage, RuleDiagnostic, RuleDiagnosticCorrelation, RuleDiagnosticSeverity,
    RulePackageIdentity, RuleSubjectId, MAX_RULE_DIAGNOSTICS,
};

use crate::*;

use super::mechanics::build_mechanics_catalog;
use super::validation::dungeon_offset;
use super::*;

#[derive(Default)]
pub(super) struct DefinitionCollector {
    pub(super) abilities: BTreeMap<D20Id, (AbilityDefinition, RulePackageIdentity)>,
    pub(super) defenses: BTreeMap<D20Id, (DefenseDefinition, RulePackageIdentity)>,
    pub(super) activation_budgets:
        BTreeMap<D20Id, (ActivationBudgetDefinition, RulePackageIdentity)>,
    pub(super) damage_types: BTreeMap<D20Id, RulePackageIdentity>,
    pub(super) resources: BTreeMap<D20Id, (ResourceDefinition, RulePackageIdentity)>,
    pub(super) armors: BTreeMap<D20Id, (ArmorDefinition, RulePackageIdentity)>,
    pub(super) implements: BTreeMap<D20Id, (ImplementDefinition, RulePackageIdentity)>,
    pub(super) effects: BTreeMap<D20Id, (EffectDefinition, RulePackageIdentity)>,
    pub(super) reactions: BTreeMap<D20Id, (ReactionDefinition, RulePackageIdentity)>,
    pub(super) actions: BTreeMap<D20Id, (ActionDefinition, RulePackageIdentity)>,
    pub(super) features: BTreeMap<D20Id, (FeatureDefinition, RulePackageIdentity)>,
    pub(super) character_templates:
        BTreeMap<D20Id, (CharacterTemplateDefinition, RulePackageIdentity)>,
    pub(super) storage: BTreeMap<D20Id, (StorageDefinition, RulePackageIdentity)>,
    pub(super) item_instances: BTreeMap<D20Id, (ItemInstanceDefinition, RulePackageIdentity)>,
    pub(super) encounters: BTreeMap<D20Id, (EncounterDefinition, RulePackageIdentity)>,
    pub(super) adventures: BTreeMap<D20Id, (AdventureDefinition, RulePackageIdentity)>,
    pub(super) packages: BTreeMap<RulePackageIdentity, AdmittedRulePackage>,
    pub(super) diagnostics: Vec<RuleDiagnostic>,
}

impl DefinitionCollector {
    pub(super) fn include(&mut self, package: &AdmittedRulePackage, candidate: D20RulesCandidate) {
        self.packages
            .insert(package.identity().clone(), package.clone());
        if candidate.schema_version != D20_CANDIDATE_SCHEMA_VERSION {
            self.push_diagnostic(
                package,
                None,
                "D20_UNSUPPORTED_SCHEMA",
                "$/payload/schemaVersion",
                format!(
                    "expected schema version {D20_CANDIDATE_SCHEMA_VERSION}, found {}",
                    candidate.schema_version
                ),
            );
            return;
        }

        self.enforce_quota(package, "abilities", candidate.abilities.len());
        self.enforce_quota(package, "defenses", candidate.defenses.len());
        self.enforce_quota(
            package,
            "activationBudgets",
            candidate.activation_budgets.len(),
        );
        self.enforce_quota(package, "damageTypes", candidate.damage_types.len());
        self.enforce_quota(package, "resources", candidate.resources.len());
        self.enforce_quota(package, "armors", candidate.armors.len());
        self.enforce_quota(package, "implements", candidate.implements.len());
        self.enforce_quota(package, "effects", candidate.effects.len());
        self.enforce_quota(package, "reactions", candidate.reactions.len());
        self.enforce_quota(package, "actions", candidate.actions.len());
        self.enforce_quota(package, "features", candidate.features.len());
        self.enforce_quota(
            package,
            "characterTemplates",
            candidate.character_templates.len(),
        );
        self.enforce_quota(package, "storage", candidate.storage.len());
        self.enforce_quota(package, "itemInstances", candidate.item_instances.len());
        self.enforce_quota(package, "encounters", candidate.encounters.len());
        if candidate.adventures.len() > MAX_D20_ADVENTURES_PER_PACKAGE {
            self.push_diagnostic(
                package,
                None,
                "D20_ADVENTURE_QUOTA",
                "$/payload/adventures",
                format!(
                    "adventures contains {} definitions; maximum is {MAX_D20_ADVENTURES_PER_PACKAGE}",
                    candidate.adventures.len()
                ),
            );
        }

        for value in candidate.abilities {
            let subject = subject("ability", &value.id);
            if value.minimum < 1 || value.maximum > 30 || value.minimum > value.maximum {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_ABILITY_RANGE",
                    format!("$/payload/abilities/{}", value.id),
                    "ability score bounds must be ordered inside 1..=30".to_owned(),
                );
            }
            let definition = AbilityDefinition {
                id: value.id.clone(),
                minimum: value.minimum,
                maximum: value.maximum,
            };
            insert_unique(
                &mut self.abilities,
                value.id,
                definition,
                package,
                "ability",
                &mut self.diagnostics,
            );
        }
        for value in candidate.defenses {
            let subject = subject("defense", &value.id);
            if !(-100..=100).contains(&value.base) {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_DEFENSE_BASE",
                    format!("$/payload/defenses/{}", value.id),
                    "defense base must be inside -100..=100".to_owned(),
                );
            }
            if value.abilities.is_empty()
                || value.abilities.len() > 2
                || value.abilities.iter().collect::<BTreeSet<_>>().len() != value.abilities.len()
            {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_DEFENSE_ABILITIES",
                    format!("$/payload/defenses/{}/abilities", value.id),
                    "defense requires one or two distinct governing abilities".to_owned(),
                );
            }
            let definition = DefenseDefinition {
                id: value.id.clone(),
                base: value.base,
                abilities: value.abilities,
            };
            insert_unique(
                &mut self.defenses,
                value.id,
                definition,
                package,
                "defense",
                &mut self.diagnostics,
            );
        }
        for value in candidate.activation_budgets {
            let subject = subject("activation-budget", &value.id);
            if value.initial == 0 || value.initial > 12 {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_ACTIVATION_BUDGET",
                    format!("$/payload/activationBudgets/{}", value.id),
                    "activation budget initial amount must be inside 1..=12".to_owned(),
                );
            }
            let definition = ActivationBudgetDefinition {
                id: value.id.clone(),
                timing: match value.timing {
                    ActivationTimingCandidate::Action => ActivationTimingDefinition::Action,
                    ActivationTimingCandidate::Reaction => ActivationTimingDefinition::Reaction,
                },
                initial: value.initial,
            };
            insert_unique(
                &mut self.activation_budgets,
                value.id,
                definition,
                package,
                "activation-budget",
                &mut self.diagnostics,
            );
        }
        for value in candidate.damage_types {
            insert_marker(
                &mut self.damage_types,
                value.id,
                package,
                "damage-type",
                &mut self.diagnostics,
            );
        }
        for value in candidate.resources {
            let subject = subject("resource", &value.id);
            if value.maximum == 0 {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_RESOURCE_MAXIMUM",
                    format!("$/payload/resources/{}", value.id),
                    "resource maximum must be positive".to_owned(),
                );
            }
            let definition = ResourceDefinition {
                id: value.id.clone(),
                maximum: value.maximum,
            };
            insert_unique(
                &mut self.resources,
                value.id,
                definition,
                package,
                "resource",
                &mut self.diagnostics,
            );
        }
        for value in candidate.armors {
            let subject = subject("armor", &value.id);
            if !(0..=100).contains(&value.bonus) {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_ARMOR_BONUS",
                    format!("$/payload/armors/{}", value.id),
                    "armor bonus must be inside 0..=100".to_owned(),
                );
            }
            let definition = armor_definition(value);
            insert_unique(
                &mut self.armors,
                definition.id.clone(),
                definition,
                package,
                "armor",
                &mut self.diagnostics,
            );
        }
        for value in candidate.implements {
            let subject = subject("implement", &value.id);
            self.validate_damage(package, "implement", &value.id, &value.damage);
            self.validate_bounded_list(
                package,
                &subject,
                "implements",
                &value.id,
                "tags",
                value.tags.len(),
                MAX_D20_IMPLEMENT_TAGS,
            );
            if value.tags.iter().collect::<BTreeSet<_>>().len() != value.tags.len() {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_DUPLICATE_SEMANTIC_ENTRY",
                    format!("$/payload/implements/{}/tags", value.id),
                    "implement tags must be distinct".to_owned(),
                );
            }
            if value.range == 0 || value.range > MAX_D20_TACTICAL_RANGE {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_TACTICAL_RANGE",
                    format!("$/payload/implements/{}/range", value.id),
                    format!("implement range must be inside 1..={MAX_D20_TACTICAL_RANGE}"),
                );
            }
            let definition = implement_definition(value);
            insert_unique(
                &mut self.implements,
                definition.id.clone(),
                definition,
                package,
                "implement",
                &mut self.diagnostics,
            );
        }
        for value in candidate.effects {
            let subject = subject("effect", &value.id);
            if value.duration_turns == 0 || value.duration_turns > MAX_D20_EFFECT_DURATION_TURNS {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_EFFECT_DURATION",
                    format!("$/payload/effects/{}", value.id),
                    format!("effect duration must be inside 1..={MAX_D20_EFFECT_DURATION_TURNS}"),
                );
            }
            if value.defense.is_none() && value.defense_bonus != 0 {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INCOMPATIBLE_EFFECT_BONUS",
                    format!("$/payload/effects/{}", value.id),
                    "an effect without a defense cannot carry a defense bonus".to_owned(),
                );
            }
            if !(-100..=100).contains(&value.defense_bonus) {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_EFFECT_BONUS",
                    format!("$/payload/effects/{}", value.id),
                    "effect defense bonus must be inside -100..=100".to_owned(),
                );
            }
            self.validate_bounded_list(
                package,
                &subject,
                "effects",
                &value.id,
                "conditions",
                value.conditions.len(),
                MAX_D20_CONDITION_CLAUSES,
            );
            for condition in &value.conditions {
                if let ConditionClauseCandidate::AttackPenalty { amount } = condition {
                    if !(-100..=-1).contains(amount) {
                        self.push_diagnostic(
                            package,
                            Some(&subject),
                            "D20_INVALID_CONDITION_PENALTY",
                            format!("$/payload/effects/{}/conditions", value.id),
                            "condition attack penalty must be inside -100..=-1".to_owned(),
                        );
                    }
                }
            }
            let definition = effect_definition(value);
            insert_unique(
                &mut self.effects,
                definition.id.clone(),
                definition,
                package,
                "effect",
                &mut self.diagnostics,
            );
        }
        for value in candidate.reactions {
            let subject = subject("reaction", &value.id);
            if value.cost == 0 {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_REACTION_COST",
                    format!("$/payload/reactions/{}", value.id),
                    "reaction cost must be positive".to_owned(),
                );
            }
            self.validate_bounded_list(
                package,
                &subject,
                "reactions",
                &value.id,
                "activationCosts",
                value.activation_costs.len(),
                MAX_D20_ACTIVATION_COSTS,
            );
            if value.activation_costs.is_empty()
                || value.activation_costs.iter().any(|cost| cost.amount == 0)
            {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_REACTION_ACTIVATION_COST",
                    format!("$/payload/reactions/{}/activationCosts", value.id),
                    "reactions require positive activation budget costs".to_owned(),
                );
            }
            if value
                .activation_costs
                .iter()
                .map(|cost| &cost.budget)
                .collect::<BTreeSet<_>>()
                .len()
                != value.activation_costs.len()
            {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_DUPLICATE_SEMANTIC_ENTRY",
                    format!("$/payload/reactions/{}/activationCosts", value.id),
                    "a reaction may charge each activation budget at most once".to_owned(),
                );
            }
            let definition = reaction_definition(value);
            insert_unique(
                &mut self.reactions,
                definition.id.clone(),
                definition,
                package,
                "reaction",
                &mut self.diagnostics,
            );
        }
        for value in candidate.actions {
            let subject = subject("action", &value.id);
            self.validate_bounded_list(
                package,
                &subject,
                "actions",
                &value.id,
                "tags",
                value.tags.len(),
                MAX_D20_ACTION_TAGS,
            );
            self.validate_bounded_list(
                package,
                &subject,
                "actions",
                &value.id,
                "activationCosts",
                value.activation_costs.len(),
                MAX_D20_ACTIVATION_COSTS,
            );
            if value.tags.iter().collect::<BTreeSet<_>>().len() != value.tags.len() {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_DUPLICATE_SEMANTIC_ENTRY",
                    format!("$/payload/actions/{}/tags", value.id),
                    "action tags must be distinct".to_owned(),
                );
            }
            if value
                .activation_costs
                .iter()
                .map(|cost| &cost.budget)
                .collect::<BTreeSet<_>>()
                .len()
                != value.activation_costs.len()
            {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_DUPLICATE_SEMANTIC_ENTRY",
                    format!("$/payload/actions/{}/activationCosts", value.id),
                    "an action may charge each activation budget at most once".to_owned(),
                );
            }
            if value.target.maximum_targets == 0
                || value.target.maximum_targets > MAX_D20_ACTION_TARGETS
            {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_ACTION_TARGET_LIMIT",
                    format!("$/payload/actions/{}/target/maximumTargets", value.id),
                    format!("action maximum targets must be inside 1..={MAX_D20_ACTION_TARGETS}"),
                );
            }
            if let ActionAttackCandidate::Fixed { damage, range, .. } = &value.attack {
                self.validate_damage(package, "action", &value.id, damage);
                if *range == 0 || *range > MAX_D20_TACTICAL_RANGE {
                    self.push_diagnostic(
                        package,
                        Some(&subject),
                        "D20_INVALID_TACTICAL_RANGE",
                        format!("$/payload/actions/{}/attack/range", value.id),
                        format!("action range must be inside 1..={MAX_D20_TACTICAL_RANGE}"),
                    );
                }
            }
            if value.forced_movement > MAX_D20_FORCED_MOVEMENT {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_FORCED_MOVEMENT",
                    format!("$/payload/actions/{}/forcedMovement", value.id),
                    format!("forced movement must be inside 0..={MAX_D20_FORCED_MOVEMENT} squares"),
                );
            }
            if value.activation_costs.is_empty() {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_MISSING_ACTIVATION_COST",
                    format!("$/payload/actions/{}/activationCosts", value.id),
                    "combat actions require at least one activation budget cost".to_owned(),
                );
            }
            if value.activation_costs.iter().any(|cost| cost.amount == 0) {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_ACTIVATION_COST",
                    format!("$/payload/actions/{}/activationCosts", value.id),
                    "activation costs must be positive".to_owned(),
                );
            }
            let definition = action_definition(value);
            insert_unique(
                &mut self.actions,
                definition.id.clone(),
                definition,
                package,
                "action",
                &mut self.diagnostics,
            );
        }
        for value in candidate.features {
            let subject = subject("feature", &value.id);
            self.validate_text(package, &subject, "features/label", &value.id, &value.label);
            self.validate_text(
                package,
                &subject,
                "features/description",
                &value.id,
                &value.description,
            );
            let definition = feature_definition(value);
            insert_unique(
                &mut self.features,
                definition.id.clone(),
                definition,
                package,
                "feature",
                &mut self.diagnostics,
            );
        }
        for value in candidate.character_templates {
            let subject = subject("character-template", &value.id);
            self.validate_character_candidate(package, &subject, &value);
            let definition = character_template_definition(value);
            insert_unique(
                &mut self.character_templates,
                definition.id.clone(),
                definition,
                package,
                "character-template",
                &mut self.diagnostics,
            );
        }
        for value in candidate.storage {
            let subject = subject("storage", &value.id);
            if value.entity_id == 0 || value.capacity == 0 {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_STORAGE",
                    format!("$/payload/storage/{}", value.id),
                    "storage requires nonzero entity identity and capacity".to_owned(),
                );
            }
            self.validate_text(package, &subject, "storage/name", &value.id, &value.name);
            let definition = storage_definition(value);
            insert_unique(
                &mut self.storage,
                definition.id.clone(),
                definition,
                package,
                "storage",
                &mut self.diagnostics,
            );
        }
        for value in candidate.item_instances {
            let subject = subject("item-instance", &value.id);
            if value.entity_id == 0 {
                self.push_diagnostic(
                    package,
                    Some(&subject),
                    "D20_INVALID_ITEM_ENTITY",
                    format!("$/payload/itemInstances/{}/entityId", value.id),
                    "item entity identity must be nonzero".to_owned(),
                );
            }
            self.validate_text(
                package,
                &subject,
                "itemInstances/name",
                &value.id,
                &value.name,
            );
            self.validate_text(
                package,
                &subject,
                "itemInstances/icon",
                &value.id,
                &value.icon,
            );
            let definition = item_instance_definition(value);
            insert_unique(
                &mut self.item_instances,
                definition.id.clone(),
                definition,
                package,
                "item-instance",
                &mut self.diagnostics,
            );
        }
        for value in candidate.encounters {
            let subject = subject("encounter", &value.id);
            self.validate_encounter_candidate(package, &subject, &value);
            let definition = encounter_definition(value);
            insert_unique(
                &mut self.encounters,
                definition.id.clone(),
                definition,
                package,
                "encounter",
                &mut self.diagnostics,
            );
        }
        for value in candidate.adventures {
            let subject = subject("adventure", &value.id);
            self.validate_adventure_candidate(package, &subject, &value);
            let definition = adventure_definition(value);
            insert_unique(
                &mut self.adventures,
                definition.id.clone(),
                definition,
                package,
                "adventure",
                &mut self.diagnostics,
            );
        }
    }

    fn validate_character_candidate(
        &mut self,
        package: &AdmittedRulePackage,
        subject: &str,
        value: &CharacterTemplateCandidate,
    ) {
        if value.entity_id == 0
            || value.level == 0
            || value.experience > MAX_D20_EXPERIENCE
            || value.vitality == 0
            || value.vitality > 1_000_000
            || value.inventory_capacity == 0
        {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_INVALID_CHARACTER_TEMPLATE",
                format!("$/payload/characterTemplates/{}", value.id),
                "character requires nonzero entity, level, vitality, and inventory capacity with bounded experience"
                    .to_owned(),
            );
        }
        for (field, text) in [
            ("name", value.name.as_str()),
            ("title", value.title.as_str()),
        ] {
            self.validate_text(
                package,
                subject,
                &format!("characterTemplates/{field}"),
                &value.id,
                text,
            );
        }
        for (field, actual) in [
            ("abilities", value.abilities.len()),
            ("resources", value.resources.len()),
            ("actions", value.actions.len()),
            ("reactions", value.reactions.len()),
            ("affinities", value.affinities.len()),
            ("features", value.features.len()),
        ] {
            self.validate_adventure_list(
                package,
                subject,
                "characterTemplates",
                &value.id,
                field,
                actual,
            );
        }
    }

    fn validate_encounter_candidate(
        &mut self,
        package: &AdmittedRulePackage,
        subject: &str,
        value: &EncounterCandidate,
    ) {
        for (field, text) in [
            ("title", value.title.as_str()),
            ("summary", value.summary.as_str()),
            ("introductionSource", value.introduction_source.as_str()),
            ("introductionText", value.introduction_text.as_str()),
            ("victory/title", value.victory.title.as_str()),
            ("victory/summary", value.victory.summary.as_str()),
            ("victory/logSource", value.victory.log_source.as_str()),
            ("victory/logText", value.victory.log_text.as_str()),
            ("defeat/title", value.defeat.title.as_str()),
            ("defeat/summary", value.defeat.summary.as_str()),
            ("defeat/logSource", value.defeat.log_source.as_str()),
            ("defeat/logText", value.defeat.log_text.as_str()),
        ] {
            self.validate_text(
                package,
                subject,
                &format!("encounters/{field}"),
                &value.id,
                text,
            );
        }
        for (field, details) in [
            ("introductionDetails", value.introduction_details.as_slice()),
            ("victory/logDetails", value.victory.log_details.as_slice()),
            ("defeat/logDetails", value.defeat.log_details.as_slice()),
        ] {
            self.validate_adventure_list(
                package,
                subject,
                "encounters",
                &value.id,
                field,
                details.len(),
            );
            for detail in details {
                self.validate_text(
                    package,
                    subject,
                    &format!("encounters/{field}"),
                    &value.id,
                    detail,
                );
            }
        }
        let party_count = value
            .roster
            .iter()
            .filter(|participant| participant.faction == EncounterFactionCandidate::Party)
            .count();
        let opposition_count = value.roster.len().saturating_sub(party_count);
        let distinct_participants = value
            .roster
            .iter()
            .map(|participant| &participant.character)
            .collect::<BTreeSet<_>>()
            .len();
        if value.roster.len() > MAX_D20_ENCOUNTER_PARTICIPANTS
            || party_count == 0
            || party_count > MAX_D20_PARTY_MEMBERS
            || opposition_count == 0
            || distinct_participants != value.roster.len()
        {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_INVALID_ENCOUNTER_ROSTER",
                format!("$/payload/encounters/{}/roster", value.id),
                format!(
                    "encounter roster must contain distinct participants, 1..={MAX_D20_PARTY_MEMBERS} party members, at least one opponent, and no more than {MAX_D20_ENCOUNTER_PARTICIPANTS} total participants"
                ),
            );
        }
        self.validate_tactical_board_candidate(package, subject, value);
        if value.victory.reward_item.is_some() != value.victory.reward_label.is_some()
            || value.victory.recovery_vitality.is_some()
            || value.defeat.reward_item.is_some()
            || value.defeat.reward_label.is_some()
            || value.defeat.recovery_vitality == Some(0)
        {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_INVALID_ENCOUNTER_OUTCOME",
                format!("$/payload/encounters/{}", value.id),
                "victory reward identity/label must be paired, victory cannot recover vitality, and defeat cannot reward or recover zero vitality".to_owned(),
            );
        }
    }

    fn validate_tactical_board_candidate(
        &mut self,
        package: &AdmittedRulePackage,
        subject: &str,
        encounter: &EncounterCandidate,
    ) {
        let board = &encounter.board;
        let dimensions_valid = board.width >= 5
            && board.height >= 5
            && board.width <= MAX_D20_TACTICAL_BOARD_WIDTH
            && board.height <= MAX_D20_TACTICAL_BOARD_HEIGHT
            && usize::from(board.width) * usize::from(board.height) <= MAX_D20_TACTICAL_BOARD_CELLS;
        let rows_valid = dimensions_valid
            && board.rows.len() == usize::from(board.height)
            && board.rows.iter().all(|row| {
                row.len() == usize::from(board.width)
                    && row.bytes().all(|cell| matches!(cell, b'#' | b'.'))
            });
        let enclosed = rows_valid
            && board
                .rows
                .first()
                .is_some_and(|row| row.bytes().all(|cell| cell == b'#'))
            && board
                .rows
                .last()
                .is_some_and(|row| row.bytes().all(|cell| cell == b'#'))
            && board.rows.iter().all(|row| {
                row.as_bytes().first() == Some(&b'#') && row.as_bytes().last() == Some(&b'#')
            });
        if !rows_valid || !enclosed {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_INVALID_TACTICAL_BOARD",
                format!("$/payload/encounters/{}/board", encounter.id),
                format!(
                    "tactical board must be an enclosed 5..={MAX_D20_TACTICAL_BOARD_WIDTH} by 5..={MAX_D20_TACTICAL_BOARD_HEIGHT} ASCII grid containing only # and ."
                ),
            );
            return;
        }

        let is_floor = |x: u16, y: u16| {
            board
                .rows
                .get(usize::from(y))
                .and_then(|row| row.as_bytes().get(usize::from(x)))
                == Some(&b'.')
        };
        let roster = encounter
            .roster
            .iter()
            .map(|participant| &participant.character)
            .collect::<BTreeSet<_>>();
        let placed = board
            .placements
            .iter()
            .map(|placement| &placement.character)
            .collect::<BTreeSet<_>>();
        let distinct_cells = board
            .placements
            .iter()
            .map(|placement| (placement.x, placement.y))
            .collect::<BTreeSet<_>>();
        if board.placements.len() != encounter.roster.len()
            || placed != roster
            || placed.len() != board.placements.len()
            || distinct_cells.len() != board.placements.len()
            || board
                .placements
                .iter()
                .any(|placement| !is_floor(placement.x, placement.y))
        {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_INVALID_TACTICAL_PLACEMENTS",
                format!("$/payload/encounters/{}/board/placements", encounter.id),
                "tactical placements must name every roster participant exactly once on distinct traversable cells"
                    .to_owned(),
            );
            return;
        }

        let Some(start) = board
            .placements
            .first()
            .map(|placement| (placement.x, placement.y))
        else {
            return;
        };
        let mut reachable = BTreeSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some((x, y)) = queue.pop_front() {
            for (next_x, next_y) in [
                (x.wrapping_sub(1), y),
                (x.saturating_add(1), y),
                (x, y.wrapping_sub(1)),
                (x, y.saturating_add(1)),
            ] {
                if is_floor(next_x, next_y) && reachable.insert((next_x, next_y)) {
                    queue.push_back((next_x, next_y));
                }
            }
        }
        if board
            .placements
            .iter()
            .any(|placement| !reachable.contains(&(placement.x, placement.y)))
        {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_UNREACHABLE_TACTICAL_PLACEMENT",
                format!("$/payload/encounters/{}/board", encounter.id),
                "every tactical placement must share one traversable board region".to_owned(),
            );
        }
    }

    fn validate_adventure_candidate(
        &mut self,
        package: &AdmittedRulePackage,
        subject: &str,
        value: &AdventureCandidate,
    ) {
        for (field, text) in [
            ("title", value.title.as_str()),
            ("dungeon/title", value.dungeon.title.as_str()),
            ("startSource", value.start_source.as_str()),
            ("startText", value.start_text.as_str()),
            ("completion/source", value.completion.source.as_str()),
            (
                "completion/victoryTitle",
                value.completion.victory_title.as_str(),
            ),
            (
                "completion/victoryText",
                value.completion.victory_text.as_str(),
            ),
            (
                "completion/defeatTitle",
                value.completion.defeat_title.as_str(),
            ),
            (
                "completion/defeatText",
                value.completion.defeat_text.as_str(),
            ),
        ] {
            self.validate_text(
                package,
                subject,
                &format!("adventures/{field}"),
                &value.id,
                text,
            );
        }
        for (field, actual) in [
            ("party", value.party.len()),
            ("characters", value.characters.len()),
            ("storage", value.storage.len()),
            ("items", value.items.len()),
            ("encounters", value.encounters.len()),
            ("dungeon/encounters", value.dungeon.encounters.len()),
            ("dungeon/landmarks", value.dungeon.landmarks.len()),
            ("dungeon/doors", value.dungeon.doors.len()),
            ("dungeon/treasures", value.dungeon.treasures.len()),
            ("dungeon/checkpoints", value.dungeon.checkpoints.len()),
            ("startDetails", value.start_details.len()),
            ("completion/details", value.completion.details.len()),
        ] {
            self.validate_adventure_list(package, subject, "adventures", &value.id, field, actual);
        }
        if value.party.is_empty() || value.party.len() > MAX_D20_PARTY_MEMBERS {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_INVALID_PARTY_SIZE",
                format!("$/payload/adventures/{}/party", value.id),
                format!("adventure party must contain 1..={MAX_D20_PARTY_MEMBERS} members"),
            );
        }
        for detail in &value.start_details {
            self.validate_text(
                package,
                subject,
                "adventures/startDetails",
                &value.id,
                detail,
            );
        }
        for detail in &value.completion.details {
            self.validate_text(
                package,
                subject,
                "adventures/completion/details",
                &value.id,
                detail,
            );
        }
        for landmark in &value.dungeon.landmarks {
            for (field, text) in [
                ("title", landmark.title.as_str()),
                ("text", landmark.text.as_str()),
            ] {
                self.validate_text(
                    package,
                    subject,
                    &format!("adventures/dungeon/landmarks/{field}"),
                    &landmark.id,
                    text,
                );
            }
        }
        for door in &value.dungeon.doors {
            for (field, text) in [("title", door.title.as_str()), ("text", door.text.as_str())] {
                self.validate_text(
                    package,
                    subject,
                    &format!("adventures/dungeon/doors/{field}"),
                    &door.id,
                    text,
                );
            }
        }
        for treasure in &value.dungeon.treasures {
            for (field, text) in [
                ("title", treasure.title.as_str()),
                ("text", treasure.text.as_str()),
            ] {
                self.validate_text(
                    package,
                    subject,
                    &format!("adventures/dungeon/treasures/{field}"),
                    &treasure.id,
                    text,
                );
            }
        }
        for checkpoint in &value.dungeon.checkpoints {
            for (field, text) in [
                ("title", checkpoint.title.as_str()),
                ("text", checkpoint.text.as_str()),
            ] {
                self.validate_text(
                    package,
                    subject,
                    &format!("adventures/dungeon/checkpoints/{field}"),
                    &checkpoint.id,
                    text,
                );
            }
        }
        self.validate_dungeon_candidate(package, subject, &value.id, &value.dungeon);
    }

    fn validate_dungeon_candidate(
        &mut self,
        package: &AdmittedRulePackage,
        subject: &str,
        adventure: &D20Id,
        dungeon: &DungeonCandidate,
    ) {
        let dimensions_valid = dungeon.width >= 3
            && dungeon.height >= 3
            && dungeon.width <= MAX_D20_DUNGEON_WIDTH
            && dungeon.height <= MAX_D20_DUNGEON_HEIGHT
            && usize::from(dungeon.width) * usize::from(dungeon.height) <= MAX_D20_DUNGEON_CELLS;
        let rows_valid = dimensions_valid
            && dungeon.rows.len() == usize::from(dungeon.height)
            && dungeon.rows.iter().all(|row| {
                row.len() == usize::from(dungeon.width)
                    && row.bytes().all(|cell| matches!(cell, b'#' | b'.'))
            });
        let enclosed = rows_valid
            && dungeon
                .rows
                .first()
                .is_some_and(|row| row.bytes().all(|cell| cell == b'#'))
            && dungeon
                .rows
                .last()
                .is_some_and(|row| row.bytes().all(|cell| cell == b'#'))
            && dungeon.rows.iter().all(|row| {
                row.as_bytes().first() == Some(&b'#') && row.as_bytes().last() == Some(&b'#')
            });
        if !dimensions_valid || !rows_valid || !enclosed {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_INVALID_DUNGEON_TOPOLOGY",
                format!("$/payload/adventures/{adventure}/dungeon"),
                format!(
                    "dungeon must be an enclosed 3..={MAX_D20_DUNGEON_WIDTH} by 3..={MAX_D20_DUNGEON_HEIGHT} ASCII grid containing only # and ."
                ),
            );
            return;
        }

        let is_floor = |x: u16, y: u16| {
            dungeon
                .rows
                .get(usize::from(y))
                .and_then(|row| row.as_bytes().get(usize::from(x)))
                == Some(&b'.')
        };
        if !is_floor(dungeon.start_x, dungeon.start_y) {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_INVALID_DUNGEON_START",
                format!("$/payload/adventures/{adventure}/dungeon/start"),
                "dungeon start must be a traversable cell".to_owned(),
            );
        }

        let mut placements = BTreeSet::new();
        let invalid_placement = dungeon
            .encounters
            .iter()
            .map(|entry| ("encounter", entry.x, entry.y))
            .chain(
                dungeon
                    .landmarks
                    .iter()
                    .map(|entry| ("landmark", entry.x, entry.y)),
            )
            .chain(
                dungeon
                    .treasures
                    .iter()
                    .map(|entry| ("treasure", entry.x, entry.y)),
            )
            .chain(
                dungeon
                    .checkpoints
                    .iter()
                    .map(|entry| ("checkpoint", entry.x, entry.y)),
            )
            .find(|(_, x, y)| !is_floor(*x, *y) || !placements.insert((*x, *y)));
        if let Some((kind, x, y)) = invalid_placement {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_INVALID_DUNGEON_PLACEMENT",
                format!("$/payload/adventures/{adventure}/dungeon/{kind}s"),
                format!("{kind} placement ({x},{y}) is blocked, out of bounds, or overlaps another placement"),
            );
        }

        let invalid_door = dungeon.doors.iter().find(|door| {
            let destination = dungeon_offset(door.x, door.y, door.facing);
            !is_floor(door.x, door.y) || destination.is_none_or(|(x, y)| !is_floor(x, y))
        });
        if let Some(door) = invalid_door {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_INVALID_DUNGEON_DOOR",
                format!("$/payload/adventures/{adventure}/dungeon/doors"),
                format!(
                    "door {} must connect two orthogonally adjacent traversable cells",
                    door.id
                ),
            );
        }
        let distinct_edges = dungeon
            .doors
            .iter()
            .filter_map(|door| {
                dungeon_offset(door.x, door.y, door.facing).map(|destination| {
                    let mut edge = [(door.x, door.y), destination];
                    edge.sort();
                    edge
                })
            })
            .collect::<BTreeSet<_>>();
        if distinct_edges.len() != dungeon.doors.len() {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_DUPLICATE_DUNGEON_DOOR",
                format!("$/payload/adventures/{adventure}/dungeon/doors"),
                "dungeon doors must own distinct traversable edges".to_owned(),
            );
        }

        let mut opened_doors = BTreeSet::new();
        let reachable = loop {
            let mut reachable = BTreeSet::from([(dungeon.start_x, dungeon.start_y)]);
            let mut queue = VecDeque::from([(dungeon.start_x, dungeon.start_y)]);
            while let Some((x, y)) = queue.pop_front() {
                for (next_x, next_y) in [
                    (x.wrapping_sub(1), y),
                    (x.saturating_add(1), y),
                    (x, y.wrapping_sub(1)),
                    (x, y.saturating_add(1)),
                ] {
                    let blocked_by_door = dungeon.doors.iter().any(|door| {
                        let Some(destination) = dungeon_offset(door.x, door.y, door.facing) else {
                            return false;
                        };
                        let connects = ((door.x, door.y) == (x, y)
                            && destination == (next_x, next_y))
                            || ((door.x, door.y) == (next_x, next_y) && destination == (x, y));
                        connects && !opened_doors.contains(door.id.as_str())
                    });
                    if is_floor(next_x, next_y)
                        && !blocked_by_door
                        && reachable.insert((next_x, next_y))
                    {
                        queue.push_back((next_x, next_y));
                    }
                }
            }

            let reachable_treasures = dungeon
                .treasures
                .iter()
                .filter(|treasure| reachable.contains(&(treasure.x, treasure.y)))
                .map(|treasure| treasure.id.as_str())
                .collect::<BTreeSet<_>>();
            let newly_opened = dungeon.doors.iter().any(|door| {
                if opened_doors.contains(door.id.as_str()) {
                    return false;
                }
                let Some(destination) = dungeon_offset(door.x, door.y, door.facing) else {
                    return false;
                };
                let approachable =
                    reachable.contains(&(door.x, door.y)) || reachable.contains(&destination);
                let prerequisite_met = door
                    .requires_treasure
                    .as_ref()
                    .is_none_or(|required| reachable_treasures.contains(required.as_str()));
                approachable && prerequisite_met && opened_doors.insert(door.id.to_string())
            });
            if !newly_opened {
                break reachable;
            }
        };
        if dungeon
            .encounters
            .iter()
            .any(|entry| !reachable.contains(&(entry.x, entry.y)))
            || dungeon
                .landmarks
                .iter()
                .any(|entry| !reachable.contains(&(entry.x, entry.y)))
            || dungeon
                .treasures
                .iter()
                .any(|entry| !reachable.contains(&(entry.x, entry.y)))
            || dungeon
                .checkpoints
                .iter()
                .any(|entry| !reachable.contains(&(entry.x, entry.y)))
        {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_UNREACHABLE_DUNGEON_CONTENT",
                format!("$/payload/adventures/{adventure}/dungeon"),
                "encounters, landmarks, treasures, and checkpoints must be reachable through the authored door and treasure sequence"
                    .to_owned(),
            );
        }
    }

    fn validate_text(
        &mut self,
        package: &AdmittedRulePackage,
        subject: &str,
        field: &str,
        id: &D20Id,
        value: &str,
    ) {
        if value.is_empty() || value.len() > MAX_D20_AUTHORED_TEXT_BYTES {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_AUTHORED_TEXT_BOUNDS",
                format!("$/payload/{field}/{id}"),
                format!("authored text must contain 1..={MAX_D20_AUTHORED_TEXT_BYTES} bytes"),
            );
        }
    }

    fn validate_adventure_list(
        &mut self,
        package: &AdmittedRulePackage,
        subject: &str,
        kind: &str,
        id: &D20Id,
        field: &str,
        actual: usize,
    ) {
        if actual > MAX_D20_ADVENTURE_ENTRIES {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_ADVENTURE_ENTRY_QUOTA",
                format!("$/payload/{kind}/{id}/{field}"),
                format!(
                    "{field} contains {actual} entries; maximum is {MAX_D20_ADVENTURE_ENTRIES}"
                ),
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn validate_bounded_list(
        &mut self,
        package: &AdmittedRulePackage,
        subject: &str,
        kind: &str,
        id: &D20Id,
        field: &str,
        actual: usize,
        maximum: usize,
    ) {
        if actual > maximum {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_SEMANTIC_ENTRY_QUOTA",
                format!("$/payload/{kind}/{id}/{field}"),
                format!("{field} contains {actual} entries; maximum is {maximum}"),
            );
        }
    }

    fn validate_damage(
        &mut self,
        package: &AdmittedRulePackage,
        kind: &str,
        definition: &D20Id,
        damage: &DamageCandidate,
    ) {
        let subject = subject(kind, definition);
        if damage.dice == 0
            || damage.dice > MAX_D20_DAMAGE_DICE
            || damage.sides < 2
            || damage.sides > MAX_D20_DAMAGE_DIE_SIDES
            || !(-1_000..=1_000).contains(&damage.bonus)
        {
            self.push_diagnostic(
                package,
                Some(&subject),
                "D20_INVALID_DAMAGE_DICE",
                format!("$/payload/{kind}s/{definition}/damage"),
                format!(
                    "damage requires 1..={MAX_D20_DAMAGE_DICE} dice, 2..={MAX_D20_DAMAGE_DIE_SIDES} sides, and bonus inside -1000..=1000"
                ),
            );
        }
    }

    fn enforce_quota(&mut self, package: &AdmittedRulePackage, field: &str, actual: usize) {
        if actual > MAX_D20_DEFINITIONS_PER_KIND {
            self.push_diagnostic(
                package,
                None,
                "D20_DEFINITION_QUOTA",
                format!("$/payload/{field}"),
                format!(
                    "{field} contains {actual} definitions; maximum is {MAX_D20_DEFINITIONS_PER_KIND}"
                ),
            );
        }
    }
}

impl DefinitionCollector {
    pub(super) fn finish(self, fingerprint: String) -> Result<D20Ruleset, D20CompileError> {
        let abilities = strip_origins(self.abilities);
        let defenses = strip_origins(self.defenses);
        let activation_budgets = strip_origins(self.activation_budgets);
        let resources = strip_origins(self.resources);
        let armors = strip_origins(self.armors);
        let implements = strip_origins(self.implements);
        let effects = strip_origins(self.effects);
        let reactions = strip_origins(self.reactions);
        let actions = strip_origins(self.actions);
        let features = strip_origins(self.features);
        let character_templates = strip_origins(self.character_templates);
        let storage = strip_origins(self.storage);
        let item_instances = strip_origins(self.item_instances);
        let encounters = strip_origins(self.encounters);
        let adventures = strip_origins(self.adventures);
        let damage_types = self.damage_types.into_keys().collect::<BTreeSet<_>>();
        let mechanics =
            build_mechanics_catalog(&defenses, &damage_types, &armors, &implements, &effects)
                .map_err(D20CompileError::MechanicsCatalog)?;
        Ok(D20Ruleset {
            fingerprint,
            mechanics,
            abilities,
            defenses,
            activation_budgets,
            damage_types,
            resources,
            armors,
            implements,
            effects,
            reactions,
            actions,
            features,
            character_templates,
            storage,
            item_instances,
            encounters,
            adventures,
        })
    }

    pub(super) fn push_global(&mut self, code: &str, path: &str, message: &str) {
        push_bounded(
            &mut self.diagnostics,
            RuleDiagnostic::new(
                code,
                RuleDiagnosticSeverity::Error,
                path,
                message,
                None,
                None,
            )
            .expect("fixed d20 diagnostic contract is valid"),
        );
    }

    pub(super) fn push_for_identity(
        &mut self,
        package: &RulePackageIdentity,
        correlation: Option<&str>,
        code: &str,
        path: String,
        message: String,
    ) {
        let owned = self.packages.get(package).cloned();
        if let Some(package) = owned.as_ref() {
            self.push_diagnostic(package, correlation, code, path, message);
        }
    }

    pub(super) fn push_diagnostic(
        &mut self,
        package: &AdmittedRulePackage,
        subject: Option<&str>,
        code: &str,
        path: impl Into<String>,
        message: String,
    ) {
        let correlation = subject.and_then(|subject| {
            let subject = RuleSubjectId::parse(subject).ok()?;
            let (provenance, _) = package.correlated_source(&subject)?;
            RuleDiagnosticCorrelation::new(
                subject,
                provenance.source().clone(),
                provenance.line(),
                provenance.column(),
            )
            .ok()
        });
        push_bounded(
            &mut self.diagnostics,
            RuleDiagnostic::new(
                code,
                RuleDiagnosticSeverity::Error,
                path,
                message,
                Some(package.identity().clone()),
                correlation,
            )
            .expect("bounded d20 diagnostic values are valid"),
        );
    }
}

fn insert_unique<T>(
    values: &mut BTreeMap<D20Id, (T, RulePackageIdentity)>,
    id: D20Id,
    value: T,
    package: &AdmittedRulePackage,
    kind: &str,
    diagnostics: &mut Vec<RuleDiagnostic>,
) {
    match values.entry(id.clone()) {
        std::collections::btree_map::Entry::Occupied(_) => push_bounded(
            diagnostics,
            RuleDiagnostic::new(
                "D20_DUPLICATE_DEFINITION",
                RuleDiagnosticSeverity::Error,
                format!("$/payload/{kind}/{id}"),
                format!("duplicate {kind} definition {id}"),
                Some(package.identity().clone()),
                correlation(package, &subject(kind, &id)),
            )
            .expect("fixed duplicate diagnostic is valid"),
        ),
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert((value, package.identity().clone()));
        }
    }
}

fn insert_marker(
    values: &mut BTreeMap<D20Id, RulePackageIdentity>,
    id: D20Id,
    package: &AdmittedRulePackage,
    kind: &str,
    diagnostics: &mut Vec<RuleDiagnostic>,
) {
    if values
        .insert(id.clone(), package.identity().clone())
        .is_some()
    {
        push_bounded(
            diagnostics,
            RuleDiagnostic::new(
                "D20_DUPLICATE_DEFINITION",
                RuleDiagnosticSeverity::Error,
                format!("$/payload/{kind}/{id}"),
                format!("duplicate {kind} definition {id}"),
                Some(package.identity().clone()),
                correlation(package, &subject(kind, &id)),
            )
            .expect("fixed duplicate diagnostic is valid"),
        );
    }
}

fn push_bounded(diagnostics: &mut Vec<RuleDiagnostic>, diagnostic: RuleDiagnostic) {
    if diagnostics.len() < MAX_RULE_DIAGNOSTICS {
        diagnostics.push(diagnostic);
    }
}

fn correlation(package: &AdmittedRulePackage, subject: &str) -> Option<RuleDiagnosticCorrelation> {
    let subject = RuleSubjectId::parse(subject).ok()?;
    let (provenance, _) = package.correlated_source(&subject)?;
    RuleDiagnosticCorrelation::new(
        subject,
        provenance.source().clone(),
        provenance.line(),
        provenance.column(),
    )
    .ok()
}

pub(super) fn subject(kind: &str, id: &D20Id) -> String {
    format!("{kind}:{id}")
}

fn strip_origins<T>(values: BTreeMap<D20Id, (T, RulePackageIdentity)>) -> BTreeMap<D20Id, T> {
    values
        .into_iter()
        .map(|(id, (value, _))| (id, value))
        .collect()
}

fn armor_definition(value: ArmorCandidate) -> ArmorDefinition {
    ArmorDefinition {
        id: value.id,
        defense: value.defense,
        bonus: value.bonus,
        slot: value.slot,
    }
}

fn implement_definition(value: ImplementCandidate) -> ImplementDefinition {
    ImplementDefinition {
        id: value.id,
        slot: value.slot,
        tags: value.tags,
        ability: value.ability,
        defense: value.defense,
        damage: damage_definition(value.damage),
        range: value.range,
    }
}

fn effect_definition(value: EffectCandidate) -> EffectDefinition {
    EffectDefinition {
        id: value.id,
        defense: value.defense,
        defense_bonus: value.defense_bonus,
        duration_turns: value.duration_turns,
        conditions: value
            .conditions
            .into_iter()
            .map(|condition| match condition {
                ConditionClauseCandidate::ForbidMovement => {
                    ConditionClauseDefinition::ForbidMovement
                }
                ConditionClauseCandidate::ForbidActionTag { tag } => {
                    ConditionClauseDefinition::ForbidActionTag { tag }
                }
                ConditionClauseCandidate::AttackPenalty { amount } => {
                    ConditionClauseDefinition::AttackPenalty { amount }
                }
            })
            .collect(),
    }
}

fn reaction_definition(value: ReactionCandidate) -> ReactionDefinition {
    ReactionDefinition {
        id: value.id,
        defense: value.defense,
        bonus: value.bonus,
        resource: value.resource,
        cost: value.cost,
        activation_costs: value
            .activation_costs
            .into_iter()
            .map(|cost| ActivationCostDefinition {
                budget: cost.budget,
                amount: cost.amount,
            })
            .collect(),
        effect: value.effect,
    }
}

fn action_definition(value: ActionCandidate) -> ActionDefinition {
    ActionDefinition {
        id: value.id,
        tags: value.tags,
        activation_costs: value
            .activation_costs
            .into_iter()
            .map(|cost| ActivationCostDefinition {
                budget: cost.budget,
                amount: cost.amount,
            })
            .collect(),
        target: ActionTargetDefinition {
            kind: match value.target.kind {
                ActionTargetKindCandidate::Participant => ActionTargetKindDefinition::Participant,
                ActionTargetKindCandidate::Cell => ActionTargetKindDefinition::Cell,
            },
            team: match value.target.team {
                ActionTargetTeamCandidate::Hostile => ActionTargetTeamDefinition::Hostile,
                ActionTargetTeamCandidate::Ally => ActionTargetTeamDefinition::Ally,
                ActionTargetTeamCandidate::SelfOnly => ActionTargetTeamDefinition::SelfOnly,
                ActionTargetTeamCandidate::Any => ActionTargetTeamDefinition::Any,
            },
            maximum_targets: value.target.maximum_targets,
            line_of_effect: match value.target.line_of_effect {
                ActionLineOfEffectCandidate::Required => ActionLineOfEffectDefinition::Required,
                ActionLineOfEffectCandidate::Ignored => ActionLineOfEffectDefinition::Ignored,
            },
        },
        attack: match value.attack {
            ActionAttackCandidate::Fixed {
                ability,
                defense,
                damage,
                range,
            } => ActionAttackDefinition::Fixed {
                ability,
                defense,
                damage: damage_definition(damage),
                range,
            },
            ActionAttackCandidate::Implement { implement } => {
                ActionAttackDefinition::Implement { implement }
            }
        },
        effect: value.effect,
        forced_movement: value.forced_movement,
    }
}

fn damage_definition(value: DamageCandidate) -> DamageDefinition {
    DamageDefinition {
        kind: value.kind,
        dice: value.dice,
        sides: value.sides,
        bonus: value.bonus,
    }
}

fn feature_definition(value: FeatureCandidate) -> FeatureDefinition {
    FeatureDefinition {
        id: value.id,
        label: value.label,
        description: value.description,
    }
}

fn character_template_definition(value: CharacterTemplateCandidate) -> CharacterTemplateDefinition {
    CharacterTemplateDefinition {
        id: value.id,
        entity_id: value.entity_id,
        name: value.name,
        title: value.title,
        level: value.level,
        experience: value.experience,
        vitality: value.vitality,
        inventory_capacity: value.inventory_capacity,
        abilities: value
            .abilities
            .into_iter()
            .map(|ability| (ability.ability, ability.score))
            .collect(),
        resources: value
            .resources
            .into_iter()
            .map(|resource| (resource.resource, resource.current))
            .collect(),
        actions: value.actions,
        reactions: value.reactions,
        affinities: value
            .affinities
            .into_iter()
            .map(|affinity| CharacterAffinityDefinition {
                damage_type: affinity.damage_type,
                affinity: match affinity.affinity {
                    CharacterAffinityKindCandidate::Resistant => {
                        CharacterAffinityKindDefinition::Resistant
                    }
                    CharacterAffinityKindCandidate::Vulnerable => {
                        CharacterAffinityKindDefinition::Vulnerable
                    }
                },
            })
            .collect(),
        features: value.features,
    }
}

fn storage_definition(value: StorageCandidate) -> StorageDefinition {
    StorageDefinition {
        id: value.id,
        entity_id: value.entity_id,
        name: value.name,
        capacity: value.capacity,
    }
}

fn item_instance_definition(value: ItemInstanceCandidate) -> ItemInstanceDefinition {
    ItemInstanceDefinition {
        id: value.id,
        entity_id: value.entity_id,
        name: value.name,
        equipment: match value.equipment {
            EquipmentReferenceCandidate::Armor { armor } => {
                EquipmentReferenceDefinition::Armor { armor }
            }
            EquipmentReferenceCandidate::Implement { implement } => {
                EquipmentReferenceDefinition::Implement { implement }
            }
        },
        owner: value.owner,
        icon: value.icon,
        rarity: match value.rarity {
            ItemRarityCandidate::Common => ItemRarityDefinition::Common,
            ItemRarityCandidate::Uncommon => ItemRarityDefinition::Uncommon,
            ItemRarityCandidate::Rare => ItemRarityDefinition::Rare,
            ItemRarityCandidate::Epic => ItemRarityDefinition::Epic,
        },
        equipped: value.equipped,
    }
}

fn encounter_outcome_definition(value: EncounterOutcomeCandidate) -> EncounterOutcomeDefinition {
    EncounterOutcomeDefinition {
        title: value.title,
        summary: value.summary,
        log_source: value.log_source,
        log_text: value.log_text,
        log_details: value.log_details,
        reward_item: value.reward_item,
        reward_label: value.reward_label,
        recovery_vitality: value.recovery_vitality,
    }
}

fn encounter_definition(value: EncounterCandidate) -> EncounterDefinition {
    EncounterDefinition {
        id: value.id,
        title: value.title,
        summary: value.summary,
        roster: value
            .roster
            .into_iter()
            .map(|participant| EncounterParticipantDefinition {
                character: participant.character,
                faction: match participant.faction {
                    EncounterFactionCandidate::Party => EncounterFactionDefinition::Party,
                    EncounterFactionCandidate::Opposition => EncounterFactionDefinition::Opposition,
                },
            })
            .collect(),
        board: TacticalBoardDefinition {
            width: value.board.width,
            height: value.board.height,
            rows: value.board.rows,
            placements: value
                .board
                .placements
                .into_iter()
                .map(|placement| TacticalPlacementDefinition {
                    character: placement.character,
                    position: TacticalPositionDefinition {
                        x: placement.x,
                        y: placement.y,
                    },
                })
                .collect(),
        },
        available_from_camp: value.available_from_camp,
        introduction_source: value.introduction_source,
        introduction_text: value.introduction_text,
        introduction_details: value.introduction_details,
        victory: encounter_outcome_definition(value.victory),
        defeat: encounter_outcome_definition(value.defeat),
    }
}

fn adventure_definition(value: AdventureCandidate) -> AdventureDefinition {
    AdventureDefinition {
        id: value.id,
        title: value.title,
        default: value.default,
        selectable: value.selectable,
        party: value.party,
        characters: value.characters,
        camp_storage: value.camp_storage,
        storage: value.storage,
        items: value.items,
        encounters: value.encounters,
        dungeon: dungeon_definition(value.dungeon),
        start_source: value.start_source,
        start_text: value.start_text,
        start_details: value.start_details,
        completion: AdventureCompletionDefinition {
            source: value.completion.source,
            victory_title: value.completion.victory_title,
            victory_text: value.completion.victory_text,
            defeat_title: value.completion.defeat_title,
            defeat_text: value.completion.defeat_text,
            details: value.completion.details,
        },
    }
}

fn dungeon_definition(value: DungeonCandidate) -> DungeonDefinition {
    DungeonDefinition {
        title: value.title,
        wall_style: value.wall_style,
        width: value.width,
        height: value.height,
        rows: value.rows,
        start_x: value.start_x,
        start_y: value.start_y,
        start_checkpoint: value.start_checkpoint,
        start_facing: match value.start_facing {
            DungeonFacingCandidate::North => DungeonFacingDefinition::North,
            DungeonFacingCandidate::East => DungeonFacingDefinition::East,
            DungeonFacingCandidate::South => DungeonFacingDefinition::South,
            DungeonFacingCandidate::West => DungeonFacingDefinition::West,
        },
        encounters: value
            .encounters
            .into_iter()
            .map(|entry| DungeonEncounterDefinition {
                encounter: entry.encounter,
                x: entry.x,
                y: entry.y,
            })
            .collect(),
        landmarks: value
            .landmarks
            .into_iter()
            .map(|entry| DungeonLandmarkDefinition {
                id: entry.id,
                x: entry.x,
                y: entry.y,
                title: entry.title,
                text: entry.text,
            })
            .collect(),
        doors: value
            .doors
            .into_iter()
            .map(|entry| DungeonDoorDefinition {
                id: entry.id,
                x: entry.x,
                y: entry.y,
                facing: match entry.facing {
                    DungeonFacingCandidate::North => DungeonFacingDefinition::North,
                    DungeonFacingCandidate::East => DungeonFacingDefinition::East,
                    DungeonFacingCandidate::South => DungeonFacingDefinition::South,
                    DungeonFacingCandidate::West => DungeonFacingDefinition::West,
                },
                title: entry.title,
                text: entry.text,
                requires_treasure: entry.requires_treasure,
            })
            .collect(),
        treasures: value
            .treasures
            .into_iter()
            .map(|entry| DungeonTreasureDefinition {
                id: entry.id,
                x: entry.x,
                y: entry.y,
                item: entry.item,
                title: entry.title,
                text: entry.text,
            })
            .collect(),
        checkpoints: value
            .checkpoints
            .into_iter()
            .map(|entry| DungeonCheckpointDefinition {
                id: entry.id,
                x: entry.x,
                y: entry.y,
                title: entry.title,
                text: entry.text,
            })
            .collect(),
    }
}
