use std::collections::{BTreeMap, BTreeSet};

use gameplay_mechanics::{
    CapacityMetricDefinition, CatalogError, CatalogVersion, DamageKindDefinition,
    DamageKindSelector, DamageResponseDefinition, EffectDefinition as MechanicsEffectDefinition,
    EffectStackingPolicy, EquipmentSlotDefinition, ExactRatio, ItemCapacityCost, ItemDefinition,
    ItemEquipmentPolicy, ItemKind, MechanicsCatalog, MechanicsCatalogDefinition, SourceDefinition,
    StackingPolicy, StatContribution, StatContributionDefinition, StatDefinition, TrackDefinition,
    TrackMaximum,
};

use crate::D20Id;

use super::ids::{
    armor_source_id, effect_source_id, equipment_classification_id, scalar, stacking_group_id,
};
use super::*;

pub(super) fn build_mechanics_catalog(
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
