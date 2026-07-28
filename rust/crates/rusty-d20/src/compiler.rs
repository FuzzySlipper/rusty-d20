use std::collections::{BTreeMap, BTreeSet};

use gameplay_mechanics::{
    CapacityMetricDefinition, CapacityMetricId, CatalogError, CatalogVersion, DamageKindDefinition,
    DamageKindId, DamageKindSelector, DamageResponseDefinition,
    EffectDefinition as MechanicsEffectDefinition, EffectDefinitionId, EffectStackingPolicy,
    EquipmentSlotDefinition, EquipmentSlotId, ExactRatio, ItemCapacityCost, ItemClassificationId,
    ItemDefinition, ItemEquipmentPolicy, ItemKind, MechanicsCatalog, MechanicsCatalogDefinition,
    MechanicsScalar, SourceDefinition, SourceDefinitionId, StackingGroupId, StackingPolicy,
    StatContribution, StatContributionDefinition, StatDefinition, StatId, TrackDefinition, TrackId,
    TrackMaximum,
};
use gameplay_rules::{
    resolve_rule_packages, AdmittedRulePackage, RuleDiagnostic, RuleDiagnosticCorrelation,
    RuleDiagnosticError, RuleDiagnosticReport, RuleDiagnosticSeverity, RulePackageIdentity,
    RulePackageSetError, RuleSubjectId, MAX_RULE_DIAGNOSTICS,
};

use crate::{
    ActionCandidate, ArmorCandidate, D20Id, D20RulesCandidate, DamageCandidate, EffectCandidate,
    ReactionCandidate, D20_CANDIDATE_SCHEMA_VERSION,
};

