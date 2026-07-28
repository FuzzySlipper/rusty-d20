use core_ids::EntityId;
use gameplay_mechanics::{
    EffectInstanceId, OperationId, SourceInstanceIdentity, TrackId, TracksComponent,
};
use gameplay_rules::{
    admit_rule_package, decode_canonical_rule_package, encode_rule_package, AdmittedRulePackage,
    RuleDomainId, RulePackageCandidate, RulePackageDependency, RulePackageId, RuleProvenance,
    RuleSource, RuleSourceId, RuleSubjectId, RuleVersion,
};
use rusty_d20::{
    ability_modifier, admit_d20_candidate, AbilityCandidate, AbilityScore, ActionCandidate,
    ActionResource, AffinitySeed, ApplyActionRequest, ArmorCandidate, ArmorItemSeed, CharacterSeed,
    D20CompileError, D20Id, D20PackageEnvelope, D20RulesCandidate, D20Ruleset, D20Session,
    D20SessionError, DamageAffinity, DamageCandidate, DamageTypeCandidate, DefenseCandidate,
    EffectCandidate, ReactionCandidate, ResourceCandidate, SessionSaveError,
    D20_CANDIDATE_SCHEMA_VERSION,
};
use serde_json::json;
use svc_rng::RngSeed;

const ATTACKER: EntityId = EntityId::new(101);
const TARGET: EntityId = EntityId::new(102);
const ARMOR_ITEM: EntityId = EntityId::new(201);

fn id(value: &str) -> D20Id {
    D20Id::parse(value).unwrap()
}

fn operation(value: &str) -> OperationId {
    OperationId::parse(value).unwrap()
}

fn effect_instance(value: &str) -> EffectInstanceId {
    EffectInstanceId::parse(value).unwrap()
}

fn base_candidate() -> D20RulesCandidate {
    D20RulesCandidate {
        schema_version: D20_CANDIDATE_SCHEMA_VERSION,
        abilities: vec![
            AbilityCandidate {
                id: id("dexterity"),
                minimum: 1,
                maximum: 30,
            },
            AbilityCandidate {
                id: id("strength"),
                minimum: 1,
                maximum: 30,
            },
        ],
        defenses: vec![DefenseCandidate {
            id: id("armor"),
            base: 1,
            ability: id("dexterity"),
        }],
        damage_types: vec![DamageTypeCandidate { id: id("slashing") }],
        resources: vec![ResourceCandidate {
            id: id("guard"),
            maximum: 2,
        }],
        armors: vec![ArmorCandidate {
            id: id("chain"),
            defense: id("armor"),
            bonus: 3,
            slot: id("body"),
        }],
        effects: vec![
            EffectCandidate {
                id: id("bleeding"),
                defense: None,
                defense_bonus: 0,
                duration_turns: 2,
            },
            EffectCandidate {
                id: id("reaction-guard"),
                defense: Some(id("armor")),
                defense_bonus: 5,
                duration_turns: 1,
            },
        ],
        reactions: vec![ReactionCandidate {
            id: id("parry"),
            defense: id("armor"),
            bonus: 5,
            resource: id("guard"),
            cost: 1,
            effect: id("reaction-guard"),
        }],
        actions: vec![ActionCandidate {
            id: id("strike"),
            ability: id("strength"),
            defense: id("armor"),
            damage: DamageCandidate {
                kind: id("slashing"),
                dice: 1,
                sides: 8,
                bonus: 2,
            },
            effect: Some(id("bleeding")),
        }],
    }
}

fn envelope(
    package: &str,
    dependencies: Vec<RulePackageDependency>,
    subjects: &[&str],
) -> D20PackageEnvelope {
    let source_id = RuleSourceId::parse(format!("{package}-source")).unwrap();
    D20PackageEnvelope {
        domain: RuleDomainId::parse("rusty-d20").unwrap(),
        package: RulePackageId::parse(package).unwrap(),
        version: RuleVersion::new(1).unwrap(),
        dependencies,
        sources: vec![RuleSource::new(source_id.clone(), format!("content/{package}.ts")).unwrap()],
        provenance: subjects
            .iter()
            .enumerate()
            .map(|(index, subject)| {
                RuleProvenance::new(
                    RuleSubjectId::parse(*subject).unwrap(),
                    source_id.clone(),
                    Some(u64::try_from(index + 1).unwrap()),
                    Some(1),
                )
                .unwrap()
            })
            .collect(),
    }
}

fn admitted_base() -> AdmittedRulePackage {
    admit_d20_candidate(
        envelope("core", vec![], &["action:strike"]),
        base_candidate(),
    )
    .unwrap()
}

fn ruleset() -> D20Ruleset {
    D20Ruleset::compile(vec![admitted_base()]).unwrap()
}

