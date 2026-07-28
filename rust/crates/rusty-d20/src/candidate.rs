use gameplay_rules::{
    admit_rule_package, AdmittedRulePackage, RuleDomainId, RulePackageCandidate,
    RulePackageDependency, RulePackageError, RulePackageId, RuleProvenance, RuleSource,
    RuleVersion,
};
use serde::{Deserialize, Serialize};

use crate::D20Id;

pub const D20_CANDIDATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct D20RulesCandidate {
    pub schema_version: u32,
    #[serde(default)]
    pub abilities: Vec<AbilityCandidate>,
    #[serde(default)]
    pub defenses: Vec<DefenseCandidate>,
    #[serde(default)]
    pub damage_types: Vec<DamageTypeCandidate>,
    #[serde(default)]
    pub resources: Vec<ResourceCandidate>,
    #[serde(default)]
    pub armors: Vec<ArmorCandidate>,
    #[serde(default)]
    pub effects: Vec<EffectCandidate>,
    #[serde(default)]
    pub reactions: Vec<ReactionCandidate>,
    #[serde(default)]
    pub actions: Vec<ActionCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AbilityCandidate {
    pub id: D20Id,
    pub minimum: i16,
    pub maximum: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DefenseCandidate {
    pub id: D20Id,
    pub base: i16,
    pub ability: D20Id,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DamageTypeCandidate {
    pub id: D20Id,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ResourceCandidate {
    pub id: D20Id,
    pub maximum: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ArmorCandidate {
    pub id: D20Id,
    pub defense: D20Id,
    pub bonus: i16,
    pub slot: D20Id,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EffectCandidate {
    pub id: D20Id,
    pub defense: Option<D20Id>,
    pub defense_bonus: i16,
    pub duration_turns: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReactionCandidate {
    pub id: D20Id,
    pub defense: D20Id,
    pub bonus: i16,
    pub resource: D20Id,
    pub cost: u16,
    pub effect: D20Id,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ActionCandidate {
    pub id: D20Id,
    pub ability: D20Id,
    pub defense: D20Id,
    pub damage: DamageCandidate,
    pub effect: Option<D20Id>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DamageCandidate {
    pub kind: D20Id,
    pub dice: u8,
    pub sides: u16,
    pub bonus: i16,
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
