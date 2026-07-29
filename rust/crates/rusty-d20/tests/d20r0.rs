use core_ids::EntityId;
use gameplay_mechanics::{
    EffectInstanceId, EffectMutationKind, OperationId, SourceInstanceIdentity, TrackId,
    TracksComponent,
};
use gameplay_rules::{
    admit_rule_package, decode_canonical_rule_package, encode_rule_package, AdmittedRulePackage,
    RuleDomainId, RulePackageCandidate, RulePackageDependency, RulePackageId, RuleProvenance,
    RuleSource, RuleSourceId, RuleSubjectId, RuleVersion,
};
use rusty_d20::{
    ability_modifier, admit_d20_candidate, AbilityCandidate, AbilityScore, ActionAttackCandidate,
    ActionCandidate, ActionLineOfEffectCandidate, ActionResource, ActionTargetCandidate,
    ActionTargetKindCandidate, ActionTargetTeamCandidate, ActivationBudgetCandidate,
    ActivationCostCandidate, ActivationTimingCandidate, AdventureCandidate, AffinitySeed,
    ApplyActionRequest, ArmorCandidate, ArmorItemSeed, CharacterAbilityCandidate,
    CharacterResourceCandidate, CharacterSeed, CharacterTemplateCandidate,
    ConditionClauseCandidate, D20CompileError, D20Id, D20PackageEnvelope, D20RulesCandidate,
    D20Ruleset, D20Session, D20SessionError, DamageAffinity, DamageCandidate, DamageTypeCandidate,
    DefenseCandidate, DungeonCandidate, DungeonEncounterCandidate, DungeonFacingCandidate,
    EffectCandidate, EncounterCandidate, EncounterOutcomeCandidate, EquipmentItemSeed,
    EquipmentReferenceCandidate, ImplementCandidate, ItemInstanceCandidate, ItemRarityCandidate,
    ReactionCandidate, ResourceCandidate, SessionSaveError, StorageCandidate,
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

fn dungeon(encounter: &str) -> DungeonCandidate {
    DungeonCandidate {
        title: "Test dungeon".to_owned(),
        wall_style: id("test-stone"),
        width: 5,
        height: 5,
        rows: vec![
            "#####".to_owned(),
            "#...#".to_owned(),
            "#.#.#".to_owned(),
            "#...#".to_owned(),
            "#####".to_owned(),
        ],
        start_x: 1,
        start_y: 1,
        checkpoint_x: 1,
        checkpoint_y: 1,
        start_facing: DungeonFacingCandidate::East,
        encounters: vec![DungeonEncounterCandidate {
            encounter: id(encounter),
            x: 3,
            y: 3,
        }],
        landmarks: Vec::new(),
    }
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
            abilities: vec![id("dexterity")],
        }],
        activation_budgets: vec![ActivationBudgetCandidate {
            id: id("standard-action"),
            timing: ActivationTimingCandidate::Action,
            initial: 1,
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
                conditions: vec![],
            },
            EffectCandidate {
                id: id("reaction-guard"),
                defense: Some(id("armor")),
                defense_bonus: 5,
                duration_turns: 1,
                conditions: vec![],
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
            tags: vec![id("attack")],
            activation_costs: vec![ActivationCostCandidate {
                budget: id("standard-action"),
                amount: 1,
            }],
            target: ActionTargetCandidate {
                kind: ActionTargetKindCandidate::Participant,
                team: ActionTargetTeamCandidate::Hostile,
                maximum_targets: 1,
                line_of_effect: ActionLineOfEffectCandidate::Required,
            },
            attack: ActionAttackCandidate::Fixed {
                ability: id("strength"),
                defense: id("armor"),
                damage: DamageCandidate {
                    kind: id("slashing"),
                    dice: 1,
                    sides: 8,
                    bonus: 2,
                },
                range: 1,
            },
            effect: Some(id("bleeding")),
        }],
        ..D20RulesCandidate::default()
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
fn implement_bound_actions_require_live_equipment_and_stale_after_unequip() {
    let mut candidate = base_candidate();
    candidate.implements = vec![ImplementCandidate {
        id: id("training-blade"),
        slot: id("main-hand"),
        tags: vec![id("melee"), id("weapon")],
        ability: id("strength"),
        defense: id("armor"),
        damage: DamageCandidate {
            kind: id("slashing"),
            dice: 1,
            sides: 8,
            bonus: 2,
        },
        range: 1,
    }];
    candidate.actions[0].attack = ActionAttackCandidate::Implement {
        implement: id("training-blade"),
    };
    let rules = D20Ruleset::compile(vec![admit_d20_candidate(
        envelope("implement-test", vec![], &[]),
        candidate,
    )
    .unwrap()])
    .unwrap();
    let equipment = rusty_d20::EquipmentReferenceDefinition::Implement {
        implement: id("training-blade"),
    };
    let item = EntityId::new(301);
    let mut session = D20Session::new_with_equipment_loadout(
        rules,
        RngSeed::new(0xD20),
        vec![
            character(ATTACKER, "attacker", 18, 10, vec![]),
            character(TARGET, "target", 10, 10, vec![]),
        ],
        vec![],
        vec![],
        vec![EquipmentItemSeed {
            entity: item,
            owner: ATTACKER,
            name: "training blade".to_owned(),
            equipment: equipment.clone(),
        }],
    )
    .unwrap();

    assert!(matches!(
        session.preview_action(
            ATTACKER,
            TARGET,
            &id("strike"),
            operation("missing-implement")
        ),
        Err(D20SessionError::RequiredImplementNotEquipped { .. })
    ));
    session
        .equip_item(ATTACKER, item, &equipment, operation("equip-implement"))
        .unwrap();
    let preview = session
        .preview_action(
            ATTACKER,
            TARGET,
            &id("strike"),
            operation("equipped-implement"),
        )
        .unwrap();
    session
        .unequip_item(ATTACKER, item, operation("unequip-implement"))
        .unwrap();
    assert!(matches!(
        session.apply_action(ApplyActionRequest {
            preview,
            effect_instance: Some(effect_instance("implement-effect")),
        }),
        Err(D20SessionError::StalePreview { .. })
    ));
}