fn character(
    entity: EntityId,
    name: &str,
    strength: i16,
    dexterity: i16,
    affinities: Vec<AffinitySeed>,
) -> CharacterSeed {
    CharacterSeed {
        entity,
        name: name.to_owned(),
        vitality: 100,
        abilities: vec![
            AbilityScore::new(id("strength"), strength),
            AbilityScore::new(id("dexterity"), dexterity),
        ],
        resources: vec![ActionResource::new(id("guard"), 2)],
        affinities,
    }
}

fn configured_session() -> D20Session {
    let mut session = D20Session::new(
        ruleset(),
        RngSeed::new(0xD20),
        vec![
            character(ATTACKER, "attacker", 30, 10, vec![]),
            character(
                TARGET,
                "target",
                10,
                10,
                vec![AffinitySeed {
                    damage_type: id("slashing"),
                    affinity: DamageAffinity::Resistant,
                }],
            ),
        ],
        vec![ArmorItemSeed {
            entity: ARMOR_ITEM,
            owner: TARGET,
            name: "chain armor".to_owned(),
            armor: id("chain"),
        }],
    )
    .unwrap();
    session
        .equip_armor(TARGET, ARMOR_ITEM, &id("chain"), operation("equip-chain"))
        .unwrap();
    session
}

#[test]
fn candidate_artifact_and_direct_construction_converge() {
    let direct = admitted_base();
    let bytes = encode_rule_package(&direct);
    let decoded = decode_canonical_rule_package(&bytes).unwrap();
    let direct_rules = D20Ruleset::compile(vec![direct]).unwrap();
    let artifact_rules = D20Ruleset::compile(vec![decoded]).unwrap();

    assert_eq!(direct_rules.fingerprint(), artifact_rules.fingerprint());
    assert_eq!(
        direct_rules.mechanics().fingerprint(),
        artifact_rules.mechanics().fingerprint()
    );
    assert_eq!(
        direct_rules.action(&id("strike")),
        artifact_rules.action(&id("strike"))
    );
}

#[test]
fn compiler_reports_correlated_invalid_content_and_package_cycles() {
    let mut invalid = base_candidate();
    invalid.actions[0].ability = id("unknown");
    let package =
        admit_d20_candidate(envelope("invalid", vec![], &["action:strike"]), invalid).unwrap();
    let error = D20Ruleset::compile(vec![package]).unwrap_err();
    let D20CompileError::Diagnostics(report) = error else {
        panic!("expected semantic diagnostics");
    };
    let diagnostic = report
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code() == "D20_UNKNOWN_ABILITY")
        .unwrap();
    assert_eq!(
        diagnostic.correlation().unwrap().source().as_str(),
        "invalid-source"
    );

    let a_dependency = RulePackageDependency::new(
        RuleDomainId::parse("rusty-d20").unwrap(),
        RulePackageId::parse("b").unwrap(),
        RuleVersion::new(1).unwrap(),
        None,
    );
    let b_dependency = RulePackageDependency::new(
        RuleDomainId::parse("rusty-d20").unwrap(),
        RulePackageId::parse("a").unwrap(),
        RuleVersion::new(1).unwrap(),
        None,
    );
    let a = admit_d20_candidate(envelope("a", vec![a_dependency], &[]), base_candidate()).unwrap();
    let b = admit_d20_candidate(
        envelope("b", vec![b_dependency], &[]),
        D20RulesCandidate {
            schema_version: D20_CANDIDATE_SCHEMA_VERSION,
            abilities: vec![],
            defenses: vec![],
            damage_types: vec![],
            resources: vec![],
            armors: vec![],
            effects: vec![],
            reactions: vec![],
            actions: vec![],
        },
    )
    .unwrap();
    assert!(matches!(
        D20Ruleset::compile(vec![a, b]),
        Err(D20CompileError::PackageSet(
            gameplay_rules::RulePackageSetError::DependencyCycle { .. }
        ))
    ));
}

