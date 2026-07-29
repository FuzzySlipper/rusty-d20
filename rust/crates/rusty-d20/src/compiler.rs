use std::collections::{BTreeMap, BTreeSet, VecDeque};

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
use serde::{Deserialize, Serialize};

use crate::{
    ActionAttackCandidate, ActionCandidate, ActionLineOfEffectCandidate, ActionTargetKindCandidate,
    ActionTargetTeamCandidate, ActivationTimingCandidate, AdventureCandidate, ArmorCandidate,
    CharacterAffinityKindCandidate, CharacterTemplateCandidate, ConditionClauseCandidate, D20Id,
    D20RulesCandidate, DamageCandidate, DungeonCandidate, DungeonFacingCandidate, EffectCandidate,
    EncounterCandidate, EncounterFactionCandidate, EncounterOutcomeCandidate,
    EquipmentReferenceCandidate, ImplementCandidate, ItemInstanceCandidate, ItemRarityCandidate,
    ReactionCandidate, StorageCandidate, D20_CANDIDATE_SCHEMA_VERSION,
};

pub const MAX_D20_DEFINITIONS_PER_KIND: usize = 64;
pub const MAX_D20_ADVENTURES_PER_PACKAGE: usize = 16;
pub const MAX_D20_ADVENTURE_ENTRIES: usize = 64;
pub const MAX_D20_AUTHORED_TEXT_BYTES: usize = 512;
pub const MAX_D20_DUNGEON_WIDTH: u16 = 24;
pub const MAX_D20_DUNGEON_HEIGHT: u16 = 24;
pub const MAX_D20_DUNGEON_CELLS: usize =
    MAX_D20_DUNGEON_WIDTH as usize * MAX_D20_DUNGEON_HEIGHT as usize;
pub const MAX_D20_DAMAGE_DICE: u8 = 32;
pub const MAX_D20_DAMAGE_DIE_SIDES: u16 = 1_000;
pub const MAX_D20_EFFECT_DURATION_TURNS: u16 = 10_000;
pub const MAX_D20_ACTION_TAGS: usize = 16;
pub const MAX_D20_ACTIVATION_COSTS: usize = 4;
pub const MAX_D20_CONDITION_CLAUSES: usize = 8;
pub const MAX_D20_IMPLEMENT_TAGS: usize = 16;
pub const MAX_D20_TACTICAL_RANGE: u16 = 32;
pub const MAX_D20_FORCED_MOVEMENT: u16 = 6;
pub const MAX_D20_TACTICAL_BOARD_WIDTH: u16 = 16;
pub const MAX_D20_TACTICAL_BOARD_HEIGHT: u16 = 16;
pub const MAX_D20_TACTICAL_BOARD_CELLS: usize =
    MAX_D20_TACTICAL_BOARD_WIDTH as usize * MAX_D20_TACTICAL_BOARD_HEIGHT as usize;
