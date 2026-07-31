use gameplay_rules::{
    admit_rule_package, AdmittedRulePackage, RuleDomainId, RulePackageCandidate,
    RulePackageDependency, RulePackageError, RulePackageId, RuleProvenance, RuleSource,
    RuleVersion,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    D20Id, D20_ID_PATTERN, MAX_D20_ACTION_TAGS, MAX_D20_ACTION_TARGETS, MAX_D20_ACTIVATION_COSTS,
    MAX_D20_ADVENTURES_PER_PACKAGE, MAX_D20_ADVENTURE_ENTRIES, MAX_D20_AUTHORED_TEXT_BYTES,
    MAX_D20_CONDITION_CLAUSES, MAX_D20_DAMAGE_DICE, MAX_D20_DAMAGE_DIE_SIDES,
    MAX_D20_DEFINITIONS_PER_KIND, MAX_D20_EFFECT_DURATION_TURNS, MAX_D20_EXPERIENCE,
    MAX_D20_FORCED_MOVEMENT, MAX_D20_ID_BYTES, MAX_D20_IMPLEMENT_TAGS,
    MAX_D20_TACTICAL_BOARD_CELLS, MAX_D20_TACTICAL_BOARD_HEIGHT, MAX_D20_TACTICAL_BOARD_WIDTH,
    MAX_D20_TACTICAL_RANGE,
};