#[test]
fn compiler_rejects_strict_shape_duplicates_and_d20_quotas() {
    let malformed = admit_rule_package(RulePackageCandidate::new(
        RuleDomainId::parse("rusty-d20").unwrap(),
        RulePackageId::parse("malformed").unwrap(),
        RuleVersion::new(1).unwrap(),
        vec![],
        vec![],
        vec![],
        json!({
            "schemaVersion": 1,
            "abilities": [],
            "defenses": [],
            "damageTypes": [],
            "resources": [],
            "armors": [],
            "effects": [],
            "reactions": [],
            "actions": [],
            "unexpected": true
        }),
    ))
    .unwrap();
    let D20CompileError::Diagnostics(report) = D20Ruleset::compile(vec![malformed]).unwrap_err()
    else {
        panic!("strict shape must produce diagnostics");
    };
    assert!(report
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code() == "D20_INVALID_PAYLOAD"));

    let mut oversized = base_candidate();
    oversized.abilities = (0..65)
        .map(|index| AbilityCandidate {
            id: id(&format!("ability-{index}")),
            minimum: 1,
            maximum: 30,
        })
        .collect();
    let package = admit_d20_candidate(envelope("oversized", vec![], &[]), oversized).unwrap();
    let D20CompileError::Diagnostics(report) = D20Ruleset::compile(vec![package]).unwrap_err()
    else {
        panic!("quota violation must produce diagnostics");
    };
    assert!(report
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code() == "D20_DEFINITION_QUOTA"));

    let duplicate =
        admit_d20_candidate(envelope("duplicate", vec![], &[]), base_candidate()).unwrap();
    let D20CompileError::Diagnostics(report) =
        D20Ruleset::compile(vec![admitted_base(), duplicate]).unwrap_err()
    else {
        panic!("duplicate definitions must produce diagnostics");
    };
    assert!(report
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code() == "D20_DUPLICATE_DEFINITION"));
}

#[test]
fn content_only_package_extends_the_compiled_definition_set() {
    let mut base = base_candidate();
    base.armors.clear();
    let admitted = admit_d20_candidate(envelope("core", vec![], &[]), base).unwrap();
    let dependency = RulePackageDependency::new(
        admitted.identity().domain().clone(),
        admitted.identity().package().clone(),
        admitted.identity().version(),
        Some(admitted.fingerprint().clone()),
    );
    let content = admit_d20_candidate(
        envelope("equipment", vec![dependency], &[]),
        D20RulesCandidate {
            schema_version: D20_CANDIDATE_SCHEMA_VERSION,
            abilities: vec![],
            defenses: vec![],
            damage_types: vec![],
            resources: vec![],
            armors: vec![ArmorCandidate {
                id: id("chain"),
                defense: id("armor"),
                bonus: 3,
                slot: id("body"),
            }],
            effects: vec![],
            reactions: vec![],
            actions: vec![],
        },
    )
    .unwrap();
    let rules = D20Ruleset::compile(vec![content, admitted]).unwrap();
    assert!(rules.armor(&id("chain")).is_some());
}

#[test]
fn full_action_reaction_effect_expiry_and_attribution_are_explicit() {
    assert_eq!(ability_modifier(9), -1);
    let mut session = configured_session();
    let preview = session
        .preview_action(ATTACKER, TARGET, &id("strike"), operation("strike-one"))
        .unwrap();
    assert_eq!(preview.defense().value.get(), 4);
    assert!(preview.defense().decisions.iter().any(|decision| {
        matches!(
            decision.source,
            SourceInstanceIdentity::EquippedItem {
                owner: TARGET,
                item: ARMOR_ITEM,
                ..
            }
        )
    }));
    assert_eq!(preview.reactions().len(), 1);

    let reaction = session
        .apply_reaction(&preview, &id("parry"), effect_instance("parry-strike-one"))
        .unwrap();
    assert_eq!((reaction.before, reaction.after), (2, 1));
    assert_eq!(reaction.expires_at_turn, 1);

    let before_stale = session.encode_save().unwrap();
    assert!(matches!(
        session.apply_action(ApplyActionRequest {
            preview,
            effect_instance: Some(effect_instance("bleeding-strike-one")),
        }),
        Err(D20SessionError::StalePreview { .. })
    ));
    assert_eq!(session.encode_save().unwrap(), before_stale);

    let fresh = session
        .preview_action(ATTACKER, TARGET, &id("strike"), operation("strike-one"))
        .unwrap();
    assert_eq!(fresh.defense().value.get(), 9);
    let receipt = session
        .apply_action(ApplyActionRequest {
            preview: fresh,
            effect_instance: Some(effect_instance("bleeding-strike-one")),
        })
        .unwrap();
    assert!(receipt.hit);
    let damage = receipt.damage.as_ref().unwrap();
    assert!(matches!(
        &damage.source,
        SourceInstanceIdentity::Request {
            operation,
            instance,
        } if operation.as_str() == "strike-one" && instance.as_str() == "action"
    ));
    assert!(damage.decisions.iter().any(|decision| {
        matches!(
            decision.source,
            SourceInstanceIdentity::Intrinsic { entity: TARGET, .. }
        )
    }));
    assert_eq!(
        damage.parts[0].applied.get(),
        i64::from(receipt.rolled_damage + 2) / 2
    );
    assert_eq!(receipt.expires_at_turn, Some(2));

    let turn_one = session.advance_turn(1, operation("turn-one")).unwrap();
    assert_eq!(turn_one.expired.len(), 1);
    let turn_two = session.advance_turn(2, operation("turn-two")).unwrap();
    assert_eq!(turn_two.expired.len(), 1);
}