#[test]
fn active_condition_clauses_penalize_and_forbid_tagged_actions() {
    let mut penalized = base_candidate();
    penalized.effects[0].conditions = vec![ConditionClauseCandidate::AttackPenalty { amount: -2 }];
    let rules = D20Ruleset::compile(vec![admit_d20_candidate(
        envelope("condition-penalty", vec![], &[]),
        penalized,
    )
    .unwrap()])
    .unwrap();
    let mut session = D20Session::new(
        rules,
        RngSeed::new(0xD20),
        vec![
            character(ATTACKER, "attacker", 30, 10, vec![]),
            character(TARGET, "target", 10, 10, vec![]),
        ],
        vec![],
    )
    .unwrap();
    let preview = session
        .preview_action(
            ATTACKER,
            TARGET,
            &id("strike"),
            operation("apply-condition"),
        )
        .unwrap();
    session
        .apply_action(ApplyActionRequest {
            preview,
            effect_instance: Some(effect_instance("condition-penalty-effect")),
        })
        .unwrap();
    let penalized_preview = session
        .preview_action(
            TARGET,
            ATTACKER,
            &id("strike"),
            operation("penalized-action"),
        )
        .unwrap();
    assert_eq!(penalized_preview.ability_modifier(), -2);

    let mut forbidden = base_candidate();
    forbidden.effects[0].conditions =
        vec![ConditionClauseCandidate::ForbidActionTag { tag: id("attack") }];
    let rules = D20Ruleset::compile(vec![admit_d20_candidate(
        envelope("condition-forbid", vec![], &[]),
        forbidden,
    )
    .unwrap()])
    .unwrap();
    let mut session = D20Session::new(
        rules,
        RngSeed::new(0xD20),
        vec![
            character(ATTACKER, "attacker", 30, 10, vec![]),
            character(TARGET, "target", 10, 10, vec![]),
        ],
        vec![],
    )
    .unwrap();
    let preview = session
        .preview_action(ATTACKER, TARGET, &id("strike"), operation("apply-forbid"))
        .unwrap();
    session
        .apply_action(ApplyActionRequest {
            preview,
            effect_instance: Some(effect_instance("condition-forbid-effect")),
        })
        .unwrap();
    assert!(matches!(
        session.preview_action(
            TARGET,
            ATTACKER,
            &id("strike"),
            operation("forbidden-action")
        ),
        Err(D20SessionError::ActionForbidden { .. })
    ));
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
    let ActionAttackCandidate::Fixed { ability, .. } = &mut invalid.actions[0].attack else {
        panic!("test action is fixed");
    };
    *ability = id("unknown");
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
            ..D20RulesCandidate::default()
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

    let mut exact_nested_limit = base_candidate();
    exact_nested_limit.actions[0].tags = (0..16).map(|index| id(&format!("tag-{index}"))).collect();
    D20Ruleset::compile(vec![admit_d20_candidate(
        envelope("exact-tags", vec![], &[]),
        exact_nested_limit,
    )
    .unwrap()])
    .unwrap();

    let mut one_over_nested_limit = base_candidate();
    one_over_nested_limit.actions[0].tags =
        (0..17).map(|index| id(&format!("tag-{index}"))).collect();
    let package = admit_d20_candidate(
        envelope("too-many-tags", vec![], &["action:strike"]),
        one_over_nested_limit,
    )
    .unwrap();
    let D20CompileError::Diagnostics(report) = D20Ruleset::compile(vec![package]).unwrap_err()
    else {
        panic!("nested quota violation must produce diagnostics");
    };
    let diagnostic = report
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code() == "D20_SEMANTIC_ENTRY_QUOTA")
        .expect("semantic quota diagnostic");
    assert_eq!(
        diagnostic.correlation().unwrap().source().as_str(),
        "too-many-tags-source"
    );

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
fn authored_adventure_failures_are_bounded_and_source_correlated() {
    let base = admitted_base();
    let dependency = RulePackageDependency::new(
        base.identity().domain().clone(),
        base.identity().package().clone(),
        base.identity().version(),
        Some(base.fingerprint().clone()),
    );
    let hero = CharacterTemplateCandidate {
        id: id("hero"),
        entity_id: 101,
        name: "Hero".to_owned(),
        title: "Tester".to_owned(),
        level: 1,
        vitality: 20,
        inventory_capacity: 2,
        abilities: vec![
            CharacterAbilityCandidate {
                ability: id("dexterity"),
                score: 10,
            },
            CharacterAbilityCandidate {
                ability: id("strength"),
                score: 10,
            },
        ],
        resources: vec![CharacterResourceCandidate {
            resource: id("guard"),
            current: 2,
        }],
        actions: vec![id("strike")],
        reactions: vec![id("parry")],
        affinities: vec![],
    };
    let outcome =
        |reward_item: Option<D20Id>, recovery_vitality: Option<u32>| EncounterOutcomeCandidate {
            title: "Outcome".to_owned(),
            summary: "Outcome summary".to_owned(),
            log_source: "Encounter".to_owned(),
            log_text: "Outcome log".to_owned(),
            log_details: vec![],
            reward_label: reward_item.as_ref().map(|_| "Reward".to_owned()),
            reward_item,
            recovery_vitality,
        };
    let mut characters = vec![id("hero")];
    characters.extend((0..64).map(|index| id(&format!("missing-{index}"))));
    let invalid = D20RulesCandidate {
        schema_version: D20_CANDIDATE_SCHEMA_VERSION,
        character_templates: vec![hero],
        storage: vec![StorageCandidate {
            id: id("camp"),
            entity_id: 101,
            name: "Camp".to_owned(),
            capacity: 4,
        }],
        item_instances: vec![
            ItemInstanceCandidate {
                id: id("orphan"),
                entity_id: 202,
                name: "Orphan armor".to_owned(),
                equipment: EquipmentReferenceCandidate::Armor { armor: id("chain") },
                owner: id("missing-owner"),
                icon: "armor".to_owned(),
                rarity: ItemRarityCandidate::Common,
                equipped: false,
            },
            ItemInstanceCandidate {
                id: id("stored-equipped"),
                entity_id: 203,
                name: "Stored armor".to_owned(),
                equipment: EquipmentReferenceCandidate::Armor { armor: id("chain") },
                owner: id("camp"),
                icon: "armor".to_owned(),
                rarity: ItemRarityCandidate::Common,
                equipped: true,
            },
        ],
        encounters: vec![EncounterCandidate {
            id: id("broken-encounter"),
            title: "Broken encounter".to_owned(),
            summary: "Missing its opponent and reward".to_owned(),
            opponent: id("missing-opponent"),
            available_from_camp: true,
            introduction_source: "Encounter".to_owned(),
            introduction_text: "A broken encounter starts.".to_owned(),
            introduction_details: vec![],
            victory: outcome(Some(id("missing-reward")), None),
            defeat: outcome(None, Some(1)),
        }],
        adventures: vec![AdventureCandidate {
            id: id("broken-adventure"),
            title: "Broken adventure".to_owned(),
            default: true,
            selectable: true,
            hero: id("hero"),
            characters,
            camp_storage: id("camp"),
            storage: vec![id("camp")],
            items: vec![id("orphan"), id("stored-equipped")],
            encounters: vec![id("broken-encounter")],
            dungeon: dungeon("broken-encounter"),
            start_source: "Adventure".to_owned(),
            start_text: "The broken adventure starts.".to_owned(),
            start_details: vec![],
        }],
        ..D20RulesCandidate::default()
    };
    let package = admit_d20_candidate(
        envelope(
            "adventure-invalid",
            vec![dependency],
            &[
                "character-template:hero",
                "storage:camp",
                "item-instance:orphan",
                "item-instance:stored-equipped",
                "encounter:broken-encounter",
                "adventure:broken-adventure",
            ],
        ),
        invalid,
    )
    .unwrap();

    let D20CompileError::Diagnostics(report) =
        D20Ruleset::compile(vec![package, base]).unwrap_err()
    else {
        panic!("invalid authored adventure must produce diagnostics");
    };
    for (code, source_line) in [
        ("D20_DUPLICATE_ENTITY_ID", 2),
        ("D20_UNKNOWN_ITEM_OWNER", 3),
        ("D20_INCOMPATIBLE_EQUIPPED_OWNER", 4),
        ("D20_UNKNOWN_ENCOUNTER_OPPONENT", 5),
        ("D20_UNKNOWN_REWARD_ITEM", 5),
        ("D20_ADVENTURE_ENTRY_QUOTA", 6),
    ] {
        let diagnostic = report
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.code() == code)
            .unwrap_or_else(|| panic!("missing diagnostic {code}"));
        let correlation = diagnostic.correlation().expect("source correlation");
        assert_eq!(correlation.source().as_str(), "adventure-invalid-source");
        assert_eq!(correlation.line(), Some(source_line));
    }
}

