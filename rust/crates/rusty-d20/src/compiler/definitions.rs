use std::collections::{BTreeMap, BTreeSet};

use rusty_engine::gameplay_mechanics::MechanicsCatalog;
use serde::{Deserialize, Serialize};

use crate::D20Id;

use super::{armor_item_id, implement_item_id};

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
pub struct FeatureDefinition {
    pub id: D20Id,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterTemplateDefinition {
    pub id: D20Id,
    pub entity_id: u64,
    pub name: String,
    pub title: String,
    pub level: u16,
    pub experience: u32,
    pub vitality: u32,
    pub inventory_capacity: u64,
    pub abilities: BTreeMap<D20Id, i16>,
    pub resources: BTreeMap<D20Id, u16>,
    pub actions: Vec<D20Id>,
    pub reactions: Vec<D20Id>,
    pub affinities: Vec<CharacterAffinityDefinition>,
    pub features: Vec<D20Id>,
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
    pub(super) fingerprint: String,
    pub(super) mechanics: MechanicsCatalog,
    pub(super) abilities: BTreeMap<D20Id, AbilityDefinition>,
    pub(super) defenses: BTreeMap<D20Id, DefenseDefinition>,
    pub(super) activation_budgets: BTreeMap<D20Id, ActivationBudgetDefinition>,
    pub(super) damage_types: BTreeSet<D20Id>,
    pub(super) resources: BTreeMap<D20Id, ResourceDefinition>,
    pub(super) armors: BTreeMap<D20Id, ArmorDefinition>,
    pub(super) implements: BTreeMap<D20Id, ImplementDefinition>,
    pub(super) effects: BTreeMap<D20Id, EffectDefinition>,
    pub(super) reactions: BTreeMap<D20Id, ReactionDefinition>,
    pub(super) actions: BTreeMap<D20Id, ActionDefinition>,
    pub(super) features: BTreeMap<D20Id, FeatureDefinition>,
    pub(super) character_templates: BTreeMap<D20Id, CharacterTemplateDefinition>,
    pub(super) storage: BTreeMap<D20Id, StorageDefinition>,
    pub(super) item_instances: BTreeMap<D20Id, ItemInstanceDefinition>,
    pub(super) encounters: BTreeMap<D20Id, EncounterDefinition>,
    pub(super) adventures: BTreeMap<D20Id, AdventureDefinition>,
}

impl D20Ruleset {
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

    pub fn feature(&self, id: &D20Id) -> Option<&FeatureDefinition> {
        self.features.get(id)
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

    pub fn features(&self) -> impl Iterator<Item = &FeatureDefinition> {
        self.features.values()
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

    pub(crate) fn mechanics_item_id(&self) -> rusty_engine::gameplay_mechanics::ItemDefinitionId {
        match self {
            Self::Armor { armor } => armor_item_id(armor),
            Self::Implement { implement } => implement_item_id(implement),
        }
    }
}
