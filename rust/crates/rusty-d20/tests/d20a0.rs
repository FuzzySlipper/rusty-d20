use std::fs;
use std::path::PathBuf;

use rusty_d20::{D20CompileError, D20Id, D20Ruleset};
use rusty_engine::gameplay_rules::{decode_canonical_rule_package, AdmittedRulePackage};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactManifest {
    schema_version: u32,
    artifacts: Vec<ArtifactEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogArtifact {
    schema_version: u32,
    packages: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ArtifactEntry {
    path: String,
    domain: String,
    package: String,
    version: u64,
    fingerprint: String,
}

fn artifact_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../rules/artifacts/starter")
}

fn load(path: &str) -> AdmittedRulePackage {
    let bytes = fs::read(artifact_root().join(path)).expect("checked authoring artifact");
    decode_canonical_rule_package(&bytes).expect("strict canonical Engine package")
}

fn id(value: &str) -> D20Id {
    D20Id::parse(value).unwrap()
}

#[test]
fn generated_contract_and_artifact_manifest_match_rust_owners() {
    let generated_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../rules/packages/d20-authoring/src/generated.ts");
    assert_eq!(
        fs::read_to_string(generated_path).unwrap(),
        rusty_d20::generated_d20_candidate_typescript()
    );

    let manifest: ArtifactManifest =
        serde_json::from_slice(&fs::read(artifact_root().join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.artifacts.len(), 7);
    for entry in manifest.artifacts {
        let package = load(&entry.path);
        assert_eq!(package.identity().domain().as_str(), entry.domain);
        assert_eq!(package.identity().package().as_str(), entry.package);
        assert_eq!(package.identity().version().get(), entry.version);
        assert_eq!(package.fingerprint().as_str(), entry.fingerprint);
    }

    let catalog: CatalogArtifact =
        serde_json::from_slice(&fs::read(artifact_root().join("catalog.json")).unwrap()).unwrap();
    assert_eq!(catalog.schema_version, 1);
    assert_eq!(catalog.packages.len(), 6);
    let package_names = catalog
        .packages
        .iter()
        .map(|canonical| {
            decode_canonical_rule_package(canonical.as_bytes())
                .expect("catalog contains strict canonical Engine packages")
                .identity()
                .package()
                .as_str()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        package_names,
        [
            "starter-core",
            "steel-guard",
            "ember-ward",
            "embers-wake",
            "wardens-gate",
            "catalog-probe",
        ]
    );
}

#[test]
fn checked_artifacts_drive_distinct_node_free_rust_compositions() {
    let core = load("starter-core.json");
    let steel = load("steel-guard.json");
    let ember = load("ember-ward.json");
    let ember_adventure = load("embers-wake.json");
    let warden = load("wardens-gate.json");
    let probe = load("catalog-probe.json");

    let steel_rules = D20Ruleset::compile(vec![steel.clone(), core.clone()]).unwrap();
    assert!(steel_rules.action(&id("longsword-strike")).is_some());
    assert!(steel_rules.action(&id("precise-shot")).is_some());
    assert!(steel_rules.armor(&id("chain-armor")).is_some());
    assert!(steel_rules.reaction(&id("parry")).is_some());
    assert!(steel_rules.effect(&id("bleeding")).is_some());

    let ember_rules = D20Ruleset::compile(vec![ember.clone(), core.clone()]).unwrap();
    assert!(ember_rules.action(&id("fire-bolt")).is_some());
    assert!(ember_rules.action(&id("mind-spike")).is_some());
    assert!(ember_rules.armor(&id("runed-robe")).is_some());
    assert!(ember_rules.reaction(&id("ward-flare")).is_some());
    assert!(ember_rules.effect(&id("scorched")).is_some());

    let ember_adventure_rules =
        D20Ruleset::compile(vec![ember_adventure, ember.clone(), core.clone()]).unwrap();
    assert!(ember_adventure_rules
        .adventure(&id("embers-wake"))
        .is_some());
    assert_eq!(
        ember_adventure_rules
            .adventure(&id("embers-wake"))
            .unwrap()
            .party
            .iter()
            .map(D20Id::as_str)
            .collect::<Vec<_>>(),
        vec!["sera-vale"]
    );
    assert!(ember_adventure_rules.encounter(&id("ash-seer")).is_some());

    let combined = D20Ruleset::compile(vec![ember, core, steel]).unwrap();
    for action in [
        "fire-bolt",
        "disrupt",
        "longsword-strike",
        "mind-spike",
        "pin-in-place",
        "precise-shot",
    ] {
        assert!(combined.action(&id(action)).is_some());
    }
    assert_eq!(combined.abilities().count(), 6);
    assert_eq!(combined.defenses().count(), 4);
    assert_eq!(combined.activation_budgets().count(), 4);
    assert_eq!(combined.implements().count(), 2);
    assert_eq!(combined.damage_types().count(), 4);
    assert_eq!(combined.resources().count(), 3);
    assert_eq!(combined.reactions().count(), 2);

    let adventure_rules = D20Ruleset::compile(vec![
        probe,
        warden,
        load("steel-guard.json"),
        load("starter-core.json"),
    ])
    .expect("the content-only package composes without TypeScript at runtime");
    assert!(adventure_rules.adventure(&id("wardens-gate")).is_some());
    assert!(adventure_rules.adventure(&id("catalog-probe")).is_some());
}

#[test]
fn invalid_authored_semantics_keep_exact_source_correlation() {
    let core = load("starter-core.json");
    let invalid = load("invalid-semantics.json");
    let source_path = invalid
        .sources()
        .iter()
        .find(|source| source.id().as_str() == "invalid-semantics")
        .unwrap()
        .path()
        .to_owned();
    assert_eq!(
        source_path,
        "rules/packages/starter-ruleset/src/content/invalid.ts"
    );

    let D20CompileError::Diagnostics(report) =
        D20Ruleset::compile(vec![invalid, core]).unwrap_err()
    else {
        panic!("invalid authored semantics must produce canonical diagnostics");
    };
    assert_correlated(&report, "D20_INVALID_ABILITY_RANGE", 10);
    assert_correlated(&report, "D20_UNKNOWN_ABILITY", 14);
}

fn assert_correlated(
    report: &rusty_engine::gameplay_rules::RuleDiagnosticReport,
    code: &str,
    expected_line: u64,
) {
    let diagnostic = report
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code() == code)
        .unwrap_or_else(|| panic!("missing diagnostic {code}"));
    let correlation = diagnostic.correlation().expect("source correlation");
    assert_eq!(correlation.source().as_str(), "invalid-semantics");
    assert_eq!(correlation.line(), Some(expected_line));
    assert_eq!(correlation.column(), Some(1));
}