fn otherwise_valid_adventure_candidate(
    hero_actions: Vec<D20Id>,
    opponent_actions: Vec<D20Id>,
) -> D20RulesCandidate {
    let character = |id_value: &str, entity_id, actions| CharacterTemplateCandidate {
        id: id(id_value),
        entity_id,
        name: id_value.to_owned(),
        title: "Combatant".to_owned(),
        level: 1,
        vitality: 20,
        inventory_capacity: 2,
        abilities: vec![
            CharacterAbilityCandidate {
                ability: id("dexterity"),
                score: 10,
            },
            CharacterAbilityCandidate {
                ability: id("strength"),
                score: 10,
            },
        ],
        resources: vec![CharacterResourceCandidate {
            resource: id("guard"),
            current: 2,
        }],
        actions,
        reactions: vec![],
        affinities: vec![],
    };
    let outcome = |recovery_vitality| EncounterOutcomeCandidate {
        title: "Outcome".to_owned(),
        summary: "Outcome summary".to_owned(),
        log_source: "Encounter".to_owned(),
        log_text: "Outcome log".to_owned(),
        log_details: vec![],
        reward_item: None,
        reward_label: None,
        recovery_vitality,
    };

    D20RulesCandidate {
        schema_version: D20_CANDIDATE_SCHEMA_VERSION,
        character_templates: vec![
            character("hero", 401, hero_actions),
            character("opponent", 402, opponent_actions),
        ],
        storage: vec![StorageCandidate {
            id: id("camp"),
            entity_id: 403,
            name: "Camp".to_owned(),
            capacity: 4,
        }],
        encounters: vec![EncounterCandidate {
            id: id("duel"),
            title: "Duel".to_owned(),
            summary: "A valid duel.".to_owned(),
            opponent: id("opponent"),
            available_from_camp: true,
            introduction_source: "Encounter".to_owned(),
            introduction_text: "The duel starts.".to_owned(),
            introduction_details: vec![],
            victory: outcome(None),
            defeat: outcome(Some(1)),
        }],
        adventures: vec![AdventureCandidate {
            id: id("duel-adventure"),
            title: "Duel adventure".to_owned(),
            default: true,
            selectable: true,
            hero: id("hero"),
            characters: vec![id("hero"), id("opponent")],
            camp_storage: id("camp"),
            storage: vec![id("camp")],
            items: vec![],
            encounters: vec![id("duel")],
            dungeon: dungeon("duel"),
            start_source: "Adventure".to_owned(),
            start_text: "The adventure starts.".to_owned(),
            start_details: vec![],
        }],
        ..D20RulesCandidate::default()
    }
}