#[test]
fn deterministic_rng_and_complete_save_reopen_continue_identically() {
    let mut session = configured_session();
    let preview = session
        .preview_action(ATTACKER, TARGET, &id("strike"), operation("strike-save"))
        .unwrap();
    session
        .apply_action(ApplyActionRequest {
            preview,
            effect_instance: Some(effect_instance("bleeding-save")),
        })
        .unwrap();
    session
        .advance_turn(2, operation("expire-before-save"))
        .unwrap();

    let encoded = session.encode_save().unwrap();
    let mut reopened = D20Session::decode_save(ruleset(), &encoded).unwrap();
    assert_eq!(reopened.encode_save().unwrap(), encoded);
    let mut control = session.clone();

    let control_preview = control
        .preview_action(
            ATTACKER,
            TARGET,
            &id("strike"),
            operation("continued-strike"),
        )
        .unwrap();
    let reopened_preview = reopened
        .preview_action(
            ATTACKER,
            TARGET,
            &id("strike"),
            operation("continued-strike"),
        )
        .unwrap();
    let control_receipt = control
        .apply_action(ApplyActionRequest {
            preview: control_preview,
            effect_instance: Some(effect_instance("continued-bleeding")),
        })
        .unwrap();
    let reopened_receipt = reopened
        .apply_action(ApplyActionRequest {
            preview: reopened_preview,
            effect_instance: Some(effect_instance("continued-bleeding")),
        })
        .unwrap();
    assert_eq!(
        (
            control_receipt.roll_index,
            control_receipt.d20,
            control_receipt.total,
            control_receipt.rolled_damage,
            control_receipt.hit,
            control_receipt.damage.as_ref().unwrap().parts.clone(),
        ),
        (
            reopened_receipt.roll_index,
            reopened_receipt.d20,
            reopened_receipt.total,
            reopened_receipt.rolled_damage,
            reopened_receipt.hit,
            reopened_receipt.damage.as_ref().unwrap().parts.clone(),
        )
    );
    assert_eq!(
        control.encode_save().unwrap(),
        reopened.encode_save().unwrap()
    );

    let mut wrong: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    wrong["rulesetFingerprint"] = json!("wrong");
    assert!(matches!(
        D20Session::decode_save(ruleset(), &serde_json::to_string(&wrong).unwrap()),
        Err(SessionSaveError::RulesetMismatch { .. })
    ));
}

#[test]
fn a_failed_late_effect_application_does_not_publish_damage_or_rng_progress() {
    let mut session = configured_session();
    let first = session
        .preview_action(ATTACKER, TARGET, &id("strike"), operation("first-effect"))
        .unwrap();
    session
        .apply_action(ApplyActionRequest {
            preview: first,
            effect_instance: Some(effect_instance("shared-bleeding")),
        })
        .unwrap();
    let before = session.encode_save().unwrap();
    let preview = session
        .preview_action(
            ATTACKER,
            TARGET,
            &id("strike"),
            operation("duplicate-effect"),
        )
        .unwrap();
    assert!(matches!(
        session.apply_action(ApplyActionRequest {
            preview,
            effect_instance: Some(effect_instance("shared-bleeding")),
        }),
        Err(D20SessionError::Mechanics(
            gameplay_mechanics::MechanicsError::DuplicateEffectInstance { .. }
        ))
    ));
    assert_eq!(session.encode_save().unwrap(), before);

    let track = session
        .entities()
        .component::<TracksComponent>(TARGET)
        .unwrap()
        .unwrap()
        .current(&TrackId::parse("vitality").unwrap())
        .unwrap();
    assert!(track.get() < 100);

    let overflow_source = configured_session();
    let mut overflow_save: serde_json::Value =
        serde_json::from_str(&overflow_source.encode_save().unwrap()).unwrap();
    overflow_save["nextRoll"] = json!(u64::MAX);
    let mut overflow_session =
        D20Session::decode_save(ruleset(), &serde_json::to_string(&overflow_save).unwrap())
            .unwrap();
    let preview = overflow_session
        .preview_action(ATTACKER, TARGET, &id("strike"), operation("overflow-roll"))
        .unwrap();
    let before = overflow_session.encode_save().unwrap();
    assert!(matches!(
        overflow_session.apply_action(ApplyActionRequest {
            preview,
            effect_instance: Some(effect_instance("overflow-effect")),
        }),
        Err(D20SessionError::RollIndexOverflow)
    ));
    assert_eq!(overflow_session.encode_save().unwrap(), before);
}