pub const MAX_D20_DEFINITIONS_PER_KIND: usize = 64;
pub const MAX_D20_DAMAGE_DICE: u8 = 32;
pub const MAX_D20_DAMAGE_DIE_SIDES: u16 = 1_000;
pub const MAX_D20_EFFECT_DURATION_TURNS: u16 = 10_000;
const VITALITY_TRACK: &str = "vitality";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityDefinition {
    pub id: D20Id,
    pub minimum: i16,
    pub maximum: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefenseDefinition {
    pub id: D20Id,
    pub base: i16,
    pub ability: D20Id,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDefinition {
    pub id: D20Id,
    pub maximum: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmorDefinition {
    pub id: D20Id,
    pub defense: D20Id,
    pub bonus: i16,
    pub slot: D20Id,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectDefinition {
    pub id: D20Id,
    pub defense: Option<D20Id>,
    pub defense_bonus: i16,
    pub duration_turns: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionDefinition {
    pub id: D20Id,
    pub defense: D20Id,
    pub bonus: i16,
    pub resource: D20Id,
    pub cost: u16,
    pub effect: D20Id,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DamageDefinition {
    pub kind: D20Id,
    pub dice: u8,
    pub sides: u16,
    pub bonus: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionDefinition {
    pub id: D20Id,
    pub ability: D20Id,
    pub defense: D20Id,
    pub damage: DamageDefinition,
    pub effect: Option<D20Id>,
}

#[derive(Debug, Clone)]
pub struct D20Ruleset {
    fingerprint: String,
    mechanics: MechanicsCatalog,
    abilities: BTreeMap<D20Id, AbilityDefinition>,
    defenses: BTreeMap<D20Id, DefenseDefinition>,
    damage_types: BTreeSet<D20Id>,
    resources: BTreeMap<D20Id, ResourceDefinition>,
    armors: BTreeMap<D20Id, ArmorDefinition>,
    effects: BTreeMap<D20Id, EffectDefinition>,
    reactions: BTreeMap<D20Id, ReactionDefinition>,
    actions: BTreeMap<D20Id, ActionDefinition>,
}

impl D20Ruleset {
    pub fn compile(packages: Vec<AdmittedRulePackage>) -> Result<Self, D20CompileError> {
        let packages = resolve_rule_packages(packages).map_err(D20CompileError::PackageSet)?;
        let fingerprint = packages
            .packages()
            .iter()
            .map(|package| format!("{}={}", package.identity(), package.fingerprint().as_str()))
            .collect::<Vec<_>>()
            .join("|");

        let mut collector = DefinitionCollector::default();
        for package in packages.packages() {
            let candidate: D20RulesCandidate =
                match serde_json::from_value(package.payload().clone()) {
                    Ok(candidate) => candidate,
                    Err(error) => {
                        collector.push_diagnostic(
                            package,
                            None,
                            "D20_INVALID_PAYLOAD",
                            "$/payload",
                            format!(
                                "payload does not match the strict d20 candidate schema: {error}"
                            ),
                        );
                        continue;
                    }
                };
            collector.include(package, candidate);
        }
        collector.validate_references();

        let report = RuleDiagnosticReport::new(std::mem::take(&mut collector.diagnostics))
            .map_err(D20CompileError::DiagnosticContract)?;
        if report.has_errors() {
            return Err(D20CompileError::Diagnostics(report));
        }
        collector.finish(fingerprint)
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn mechanics(&self) -> &MechanicsCatalog {
        &self.mechanics
    }

    pub fn ability(&self, id: &D20Id) -> Option<&AbilityDefinition> {
        self.abilities.get(id)
    }

    pub fn defense(&self, id: &D20Id) -> Option<&DefenseDefinition> {
        self.defenses.get(id)
    }

    pub fn resource(&self, id: &D20Id) -> Option<&ResourceDefinition> {
        self.resources.get(id)
    }

    pub fn armor(&self, id: &D20Id) -> Option<&ArmorDefinition> {
        self.armors.get(id)
    }

    pub fn effect(&self, id: &D20Id) -> Option<&EffectDefinition> {
        self.effects.get(id)
    }

    pub fn reaction(&self, id: &D20Id) -> Option<&ReactionDefinition> {
        self.reactions.get(id)
    }

    pub fn action(&self, id: &D20Id) -> Option<&ActionDefinition> {
        self.actions.get(id)
    }

    pub fn abilities(&self) -> impl Iterator<Item = &AbilityDefinition> {
        self.abilities.values()
    }

    pub fn defenses(&self) -> impl Iterator<Item = &DefenseDefinition> {
        self.defenses.values()
    }

    pub fn armors(&self) -> impl Iterator<Item = &ArmorDefinition> {
        self.armors.values()
    }

    pub fn resources(&self) -> impl Iterator<Item = &ResourceDefinition> {
        self.resources.values()
    }

    pub fn reactions(&self) -> impl Iterator<Item = &ReactionDefinition> {
        self.reactions.values()
    }

    pub fn actions(&self) -> impl Iterator<Item = &ActionDefinition> {
        self.actions.values()
    }

    pub fn damage_types(&self) -> impl Iterator<Item = &D20Id> {
        self.damage_types.iter()
    }
}

#[derive(Debug)]
pub enum D20CompileError {
    PackageSet(RulePackageSetError),
    Diagnostics(RuleDiagnosticReport),
    DiagnosticContract(RuleDiagnosticError),
    MechanicsCatalog(CatalogError),
}

impl std::fmt::Display for D20CompileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "d20 rules compilation failed: {self:?}")
    }
}

impl std::error::Error for D20CompileError {}

#[derive(Default)]
struct DefinitionCollector {
    abilities: BTreeMap<D20Id, (AbilityDefinition, RulePackageIdentity)>,
    defenses: BTreeMap<D20Id, (DefenseDefinition, RulePackageIdentity)>,
    damage_types: BTreeMap<D20Id, RulePackageIdentity>,
    resources: BTreeMap<D20Id, (ResourceDefinition, RulePackageIdentity)>,
    armors: BTreeMap<D20Id, (ArmorDefinition, RulePackageIdentity)>,
    effects: BTreeMap<D20Id, (EffectDefinition, RulePackageIdentity)>,
    reactions: BTreeMap<D20Id, (ReactionDefinition, RulePackageIdentity)>,
    actions: BTreeMap<D20Id, (ActionDefinition, RulePackageIdentity)>,
    packages: BTreeMap<RulePackageIdentity, AdmittedRulePackage>,
    diagnostics: Vec<RuleDiagnostic>,
}

impl DefinitionCollector {
    fn include(&mut self, package: &AdmittedRulePackage, candidate: D20RulesCandidate) {
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
        self.enforce_quota(package, "damageTypes", candidate.damage_types.len());
        self.enforce_quota(package, "resources", candidate.resources.len());
        self.enforce_quota(package, "armors", candidate.armors.len());
        self.enforce_quota(package, "effects", candidate.effects.len());
        self.enforce_quota(package, "reactions", candidate.reactions.len());
        self.enforce_quota(package, "actions", candidate.actions.len());

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
            let definition = DefenseDefinition {
                id: value.id.clone(),
                base: value.base,
                ability: value.ability,
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
            self.validate_damage(package, &value.id, &value.damage);
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
    }

    fn validate_damage(
        &mut self,
        package: &AdmittedRulePackage,
        action: &D20Id,
        damage: &DamageCandidate,
    ) {
        let subject = subject("action", action);
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
                format!("$/payload/actions/{action}/damage"),
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

    fn validate_references(&mut self) {
        if self.abilities.is_empty()
            || self.defenses.is_empty()
            || self.damage_types.is_empty()
            || self.actions.is_empty()
        {
            self.push_global(
                "D20_INCOMPLETE_RULESET",
                "$/payload",
                "the resolved ruleset requires at least one ability, defense, damage type, and action",
            );
        }

        for (id, (definition, package_id)) in self.defenses.clone() {
            if !self.abilities.contains_key(&definition.ability) {
                self.push_for_identity(
                    &package_id,
                    Some(&subject("defense", &id)),
                    "D20_UNKNOWN_ABILITY",
                    format!("$/payload/defenses/{id}/ability"),
                    format!("unknown ability {}", definition.ability),
                );
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
                        format!("$/payload/actions/{id}/{path}"),
                        format!("unknown reference {value}"),
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
    }

    fn finish(self, fingerprint: String) -> Result<D20Ruleset, D20CompileError> {
        let abilities = strip_origins(self.abilities);
        let defenses = strip_origins(self.defenses);
        let resources = strip_origins(self.resources);
        let armors = strip_origins(self.armors);
        let effects = strip_origins(self.effects);
        let reactions = strip_origins(self.reactions);
        let actions = strip_origins(self.actions);
        let damage_types = self.damage_types.into_keys().collect::<BTreeSet<_>>();
        let mechanics = build_mechanics_catalog(&defenses, &damage_types, &armors, &effects)
            .map_err(D20CompileError::MechanicsCatalog)?;
        Ok(D20Ruleset {
            fingerprint,
            mechanics,
            abilities,
            defenses,
            damage_types,
            resources,
            armors,
            effects,
            reactions,
            actions,
        })
    }

    fn push_global(&mut self, code: &str, path: &str, message: &str) {
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

    fn push_for_identity(
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

    fn push_diagnostic(
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

fn subject(kind: &str, id: &D20Id) -> String {
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

fn effect_definition(value: EffectCandidate) -> EffectDefinition {
    EffectDefinition {
        id: value.id,
        defense: value.defense,
        defense_bonus: value.defense_bonus,
        duration_turns: value.duration_turns,
    }
}

fn reaction_definition(value: ReactionCandidate) -> ReactionDefinition {
    ReactionDefinition {
        id: value.id,
        defense: value.defense,
        bonus: value.bonus,
        resource: value.resource,
        cost: value.cost,
        effect: value.effect,
    }
}

fn action_definition(value: ActionCandidate) -> ActionDefinition {
    ActionDefinition {
        id: value.id,
        ability: value.ability,
        defense: value.defense,
        damage: DamageDefinition {
            kind: value.damage.kind,
            dice: value.damage.dice,
            sides: value.damage.sides,
            bonus: value.damage.bonus,
        },
        effect: value.effect,
    }
}

fn build_mechanics_catalog(
    defenses: &BTreeMap<D20Id, DefenseDefinition>,
    damage_types: &BTreeSet<D20Id>,
    armors: &BTreeMap<D20Id, ArmorDefinition>,
    effects: &BTreeMap<D20Id, EffectDefinition>,
) -> Result<MechanicsCatalog, CatalogError> {
    let mut sources = Vec::new();
    for kind in damage_types {
        sources.push(SourceDefinition {
            id: resistance_source_id(kind),
            priority: 0,
            stat_contributions: vec![],
            damage_responses: vec![DamageResponseDefinition::Scale {
                selector: DamageKindSelector::Exact {
                    damage_kind: damage_kind_id(kind),
                },
                ratio: ExactRatio::new(1, 2).expect("fixed ratio is valid"),
                stacking_group: stacking_group_id(&format!("resistance.{kind}")),
                stacking: StackingPolicy::UniqueBySource,
            }],
        });
        sources.push(SourceDefinition {
            id: vulnerability_source_id(kind),
            priority: 0,
            stat_contributions: vec![],
            damage_responses: vec![DamageResponseDefinition::Scale {
                selector: DamageKindSelector::Exact {
                    damage_kind: damage_kind_id(kind),
                },
                ratio: ExactRatio::new(2, 1).expect("fixed ratio is valid"),
                stacking_group: stacking_group_id(&format!("vulnerability.{kind}")),
                stacking: StackingPolicy::UniqueBySource,
            }],
        });
    }
    for armor in armors.values() {
        sources.push(SourceDefinition {
            id: armor_source_id(&armor.id),
            priority: 0,
            stat_contributions: vec![StatContributionDefinition {
                stat: defense_stat_id(&armor.defense),
                contribution: StatContribution::Add {
                    amount: scalar(i64::from(armor.bonus)),
                },
                stacking_group: stacking_group_id(&format!("armor.{}", armor.defense)),
                stacking: StackingPolicy::Highest,
            }],
            damage_responses: vec![],
        });
    }
    for effect in effects.values() {
        let stat_contributions = effect
            .defense
            .as_ref()
            .map(|defense| {
                vec![StatContributionDefinition {
                    stat: defense_stat_id(defense),
                    contribution: StatContribution::Add {
                        amount: scalar(i64::from(effect.defense_bonus)),
                    },
                    stacking_group: stacking_group_id(&format!("effect.{}", effect.id)),
                    stacking: StackingPolicy::UniqueBySource,
                }]
            })
            .unwrap_or_default();
        sources.push(SourceDefinition {
            id: effect_source_id(&effect.id),
            priority: 0,
            stat_contributions,
            damage_responses: vec![],
        });
    }

    let slots = armors
        .values()
        .map(|armor| armor.slot.clone())
        .collect::<BTreeSet<_>>();
    MechanicsCatalog::admit(MechanicsCatalogDefinition {
        version: CatalogVersion::parse("rusty-d20.v2").expect("fixed version is valid"),
        stats: defenses
            .values()
            .map(|defense| StatDefinition {
                id: defense_stat_id(&defense.id),
                minimum: scalar(-1_000),
                maximum: scalar(1_000),
            })
            .collect(),
        tracks: vec![TrackDefinition {
            id: vitality_track_id(),
            minimum: scalar(0),
            maximum: TrackMaximum::Fixed {
                value: scalar(1_000_000),
            },
        }],
        sources,
        damage_kinds: damage_types
            .iter()
            .map(|kind| DamageKindDefinition {
                id: damage_kind_id(kind),
            })
            .collect(),
        effects: effects
            .values()
            .map(|effect| MechanicsEffectDefinition {
                id: mechanics_effect_id(&effect.id),
                stacking_group: stacking_group_id(&format!("effect.{}", effect.id)),
                stacking: EffectStackingPolicy::Refresh,
                maximum_stacks: 1,
                sources: vec![effect_source_id(&effect.id)],
            })
            .collect(),
        capacity_metrics: vec![CapacityMetricDefinition {
            id: loadout_capacity_id(),
        }],
        items: armors
            .values()
            .map(|armor| ItemDefinition {
                id: armor_item_id(&armor.id),
                kind: ItemKind::Unique,
                maximum_quantity: 1,
                classifications: vec![armor_classification_id(&armor.slot)],
                capacity_costs: vec![ItemCapacityCost {
                    metric: loadout_capacity_id(),
                    units: 1,
                }],
                equipment: Some(ItemEquipmentPolicy {
                    required_slots: 1,
                    exclusive_group: None,
                }),
                sources: vec![armor_source_id(&armor.id)],
            })
            .collect(),
        equipment_slots: slots
            .iter()
            .map(|slot| EquipmentSlotDefinition {
                id: equipment_slot_id(slot),
                allowed_classifications: vec![armor_classification_id(slot)],
            })
            .collect(),
    })
}

pub(crate) fn defense_stat_id(id: &D20Id) -> StatId {
    StatId::parse(format!("defense.{id}")).expect("validated d20 identity fits mechanics identity")
}

pub(crate) fn vitality_track_id() -> TrackId {
    TrackId::parse(VITALITY_TRACK).expect("fixed track identity is valid")
}

pub(crate) fn damage_kind_id(id: &D20Id) -> DamageKindId {
    DamageKindId::parse(id.to_string()).expect("validated d20 identity fits mechanics identity")
}

pub(crate) fn mechanics_effect_id(id: &D20Id) -> EffectDefinitionId {
    EffectDefinitionId::parse(format!("effect.{id}"))
        .expect("validated d20 identity fits mechanics identity")
}

pub(crate) fn armor_item_id(id: &D20Id) -> gameplay_mechanics::ItemDefinitionId {
    gameplay_mechanics::ItemDefinitionId::parse(format!("armor.{id}"))
        .expect("validated d20 identity fits mechanics identity")
}

pub(crate) fn equipment_slot_id(id: &D20Id) -> EquipmentSlotId {
    EquipmentSlotId::parse(id.to_string()).expect("validated d20 identity fits mechanics identity")
}

pub(crate) fn loadout_capacity_id() -> CapacityMetricId {
    CapacityMetricId::parse("carried-items").expect("fixed capacity identity is valid")
}

pub(crate) fn resistance_source_id(id: &D20Id) -> SourceDefinitionId {
    SourceDefinitionId::parse(format!("resistant.{id}"))
        .expect("validated d20 identity fits mechanics identity")
}

pub(crate) fn vulnerability_source_id(id: &D20Id) -> SourceDefinitionId {
    SourceDefinitionId::parse(format!("vulnerable.{id}"))
        .expect("validated d20 identity fits mechanics identity")
}

fn armor_source_id(id: &D20Id) -> SourceDefinitionId {
    SourceDefinitionId::parse(format!("armor.{id}"))
        .expect("validated d20 identity fits mechanics identity")
}

fn effect_source_id(id: &D20Id) -> SourceDefinitionId {
    SourceDefinitionId::parse(format!("effect.{id}"))
        .expect("validated d20 identity fits mechanics identity")
}

fn armor_classification_id(id: &D20Id) -> ItemClassificationId {
    ItemClassificationId::parse(format!("armor-slot.{id}"))
        .expect("validated d20 identity fits mechanics identity")
}

fn stacking_group_id(value: &str) -> StackingGroupId {
    StackingGroupId::parse(value).expect("validated d20 identity fits mechanics identity")
}

fn scalar(value: i64) -> MechanicsScalar {
    MechanicsScalar::new(value).expect("validated d20 values fit mechanics scalar")
}