#[test]
fn authored_dungeons_reject_malformed_blocked_and_unreachable_content() {
    let compile = |package_name: &str,
                   candidate: D20RulesCandidate|
     -> gameplay_rules::RuleDiagnosticReport {
        let base = admitted_base();
        let dependency = RulePackageDependency::new(
            base.identity().domain().clone(),
            base.identity().package().clone(),
            base.identity().version(),
            Some(base.fingerprint().clone()),
        );
        let package = admit_d20_candidate(
            envelope(
                package_name,
                vec![dependency],
                &[
                    "character-template:hero",
                    "character-template:opponent",
                    "storage:camp",
                    "encounter:duel",
                    "adventure:duel-adventure",
                ],
            ),
            candidate,
        )
        .unwrap();
        let D20CompileError::Diagnostics(report) =
            D20Ruleset::compile(vec![package, base]).unwrap_err()
        else {
            panic!("invalid dungeon must fail semantic admission");
        };
        report
    };
    let valid = || otherwise_valid_adventure_candidate(vec![id("strike")], vec![id("strike")]);

    let mut malformed = valid();
    malformed.adventures[0].dungeon.rows[0] = "#...#".to_owned();
    let mut blocked_start = valid();
    blocked_start.adventures[0].dungeon.start_x = 0;
    let mut blocked_placement = valid();
    blocked_placement.adventures[0].dungeon.encounters[0].x = 2;
    blocked_placement.adventures[0].dungeon.encounters[0].y = 2;
    let mut unreachable = valid();
    unreachable.adventures[0].dungeon.rows = vec![
        "#####".to_owned(),
        "#.#.#".to_owned(),
        "###.#".to_owned(),
        "#...#".to_owned(),
        "#####".to_owned(),
    ];
    let mut excessive = valid();
    excessive.adventures[0].dungeon.width = 25;

    for (package, candidate, code) in [
        (
            "malformed-dungeon",
            malformed,
            "D20_INVALID_DUNGEON_TOPOLOGY",
        ),
        (
            "blocked-dungeon-start",
            blocked_start,
            "D20_INVALID_DUNGEON_START",
        ),
        (
            "blocked-dungeon-placement",
            blocked_placement,
            "D20_INVALID_DUNGEON_PLACEMENT",
        ),
        (
            "unreachable-dungeon",
            unreachable,
            "D20_UNREACHABLE_DUNGEON_CONTENT",
        ),
        (
            "excessive-dungeon",
            excessive,
            "D20_INVALID_DUNGEON_TOPOLOGY",
        ),
    ] {
        let report = compile(package, candidate);
        let diagnostic = report
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.code() == code)
            .unwrap_or_else(|| panic!("missing dungeon diagnostic {code} for {package}"));
        let correlation = diagnostic.correlation().expect("source correlation");
        assert_eq!(correlation.source().as_str(), format!("{package}-source"));
        assert_eq!(correlation.line(), Some(5));
    }
}