pub const D20_CANDIDATE_SCHEMA_VERSION: u32 = 6;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct D20RulesCandidate {
    pub schema_version: u32,
    #[serde(default)]
    pub abilities: Vec<AbilityCandidate>,
    #[serde(default)]
    pub defenses: Vec<DefenseCandidate>,
    #[serde(default)]
    pub activation_budgets: Vec<ActivationBudgetCandidate>,
    #[serde(default)]
    pub damage_types: Vec<DamageTypeCandidate>,
    #[serde(default)]
    pub resources: Vec<ResourceCandidate>,
    #[serde(default)]
    pub armors: Vec<ArmorCandidate>,
    #[serde(default)]
    pub implements: Vec<ImplementCandidate>,
    #[serde(default)]
    pub effects: Vec<EffectCandidate>,
    #[serde(default)]
    pub reactions: Vec<ReactionCandidate>,
    #[serde(default)]
    pub actions: Vec<ActionCandidate>,
    #[serde(default)]
    pub features: Vec<FeatureCandidate>,
    #[serde(default)]
    pub character_templates: Vec<CharacterTemplateCandidate>,
    #[serde(default)]
    pub storage: Vec<StorageCandidate>,
    #[serde(default)]
    pub item_instances: Vec<ItemInstanceCandidate>,
    #[serde(default)]
    pub encounters: Vec<EncounterCandidate>,
    #[serde(default)]
    pub adventures: Vec<AdventureCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AbilityCandidate {
    pub id: D20Id,
    pub minimum: i16,
    pub maximum: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DefenseCandidate {
    pub id: D20Id,
    pub base: i16,
    pub abilities: Vec<D20Id>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum ActivationTimingCandidate {
    Action,
    Reaction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ActivationBudgetCandidate {
    pub id: D20Id,
    pub timing: ActivationTimingCandidate,
    pub initial: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DamageTypeCandidate {
    pub id: D20Id,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ResourceCandidate {
    pub id: D20Id,
    pub maximum: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ArmorCandidate {
    pub id: D20Id,
    pub defense: D20Id,
    pub bonus: i16,
    pub slot: D20Id,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ImplementCandidate {
    pub id: D20Id,
    pub slot: D20Id,
    pub tags: Vec<D20Id>,
    pub ability: D20Id,
    pub defense: D20Id,
    pub damage: DamageCandidate,
    pub range: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[ts(tag = "kind", rename_all = "kebab-case")]
pub enum ConditionClauseCandidate {
    ForbidMovement,
    ForbidActionTag { tag: D20Id },
    AttackPenalty { amount: i16 },
}

impl<'de> Deserialize<'de> for ConditionClauseCandidate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct NoConditionFields {}

        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "kebab-case")]
        enum StrictConditionClause {
            ForbidMovement(NoConditionFields),
            ForbidActionTag { tag: D20Id },
            AttackPenalty { amount: i16 },
        }

        Ok(match StrictConditionClause::deserialize(deserializer)? {
            StrictConditionClause::ForbidMovement(_) => Self::ForbidMovement,
            StrictConditionClause::ForbidActionTag { tag } => Self::ForbidActionTag { tag },
            StrictConditionClause::AttackPenalty { amount } => Self::AttackPenalty { amount },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EffectCandidate {
    pub id: D20Id,
    pub defense: Option<D20Id>,
    pub defense_bonus: i16,
    pub duration_turns: u16,
    pub conditions: Vec<ConditionClauseCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ReactionCandidate {
    pub id: D20Id,
    pub defense: D20Id,
    pub bonus: i16,
    pub resource: D20Id,
    pub cost: u16,
    pub activation_costs: Vec<ActivationCostCandidate>,
    pub effect: D20Id,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ActionCandidate {
    pub id: D20Id,
    pub tags: Vec<D20Id>,
    pub activation_costs: Vec<ActivationCostCandidate>,
    pub target: ActionTargetCandidate,
    pub attack: ActionAttackCandidate,
    pub effect: Option<D20Id>,
    pub forced_movement: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ActivationCostCandidate {
    pub budget: D20Id,
    pub amount: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum ActionTargetKindCandidate {
    Participant,
    Cell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum ActionTargetTeamCandidate {
    Hostile,
    Ally,
    SelfOnly,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum ActionLineOfEffectCandidate {
    Required,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ActionTargetCandidate {
    pub kind: ActionTargetKindCandidate,
    pub team: ActionTargetTeamCandidate,
    pub maximum_targets: u16,
    pub line_of_effect: ActionLineOfEffectCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[ts(tag = "kind", rename_all = "kebab-case")]
pub enum ActionAttackCandidate {
    Fixed {
        ability: D20Id,
        defense: D20Id,
        damage: DamageCandidate,
        range: u16,
    },
    Implement {
        implement: D20Id,
    },
}

impl<'de> Deserialize<'de> for ActionAttackCandidate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
        enum StrictActionAttack {
            Fixed {
                ability: D20Id,
                defense: D20Id,
                damage: DamageCandidate,
                range: u16,
            },
            Implement {
                implement: D20Id,
            },
        }

        Ok(match StrictActionAttack::deserialize(deserializer)? {
            StrictActionAttack::Fixed {
                ability,
                defense,
                damage,
                range,
            } => Self::Fixed {
                ability,
                defense,
                damage,
                range,
            },
            StrictActionAttack::Implement { implement } => Self::Implement { implement },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DamageCandidate {
    pub kind: D20Id,
    pub dice: u8,
    pub sides: u16,
    pub bonus: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CharacterAbilityCandidate {
    pub ability: D20Id,
    pub score: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CharacterResourceCandidate {
    pub resource: D20Id,
    pub current: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum CharacterAffinityKindCandidate {
    Resistant,
    Vulnerable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CharacterAffinityCandidate {
    pub damage_type: D20Id,
    pub affinity: CharacterAffinityKindCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FeatureCandidate {
    pub id: D20Id,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CharacterTemplateCandidate {
    pub id: D20Id,
    #[ts(type = "number")]
    pub entity_id: u64,
    pub name: String,
    pub title: String,
    pub level: u16,
    pub experience: u32,
    pub vitality: u32,
    #[ts(type = "number")]
    pub inventory_capacity: u64,
    pub abilities: Vec<CharacterAbilityCandidate>,
    pub resources: Vec<CharacterResourceCandidate>,
    pub actions: Vec<D20Id>,
    pub reactions: Vec<D20Id>,
    pub affinities: Vec<CharacterAffinityCandidate>,
    pub features: Vec<D20Id>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct StorageCandidate {
    pub id: D20Id,
    #[ts(type = "number")]
    pub entity_id: u64,
    pub name: String,
    #[ts(type = "number")]
    pub capacity: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum ItemRarityCandidate {
    Common,
    Uncommon,
    Rare,
    Epic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ItemInstanceCandidate {
    pub id: D20Id,
    #[ts(type = "number")]
    pub entity_id: u64,
    pub name: String,
    pub equipment: EquipmentReferenceCandidate,
    pub owner: D20Id,
    pub icon: String,
    pub rarity: ItemRarityCandidate,
    pub equipped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[ts(tag = "kind", rename_all = "kebab-case")]
pub enum EquipmentReferenceCandidate {
    Armor { armor: D20Id },
    Implement { implement: D20Id },
}

impl<'de> Deserialize<'de> for EquipmentReferenceCandidate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
        enum StrictEquipmentReference {
            Armor { armor: D20Id },
            Implement { implement: D20Id },
        }

        Ok(match StrictEquipmentReference::deserialize(deserializer)? {
            StrictEquipmentReference::Armor { armor } => Self::Armor { armor },
            StrictEquipmentReference::Implement { implement } => Self::Implement { implement },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EncounterOutcomeCandidate {
    pub title: String,
    pub summary: String,
    pub log_source: String,
    pub log_text: String,
    pub log_details: Vec<String>,
    pub reward_item: Option<D20Id>,
    pub reward_label: Option<String>,
    pub recovery_vitality: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum EncounterFactionCandidate {
    Party,
    Opposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EncounterParticipantCandidate {
    pub character: D20Id,
    pub faction: EncounterFactionCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TacticalPlacementCandidate {
    pub character: D20Id,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TacticalBoardCandidate {
    pub width: u16,
    pub height: u16,
    pub rows: Vec<String>,
    pub placements: Vec<TacticalPlacementCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EncounterCandidate {
    pub id: D20Id,
    pub title: String,
    pub summary: String,
    pub roster: Vec<EncounterParticipantCandidate>,
    pub board: TacticalBoardCandidate,
    pub available_from_camp: bool,
    pub introduction_source: String,
    pub introduction_text: String,
    pub introduction_details: Vec<String>,
    pub victory: EncounterOutcomeCandidate,
    pub defeat: EncounterOutcomeCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum DungeonFacingCandidate {
    North,
    East,
    South,
    West,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DungeonEncounterCandidate {
    pub encounter: D20Id,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DungeonLandmarkCandidate {
    pub id: D20Id,
    pub x: u16,
    pub y: u16,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DungeonDoorCandidate {
    pub id: D20Id,
    pub x: u16,
    pub y: u16,
    pub facing: DungeonFacingCandidate,
    pub title: String,
    pub text: String,
    pub requires_treasure: Option<D20Id>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DungeonTreasureCandidate {
    pub id: D20Id,
    pub x: u16,
    pub y: u16,
    pub item: D20Id,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DungeonCheckpointCandidate {
    pub id: D20Id,
    pub x: u16,
    pub y: u16,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DungeonCandidate {
    pub title: String,
    pub wall_style: D20Id,
    pub width: u16,
    pub height: u16,
    pub rows: Vec<String>,
    pub start_x: u16,
    pub start_y: u16,
    pub start_checkpoint: D20Id,
    pub start_facing: DungeonFacingCandidate,
    pub encounters: Vec<DungeonEncounterCandidate>,
    pub landmarks: Vec<DungeonLandmarkCandidate>,
    pub doors: Vec<DungeonDoorCandidate>,
    pub treasures: Vec<DungeonTreasureCandidate>,
    pub checkpoints: Vec<DungeonCheckpointCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AdventureCompletionCandidate {
    pub source: String,
    pub victory_title: String,
    pub victory_text: String,
    pub defeat_title: String,
    pub defeat_text: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AdventureCandidate {
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
    pub dungeon: DungeonCandidate,
    pub start_source: String,
    pub start_text: String,
    pub start_details: Vec<String>,
    pub completion: AdventureCompletionCandidate,
}

#[derive(Debug, Clone)]
pub struct D20PackageEnvelope {
    pub domain: RuleDomainId,
    pub package: RulePackageId,
    pub version: RuleVersion,
    pub dependencies: Vec<RulePackageDependency>,
    pub sources: Vec<RuleSource>,
    pub provenance: Vec<RuleProvenance>,
}

pub fn admit_d20_candidate(
    envelope: D20PackageEnvelope,
    candidate: D20RulesCandidate,
) -> Result<AdmittedRulePackage, RulePackageError> {
    let payload =
        serde_json::to_value(candidate).map_err(|error| RulePackageError::MalformedJson {
            path: "$/payload".to_owned(),
            offset: 0,
            reason: error.to_string(),
        })?;
    admit_rule_package(RulePackageCandidate::new(
        envelope.domain,
        envelope.package,
        envelope.version,
        envelope.dependencies,
        envelope.sources,
        envelope.provenance,
        payload,
    ))
}

pub fn generated_d20_candidate_typescript() -> String {
    let declarations = [
        D20Id::decl(),
        AbilityCandidate::decl(),
        DefenseCandidate::decl(),
        ActivationTimingCandidate::decl(),
        ActivationBudgetCandidate::decl(),
        DamageTypeCandidate::decl(),
        ResourceCandidate::decl(),
        ArmorCandidate::decl(),
        ImplementCandidate::decl(),
        ConditionClauseCandidate::decl(),
        EffectCandidate::decl(),
        DamageCandidate::decl(),
        ReactionCandidate::decl(),
        ActivationCostCandidate::decl(),
        ActionTargetKindCandidate::decl(),
        ActionTargetTeamCandidate::decl(),
        ActionLineOfEffectCandidate::decl(),
        ActionTargetCandidate::decl(),
        ActionAttackCandidate::decl(),
        ActionCandidate::decl(),
        CharacterAbilityCandidate::decl(),
        CharacterResourceCandidate::decl(),
        CharacterAffinityKindCandidate::decl(),
        CharacterAffinityCandidate::decl(),
        FeatureCandidate::decl(),
        CharacterTemplateCandidate::decl(),
        StorageCandidate::decl(),
        ItemRarityCandidate::decl(),
        EquipmentReferenceCandidate::decl(),
        ItemInstanceCandidate::decl(),
        EncounterOutcomeCandidate::decl(),
        EncounterFactionCandidate::decl(),
        EncounterParticipantCandidate::decl(),
        TacticalPlacementCandidate::decl(),
        TacticalBoardCandidate::decl(),
        EncounterCandidate::decl(),
        DungeonFacingCandidate::decl(),
        DungeonEncounterCandidate::decl(),
        DungeonLandmarkCandidate::decl(),
        DungeonDoorCandidate::decl(),
        DungeonTreasureCandidate::decl(),
        DungeonCheckpointCandidate::decl(),
        DungeonCandidate::decl(),
        AdventureCompletionCandidate::decl(),
        AdventureCandidate::decl(),
        D20RulesCandidate::decl(),
    ]
    .into_iter()
    .map(|declaration| format!("export {declaration}"))
    .collect::<Vec<_>>()
    .join("\n\n");

    format!(
        "// GENERATED by `cargo run -p rusty-d20 --bin rusty-d20-rules-contract`. Do not hand-edit.\n\n\
export const D20_CANDIDATE_SCHEMA_VERSION = {D20_CANDIDATE_SCHEMA_VERSION} as const;\n\
export const D20_ID_PATTERN = {D20_ID_PATTERN:?} as const;\n\
export const D20_LIMITS = Object.freeze({{\n\
  maxIdBytes: {MAX_D20_ID_BYTES},\n\
  maxDefinitionsPerKind: {MAX_D20_DEFINITIONS_PER_KIND},\n\
  maxDamageDice: {MAX_D20_DAMAGE_DICE},\n\
  maxDamageDieSides: {MAX_D20_DAMAGE_DIE_SIDES},\n\
  maxEffectDurationTurns: {MAX_D20_EFFECT_DURATION_TURNS},\n\
  maxExperience: {MAX_D20_EXPERIENCE},\n\
  maxActionTags: {MAX_D20_ACTION_TAGS},\n\
  maxActivationCosts: {MAX_D20_ACTIVATION_COSTS},\n\
  maxConditionClauses: {MAX_D20_CONDITION_CLAUSES},\n\
  maxImplementTags: {MAX_D20_IMPLEMENT_TAGS},\n\
  maxTacticalRange: {MAX_D20_TACTICAL_RANGE},\n\
  maxForcedMovement: {MAX_D20_FORCED_MOVEMENT},\n\
  maxTacticalBoardWidth: {MAX_D20_TACTICAL_BOARD_WIDTH},\n\
  maxTacticalBoardHeight: {MAX_D20_TACTICAL_BOARD_HEIGHT},\n\
  maxTacticalBoardCells: {MAX_D20_TACTICAL_BOARD_CELLS},\n\
  maxActionTargets: {MAX_D20_ACTION_TARGETS},\n\
  maxAdventuresPerPackage: {MAX_D20_ADVENTURES_PER_PACKAGE},\n\
  maxAdventureEntries: {MAX_D20_ADVENTURE_ENTRIES},\n\
  maxAuthoredTextBytes: {MAX_D20_AUTHORED_TEXT_BYTES},\n\
}} as const);\n\n\
{declarations}\n"
    )
}
