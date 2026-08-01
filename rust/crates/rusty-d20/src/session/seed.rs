use super::*;

pub const DEFAULT_ROLL_SEED: u64 = 0xD20_2026;
pub const MAX_STATIC_ACTION_ROLLS: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StaticActionRoll {
    pub d20: u8,
    pub damage: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "kebab-case",
    tag = "kind",
    rename_all_fields = "camelCase"
)]
pub enum RollSourceConfig {
    Seeded { seed: u64 },
    Static { rolls: Vec<StaticActionRoll> },
}

impl RollSourceConfig {
    pub const fn seeded(seed: u64) -> Self {
        Self::Seeded { seed }
    }

    pub fn static_rolls(rolls: Vec<StaticActionRoll>) -> Result<Self, D20SessionError> {
        let source = Self::Static { rolls };
        source.validate()?;
        Ok(source)
    }

    pub(crate) fn validate(&self) -> Result<(), D20SessionError> {
        let Self::Static { rolls } = self else {
            return Ok(());
        };
        if rolls.is_empty() || rolls.len() > MAX_STATIC_ACTION_ROLLS {
            return Err(D20SessionError::InvalidRollSource(format!(
                "static roll tape must contain 1..={MAX_STATIC_ACTION_ROLLS} action rolls"
            )));
        }
        for (index, roll) in rolls.iter().enumerate() {
            if !(1..=20).contains(&roll.d20) {
                return Err(D20SessionError::InvalidRollSource(format!(
                    "static action roll {index} has d20 result {}; expected 1..=20",
                    roll.d20
                )));
            }
            if roll.damage.len() > usize::from(MAX_D20_DAMAGE_DICE) {
                return Err(D20SessionError::InvalidRollSource(format!(
                    "static action roll {index} has too many damage dice"
                )));
            }
            if let Some(result) = roll
                .damage
                .iter()
                .find(|result| **result == 0 || **result > MAX_D20_DAMAGE_DIE_SIDES)
            {
                return Err(D20SessionError::InvalidRollSource(format!(
                    "static action roll {index} has damage result {result}; expected 1..={MAX_D20_DAMAGE_DIE_SIDES}"
                )));
            }
        }
        Ok(())
    }
}

impl Default for RollSourceConfig {
    fn default() -> Self {
        Self::seeded(DEFAULT_ROLL_SEED)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DamageAffinity {
    Resistant,
    Vulnerable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffinitySeed {
    pub damage_type: D20Id,
    pub affinity: DamageAffinity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterSeed {
    pub entity: EntityId,
    pub name: String,
    pub vitality: u32,
    pub abilities: Vec<AbilityScore>,
    pub resources: Vec<ActionResource>,
    pub affinities: Vec<AffinitySeed>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmorItemSeed {
    pub entity: EntityId,
    pub owner: EntityId,
    pub name: String,
    pub armor: D20Id,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipmentItemSeed {
    pub entity: EntityId,
    pub owner: EntityId,
    pub name: String,
    pub equipment: EquipmentReferenceDefinition,
}

impl From<ArmorItemSeed> for EquipmentItemSeed {
    fn from(value: ArmorItemSeed) -> Self {
        Self {
            entity: value.entity,
            owner: value.owner,
            name: value.name,
            equipment: EquipmentReferenceDefinition::Armor { armor: value.armor },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventorySeed {
    pub owner: EntityId,
    pub maximum_items: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageSeed {
    pub entity: EntityId,
    pub name: String,
    pub maximum_items: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncounterParticipationSeed {
    pub entity: EntityId,
    pub faction: EncounterFaction,
    pub initiative: i16,
    pub position: TacticalPosition,
}