#[test]
fn adventure_combat_participants_require_actions_at_semantic_admission() {
    let compile = |package_name: &str,
                   candidate: D20RulesCandidate|
     -> gameplay_rules::RuleDiagnosticReport {
        let base = admitted_base();
        let dependency = RulePackageDependency::new(
            base.identity().domain().clone(),
            base.identity().package().clone(),
            base.identity().version(),
            Some(base.fingerprint().clone()),
        );
        let package = admit_d20_candidate(
            envelope(
                package_name,
                vec![dependency],
                &[
                    "character-template:hero",
                    "character-template:opponent",
                    "storage:camp",
                    "encounter:duel",
                    "adventure:duel-adventure",
                ],
            ),
            candidate,
        )
        .unwrap();
        let D20CompileError::Diagnostics(report) =
            D20Ruleset::compile(vec![package, base]).unwrap_err()
        else {
            panic!("actionless combat participant must fail semantic admission");
        };
        report
    };

    let report = compile(
        "actionless-opponent",
        otherwise_valid_adventure_candidate(vec![id("strike")], vec![]),
    );
    let diagnostic = report
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code() == "D20_ACTIONLESS_ENCOUNTER_OPPONENT")
        .expect("actionless opponent diagnostic");
    assert_eq!(
        diagnostic.logical_path(),
        "$/payload/encounters/duel/opponent"
    );
    let correlation = diagnostic.correlation().expect("source correlation");
    assert_eq!(correlation.source().as_str(), "actionless-opponent-source");
    assert_eq!(correlation.line(), Some(4));

    let report = compile(
        "actionless-hero",
        otherwise_valid_adventure_candidate(vec![], vec![id("strike")]),
    );
    let diagnostic = report
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code() == "D20_ACTIONLESS_ADVENTURE_HERO")
        .expect("actionless hero diagnostic");
    assert_eq!(
        diagnostic.logical_path(),
        "$/payload/adventures/duel-adventure/hero"
    );
    let correlation = diagnostic.correlation().expect("source correlation");
    assert_eq!(correlation.source().as_str(), "actionless-hero-source");
    assert_eq!(correlation.line(), Some(5));

    let mut hidden_default =
        otherwise_valid_adventure_candidate(vec![id("strike")], vec![id("strike")]);
    hidden_default.adventures[0].selectable = false;
    let report = compile("hidden-default", hidden_default);
    let diagnostic = report
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code() == "D20_INVALID_DEFAULT_ADVENTURE")
        .expect("hidden default diagnostic");
    assert_eq!(
        diagnostic.logical_path(),
        "$/payload/adventures/duel-adventure/selectable"
    );
    let correlation = diagnostic.correlation().expect("source correlation");
    assert_eq!(correlation.source().as_str(), "hidden-default-source");
    assert_eq!(correlation.line(), Some(5));
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
            ..D20RulesCandidate::default()
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
fn repeated_refresh_effect_reuses_the_engine_instance_and_reschedules_atomically() {
    let mut session = configured_session();
    let first = session
        .preview_action(
            ATTACKER,
            TARGET,
            &id("strike"),
            operation("refresh-reaction"),
        )
        .unwrap();
    session
        .apply_reaction(&first, &id("parry"), effect_instance("parry-original"))
        .unwrap();
    let second = session
        .preview_action(
            ATTACKER,
            TARGET,
            &id("strike"),
            operation("refresh-reaction"),
        )
        .unwrap();
    let refreshed = session
        .apply_reaction(&second, &id("parry"), effect_instance("parry-unused"))
        .unwrap();
    assert_eq!(refreshed.effect.kind, EffectMutationKind::Refresh);
    assert_eq!(
        refreshed
            .effect
            .current
            .as_ref()
            .unwrap()
            .instance()
            .as_str(),
        "parry-original"
    );
    let schedule = session
        .entities()
        .component::<rusty_d20::ScheduledEffectsComponent>(TARGET)
        .unwrap()
        .unwrap();
    assert_eq!(schedule.effects().len(), 1);
    assert_eq!(schedule.effects()[0].instance().as_str(), "parry-original");
    let encoded = session.encode_save().unwrap();
    assert_eq!(
        D20Session::decode_save(ruleset(), &encoded)
            .unwrap()
            .encode_save()
            .unwrap(),
        encoded
    );
}

#[test]
fn a_failed_late_missing_effect_identity_does_not_publish_damage_or_rng_progress() {
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
        .preview_action(ATTACKER, TARGET, &id("strike"), operation("missing-effect"))
        .unwrap();
    assert!(matches!(
        session.apply_action(ApplyActionRequest {
            preview,
            effect_instance: None,
        }),
        Err(D20SessionError::MissingEffectInstance(effect)) if effect.as_str() == "bleeding"
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