pub const MAX_D20_ACTION_TARGETS: u16 = 12;
pub const MAX_D20_PARTY_MEMBERS: usize = 4;
pub const MAX_D20_ENCOUNTER_PARTICIPANTS: usize = 12;
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
    pub abilities: Vec<D20Id>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationTimingDefinition {
    Action,
    Reaction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationBudgetDefinition {
    pub id: D20Id,
    pub timing: ActivationTimingDefinition,
    pub initial: u16,
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
pub struct ImplementDefinition {
    pub id: D20Id,
    pub slot: D20Id,
    pub tags: Vec<D20Id>,
    pub ability: D20Id,
    pub defense: D20Id,
    pub damage: DamageDefinition,
    pub range: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionClauseDefinition {
    ForbidMovement,
    ForbidActionTag { tag: D20Id },
    AttackPenalty { amount: i16 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectDefinition {
    pub id: D20Id,
    pub defense: Option<D20Id>,
    pub defense_bonus: i16,
    pub duration_turns: u16,
    pub conditions: Vec<ConditionClauseDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionDefinition {
    pub id: D20Id,
    pub defense: D20Id,
    pub bonus: i16,
    pub resource: D20Id,
    pub cost: u16,
    pub activation_costs: Vec<ActivationCostDefinition>,
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
pub struct ActivationCostDefinition {
    pub budget: D20Id,
    pub amount: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionTargetKindDefinition {
    Participant,
    Cell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionTargetTeamDefinition {
    Hostile,
    Ally,
    SelfOnly,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionLineOfEffectDefinition {
    Required,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionTargetDefinition {
    pub kind: ActionTargetKindDefinition,
    pub team: ActionTargetTeamDefinition,
    pub maximum_targets: u16,
    pub line_of_effect: ActionLineOfEffectDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionAttackDefinition {
    Fixed {
        ability: D20Id,
        defense: D20Id,
        damage: DamageDefinition,
        range: u16,
    },
    Implement {
        implement: D20Id,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionDefinition {
    pub id: D20Id,
    pub tags: Vec<D20Id>,
    pub activation_costs: Vec<ActivationCostDefinition>,
    pub target: ActionTargetDefinition,
    pub attack: ActionAttackDefinition,
    pub effect: Option<D20Id>,
    pub forced_movement: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterAffinityKindDefinition {
    Resistant,
    Vulnerable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterAffinityDefinition {
    pub damage_type: D20Id,
    pub affinity: CharacterAffinityKindDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterTemplateDefinition {
    pub id: D20Id,
    pub entity_id: u64,
    pub name: String,
    pub title: String,
    pub level: u16,
    pub vitality: u32,
    pub inventory_capacity: u64,
    pub abilities: BTreeMap<D20Id, i16>,
    pub resources: BTreeMap<D20Id, u16>,
    pub actions: Vec<D20Id>,
    pub reactions: Vec<D20Id>,
    pub affinities: Vec<CharacterAffinityDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageDefinition {
    pub id: D20Id,
    pub entity_id: u64,
    pub name: String,
    pub capacity: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemRarityDefinition {
    Common,
    Uncommon,
    Rare,
    Epic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInstanceDefinition {
    pub id: D20Id,
    pub entity_id: u64,
    pub name: String,
    pub equipment: EquipmentReferenceDefinition,
    pub owner: D20Id,
    pub icon: String,
    pub rarity: ItemRarityDefinition,
    pub equipped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EquipmentReferenceDefinition {
    Armor { armor: D20Id },
    Implement { implement: D20Id },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncounterOutcomeDefinition {
    pub title: String,
    pub summary: String,
    pub log_source: String,
    pub log_text: String,
    pub log_details: Vec<String>,
    pub reward_item: Option<D20Id>,
    pub reward_label: Option<String>,
    pub recovery_vitality: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterFactionDefinition {
    Party,
    Opposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncounterParticipantDefinition {
    pub character: D20Id,
    pub faction: EncounterFactionDefinition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TacticalPositionDefinition {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TacticalPlacementDefinition {
    pub character: D20Id,
    pub position: TacticalPositionDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TacticalBoardDefinition {
    pub width: u16,
    pub height: u16,
    pub rows: Vec<String>,
    pub placements: Vec<TacticalPlacementDefinition>,
}

impl TacticalBoardDefinition {
    pub fn is_floor(&self, position: TacticalPositionDefinition) -> bool {
        self.rows
            .get(usize::from(position.y))
            .and_then(|row| row.as_bytes().get(usize::from(position.x)))
            .is_some_and(|cell| *cell == b'.')
    }

    pub fn placement(&self, character: &D20Id) -> Option<TacticalPositionDefinition> {
        self.placements
            .iter()
            .find(|placement| &placement.character == character)
            .map(|placement| placement.position)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncounterDefinition {
    pub id: D20Id,
    pub title: String,
    pub summary: String,
    pub roster: Vec<EncounterParticipantDefinition>,
    pub board: TacticalBoardDefinition,
    pub available_from_camp: bool,
    pub introduction_source: String,
    pub introduction_text: String,
    pub introduction_details: Vec<String>,
    pub victory: EncounterOutcomeDefinition,
    pub defeat: EncounterOutcomeDefinition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DungeonFacingDefinition {
    North,
    East,
    South,
    West,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DungeonEncounterDefinition {
    pub encounter: D20Id,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DungeonLandmarkDefinition {
    pub id: D20Id,
    pub x: u16,
    pub y: u16,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DungeonDoorDefinition {
    pub id: D20Id,
    pub x: u16,
    pub y: u16,
    pub facing: DungeonFacingDefinition,
    pub title: String,
    pub text: String,
    pub requires_treasure: Option<D20Id>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DungeonTreasureDefinition {
    pub id: D20Id,
    pub x: u16,
    pub y: u16,
    pub item: D20Id,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DungeonCheckpointDefinition {
    pub id: D20Id,
    pub x: u16,
    pub y: u16,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DungeonDefinition {
    pub title: String,
    pub wall_style: D20Id,
    pub width: u16,
    pub height: u16,
    pub rows: Vec<String>,
    pub start_x: u16,
    pub start_y: u16,
    pub start_checkpoint: D20Id,
    pub start_facing: DungeonFacingDefinition,
    pub encounters: Vec<DungeonEncounterDefinition>,
    pub landmarks: Vec<DungeonLandmarkDefinition>,
    pub doors: Vec<DungeonDoorDefinition>,
    pub treasures: Vec<DungeonTreasureDefinition>,
    pub checkpoints: Vec<DungeonCheckpointDefinition>,
}

impl DungeonDefinition {
    pub fn is_floor(&self, x: u16, y: u16) -> bool {
        self.rows
            .get(usize::from(y))
            .and_then(|row| row.as_bytes().get(usize::from(x)))
            .is_some_and(|cell| *cell == b'.')
    }

    pub fn checkpoint(&self, id: &str) -> Option<&DungeonCheckpointDefinition> {
        self.checkpoints
            .iter()
            .find(|checkpoint| checkpoint.id.as_str() == id)
    }
}

fn dungeon_offset(x: u16, y: u16, facing: DungeonFacingCandidate) -> Option<(u16, u16)> {
    match facing {
        DungeonFacingCandidate::North => y.checked_sub(1).map(|y| (x, y)),
        DungeonFacingCandidate::East => x.checked_add(1).map(|x| (x, y)),
        DungeonFacingCandidate::South => y.checked_add(1).map(|y| (x, y)),
        DungeonFacingCandidate::West => x.checked_sub(1).map(|x| (x, y)),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdventureCompletionDefinition {
    pub source: String,
    pub victory_title: String,
    pub victory_text: String,
    pub defeat_title: String,
    pub defeat_text: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdventureDefinition {
    pub id: D20Id,
    pub title: String,
    pub default: bool,
    pub selectable: bool,
    pub party: Vec<D20Id>,
    pub characters: Vec<D20Id>,
    pub camp_storage: D20Id,
    pub storage: Vec<D20Id>,
    pub items: Vec<D20Id>,
    pub encounters: Vec<D20Id>,
    pub dungeon: DungeonDefinition,
    pub start_source: String,
    pub start_text: String,
    pub start_details: Vec<String>,
    pub completion: AdventureCompletionDefinition,
}

#[derive(Debug, Clone)]
pub struct D20Ruleset {
    fingerprint: String,
    mechanics: MechanicsCatalog,
    abilities: BTreeMap<D20Id, AbilityDefinition>,
    defenses: BTreeMap<D20Id, DefenseDefinition>,
    activation_budgets: BTreeMap<D20Id, ActivationBudgetDefinition>,
    damage_types: BTreeSet<D20Id>,
    resources: BTreeMap<D20Id, ResourceDefinition>,
    armors: BTreeMap<D20Id, ArmorDefinition>,
    implements: BTreeMap<D20Id, ImplementDefinition>,
    effects: BTreeMap<D20Id, EffectDefinition>,
    reactions: BTreeMap<D20Id, ReactionDefinition>,
    actions: BTreeMap<D20Id, ActionDefinition>,
    character_templates: BTreeMap<D20Id, CharacterTemplateDefinition>,
    storage: BTreeMap<D20Id, StorageDefinition>,
    item_instances: BTreeMap<D20Id, ItemInstanceDefinition>,
    encounters: BTreeMap<D20Id, EncounterDefinition>,
    adventures: BTreeMap<D20Id, AdventureDefinition>,
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

    pub fn activation_budget(&self, id: &D20Id) -> Option<&ActivationBudgetDefinition> {
        self.activation_budgets.get(id)
    }

    pub fn resource(&self, id: &D20Id) -> Option<&ResourceDefinition> {
        self.resources.get(id)
    }

    pub fn armor(&self, id: &D20Id) -> Option<&ArmorDefinition> {
        self.armors.get(id)
    }

    pub fn implement(&self, id: &D20Id) -> Option<&ImplementDefinition> {
        self.implements.get(id)
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

    pub fn character_template(&self, id: &D20Id) -> Option<&CharacterTemplateDefinition> {
        self.character_templates.get(id)
    }

    pub fn storage(&self, id: &D20Id) -> Option<&StorageDefinition> {
        self.storage.get(id)
    }

    pub fn item_instance(&self, id: &D20Id) -> Option<&ItemInstanceDefinition> {
        self.item_instances.get(id)
    }

    pub fn encounter(&self, id: &D20Id) -> Option<&EncounterDefinition> {
        self.encounters.get(id)
    }

    pub fn adventure(&self, id: &D20Id) -> Option<&AdventureDefinition> {
        self.adventures.get(id)
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

    pub fn implements(&self) -> impl Iterator<Item = &ImplementDefinition> {
        self.implements.values()
    }

    pub fn activation_budgets(&self) -> impl Iterator<Item = &ActivationBudgetDefinition> {
        self.activation_budgets.values()
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

    pub fn character_templates(&self) -> impl Iterator<Item = &CharacterTemplateDefinition> {
        self.character_templates.values()
    }

    pub fn damage_types(&self) -> impl Iterator<Item = &D20Id> {
        self.damage_types.iter()
    }

    pub fn adventures(&self) -> impl Iterator<Item = &AdventureDefinition> {
        self.adventures.values()
    }

    pub fn equipment_definition(
        &self,
        equipment: &EquipmentReferenceDefinition,
    ) -> Option<(&D20Id, &D20Id)> {
        match equipment {
            EquipmentReferenceDefinition::Armor { armor } => self
                .armor(armor)
                .map(|definition| (&definition.id, &definition.slot)),
            EquipmentReferenceDefinition::Implement { implement } => self
                .implement(implement)
                .map(|definition| (&definition.id, &definition.slot)),
        }
    }
}

impl EquipmentReferenceDefinition {
    pub const fn id(&self) -> &D20Id {
        match self {
            Self::Armor { armor } => armor,
            Self::Implement { implement } => implement,
        }
    }

    pub(crate) fn mechanics_item_id(&self) -> gameplay_mechanics::ItemDefinitionId {
        match self {
            Self::Armor { armor } => armor_item_id(armor),
            Self::Implement { implement } => implement_item_id(implement),
        }
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
    activation_budgets: BTreeMap<D20Id, (ActivationBudgetDefinition, RulePackageIdentity)>,
    damage_types: BTreeMap<D20Id, RulePackageIdentity>,
    resources: BTreeMap<D20Id, (ResourceDefinition, RulePackageIdentity)>,
    armors: BTreeMap<D20Id, (ArmorDefinition, RulePackageIdentity)>,
    implements: BTreeMap<D20Id, (ImplementDefinition, RulePackageIdentity)>,
    effects: BTreeMap<D20Id, (EffectDefinition, RulePackageIdentity)>,
    reactions: BTreeMap<D20Id, (ReactionDefinition, RulePackageIdentity)>,
    actions: BTreeMap<D20Id, (ActionDefinition, RulePackageIdentity)>,
    character_templates: BTreeMap<D20Id, (CharacterTemplateDefinition, RulePackageIdentity)>,
    storage: BTreeMap<D20Id, (StorageDefinition, RulePackageIdentity)>,
    item_instances: BTreeMap<D20Id, (ItemInstanceDefinition, RulePackageIdentity)>,
    encounters: BTreeMap<D20Id, (EncounterDefinition, RulePackageIdentity)>,
    adventures: BTreeMap<D20Id, (AdventureDefinition, RulePackageIdentity)>,
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
            || value.vitality == 0
            || value.vitality > 1_000_000
            || value.inventory_capacity == 0
        {
            self.push_diagnostic(
                package,
                Some(subject),
                "D20_INVALID_CHARACTER_TEMPLATE",
                format!("$/payload/characterTemplates/{}", value.id),
                "character requires nonzero entity, level, vitality, and inventory capacity"
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

    fn validate_references(&mut self) {
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

    fn finish(self, fingerprint: String) -> Result<D20Ruleset, D20CompileError> {
        let abilities = strip_origins(self.abilities);
        let defenses = strip_origins(self.defenses);
        let activation_budgets = strip_origins(self.activation_budgets);
        let resources = strip_origins(self.resources);
        let armors = strip_origins(self.armors);
        let implements = strip_origins(self.implements);
        let effects = strip_origins(self.effects);
        let reactions = strip_origins(self.reactions);
        let actions = strip_origins(self.actions);
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
            character_templates,
            storage,
            item_instances,
            encounters,
            adventures,
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

fn character_template_definition(value: CharacterTemplateCandidate) -> CharacterTemplateDefinition {
    CharacterTemplateDefinition {
        id: value.id,
        entity_id: value.entity_id,
        name: value.name,
        title: value.title,
        level: value.level,
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

fn build_mechanics_catalog(
    defenses: &BTreeMap<D20Id, DefenseDefinition>,
    damage_types: &BTreeSet<D20Id>,
    armors: &BTreeMap<D20Id, ArmorDefinition>,
    implements: &BTreeMap<D20Id, ImplementDefinition>,
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
        .chain(implements.values().map(|implement| implement.slot.clone()))
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
                classifications: vec![equipment_classification_id(&armor.slot)],
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
            .chain(implements.values().map(|implement| ItemDefinition {
                id: implement_item_id(&implement.id),
                kind: ItemKind::Unique,
                maximum_quantity: 1,
                classifications: vec![equipment_classification_id(&implement.slot)],
                capacity_costs: vec![ItemCapacityCost {
                    metric: loadout_capacity_id(),
                    units: 1,
                }],
                equipment: Some(ItemEquipmentPolicy {
                    required_slots: 1,
                    exclusive_group: None,
                }),
                sources: vec![],
            }))
            .collect(),
        equipment_slots: slots
            .iter()
            .map(|slot| EquipmentSlotDefinition {
                id: equipment_slot_id(slot),
                allowed_classifications: vec![equipment_classification_id(slot)],
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

pub(crate) fn implement_item_id(id: &D20Id) -> gameplay_mechanics::ItemDefinitionId {
    gameplay_mechanics::ItemDefinitionId::parse(format!("implement.{id}"))
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

fn equipment_classification_id(id: &D20Id) -> ItemClassificationId {
    ItemClassificationId::parse(format!("equipment-slot.{id}"))
        .expect("validated d20 identity fits mechanics identity")
}

fn stacking_group_id(value: &str) -> StackingGroupId {
    StackingGroupId::parse(value).expect("validated d20 identity fits mechanics identity")
}

fn scalar(value: i64) -> MechanicsScalar {
    MechanicsScalar::new(value).expect("validated d20 values fit mechanics scalar")
}
