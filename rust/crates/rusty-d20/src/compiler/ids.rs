use rusty_engine::gameplay_mechanics::{
    CapacityMetricId, DamageKindId, EffectDefinitionId, EquipmentSlotId, ItemClassificationId,
    MechanicsScalar, SourceDefinitionId, StackingGroupId, StatId, TrackId,
};

use crate::D20Id;

use super::VITALITY_TRACK;

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

pub(crate) fn armor_item_id(id: &D20Id) -> rusty_engine::gameplay_mechanics::ItemDefinitionId {
    rusty_engine::gameplay_mechanics::ItemDefinitionId::parse(format!("armor.{id}"))
        .expect("validated d20 identity fits mechanics identity")
}

pub(crate) fn implement_item_id(id: &D20Id) -> rusty_engine::gameplay_mechanics::ItemDefinitionId {
    rusty_engine::gameplay_mechanics::ItemDefinitionId::parse(format!("implement.{id}"))
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

pub(super) fn armor_source_id(id: &D20Id) -> SourceDefinitionId {
    SourceDefinitionId::parse(format!("armor.{id}"))
        .expect("validated d20 identity fits mechanics identity")
}

pub(super) fn effect_source_id(id: &D20Id) -> SourceDefinitionId {
    SourceDefinitionId::parse(format!("effect.{id}"))
        .expect("validated d20 identity fits mechanics identity")
}

pub(super) fn equipment_classification_id(id: &D20Id) -> ItemClassificationId {
    ItemClassificationId::parse(format!("equipment-slot.{id}"))
        .expect("validated d20 identity fits mechanics identity")
}

pub(super) fn stacking_group_id(value: &str) -> StackingGroupId {
    StackingGroupId::parse(value).expect("validated d20 identity fits mechanics identity")
}

pub(super) fn scalar(value: i64) -> MechanicsScalar {
    MechanicsScalar::new(value).expect("validated d20 values fit mechanics scalar")
}
