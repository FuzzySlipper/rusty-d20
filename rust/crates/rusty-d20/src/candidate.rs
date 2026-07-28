use gameplay_rules::{
    admit_rule_package, AdmittedRulePackage, RuleDomainId, RulePackageCandidate,
    RulePackageDependency, RulePackageError, RulePackageId, RuleProvenance, RuleSource,
    RuleVersion,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    D20Id, D20_ID_PATTERN, MAX_D20_DAMAGE_DICE, MAX_D20_DAMAGE_DIE_SIDES,
    MAX_D20_DEFINITIONS_PER_KIND, MAX_D20_EFFECT_DURATION_TURNS, MAX_D20_ID_BYTES,
};

pub const D20_CANDIDATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
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
    pub ability: D20Id,
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
pub struct EffectCandidate {
    pub id: D20Id,
    pub defense: Option<D20Id>,
    pub defense_bonus: i16,
    pub duration_turns: u16,
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
    pub effect: D20Id,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ActionCandidate {
    pub id: D20Id,
    pub ability: D20Id,
    pub defense: D20Id,
    pub damage: DamageCandidate,
    pub effect: Option<D20Id>,
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
        DamageTypeCandidate::decl(),
        ResourceCandidate::decl(),
        ArmorCandidate::decl(),
        EffectCandidate::decl(),
        DamageCandidate::decl(),
        ReactionCandidate::decl(),
        ActionCandidate::decl(),
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
}} as const);\n\n\
{declarations}\n"
    )
}
