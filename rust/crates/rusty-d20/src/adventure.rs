use std::collections::{BTreeMap, BTreeSet};

use gameplay_rules::{decode_canonical_rule_package, AdmittedRulePackage, RulePackageIdentity};
use serde::Deserialize;

use crate::{D20Id, D20RulesCandidate, D20Ruleset, MAX_D20_ADVENTURE_ENTRIES};

const AUTHORED_CATALOG_SCHEMA_VERSION: u32 = 1;
const MAX_AUTHORED_CATALOG_BYTES: usize = 2_000_000;
const MAX_AUTHORED_CATALOG_PACKAGES: usize = 64;
pub const MAX_D20_SELECTABLE_ADVENTURES: usize = 16;
const BUILTIN_AUTHORED_CATALOG: &str =
    include_str!("../../../../rules/artifacts/starter/catalog.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct AuthoredCatalogArtifact {
    schema_version: u32,
    packages: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct AuthoredAdventureCatalog {
    packages: BTreeMap<RulePackageIdentity, AdmittedRulePackage>,
    adventures: BTreeMap<D20Id, AuthoredAdventureEntry>,
    default_adventure: D20Id,
}

#[derive(Debug, Clone)]
pub(crate) struct AuthoredAdventureEntry {
    pub(crate) package: RulePackageIdentity,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) details: Vec<String>,
    pub(crate) selectable: bool,
}

impl AuthoredAdventureCatalog {
    pub(crate) fn builtin() -> Result<Self, String> {
        Self::decode(BUILTIN_AUTHORED_CATALOG)
    }

    pub(crate) fn decode(input: &str) -> Result<Self, String> {
        if input.len() > MAX_AUTHORED_CATALOG_BYTES {
            return Err(format!(
                "authored catalog contains {} bytes; maximum is {MAX_AUTHORED_CATALOG_BYTES}",
                input.len()
            ));
        }
        let artifact: AuthoredCatalogArtifact =
            serde_json::from_str(input).map_err(|error| error.to_string())?;
        if artifact.schema_version != AUTHORED_CATALOG_SCHEMA_VERSION {
            return Err(format!(
                "unsupported authored catalog schema {}; expected {AUTHORED_CATALOG_SCHEMA_VERSION}",
                artifact.schema_version
            ));
        }
        if artifact.packages.is_empty() || artifact.packages.len() > MAX_AUTHORED_CATALOG_PACKAGES {
            return Err(format!(
                "authored catalog requires 1..={MAX_AUTHORED_CATALOG_PACKAGES} packages"
            ));
        }

        let mut packages = BTreeMap::new();
        let mut adventures = BTreeMap::new();
        let mut default_adventure = None;
        let mut selectable_adventures = 0_usize;
        for canonical in artifact.packages {
            let package = decode_canonical_rule_package(canonical.as_bytes())
                .map_err(|error| error.to_string())?;
            let candidate: D20RulesCandidate = serde_json::from_value(package.payload().clone())
                .map_err(|error| {
                    format!(
                        "package {} does not match the strict d20 candidate schema: {error}",
                        package.identity()
                    )
                })?;
            if let Some(previous) = packages.insert(package.identity().clone(), package.clone()) {
                return Err(format!(
                    "duplicate authored catalog package {}",
                    previous.identity()
                ));
            }
            for adventure in candidate.adventures {
                let id = adventure.id.clone();
                if adventure.start_details.len() > MAX_D20_ADVENTURE_ENTRIES {
                    return Err(format!(
                        "authored adventure {id} startDetails contains {} entries; maximum is \
                         {MAX_D20_ADVENTURE_ENTRIES}",
                        adventure.start_details.len()
                    ));
                }
                if adventure.selectable {
                    selectable_adventures = selectable_adventures
                        .checked_add(1)
                        .expect("catalog package and definition quotas fit usize");
                    if selectable_adventures > MAX_D20_SELECTABLE_ADVENTURES {
                        return Err(format!(
                            "authored catalog contains {selectable_adventures} selectable \
                             adventures; maximum is {MAX_D20_SELECTABLE_ADVENTURES}"
                        ));
                    }
                }
                if adventures
                    .insert(
                        id.clone(),
                        AuthoredAdventureEntry {
                            package: package.identity().clone(),
                            title: adventure.title,
                            summary: adventure.start_text,
                            details: adventure.start_details,
                            selectable: adventure.selectable,
                        },
                    )
                    .is_some()
                {
                    return Err(format!("duplicate authored adventure identity {}", id));
                }
                if adventure.default && default_adventure.replace(id).is_some() {
                    return Err("authored catalog contains multiple default adventures".to_owned());
                }
            }
        }
        let default_adventure = default_adventure
            .ok_or_else(|| "authored catalog has no default adventure".to_owned())?;
        if selectable_adventures == 0 {
            return Err("authored catalog has no selectable adventures".to_owned());
        }
        Ok(Self {
            packages,
            adventures,
            default_adventure,
        })
    }

    pub(crate) fn default_adventure(&self) -> &D20Id {
        &self.default_adventure
    }

    pub(crate) fn adventures(&self) -> impl Iterator<Item = (&D20Id, &AuthoredAdventureEntry)> {
        self.adventures.iter()
    }

    pub(crate) fn rules_for(&self, adventure: &D20Id) -> Result<D20Ruleset, String> {
        let entry = self
            .adventures
            .get(adventure)
            .ok_or_else(|| format!("unknown authored adventure {adventure}"))?;
        let packages = self.package_closure(&entry.package)?;
        let rules = D20Ruleset::compile(packages).map_err(|error| error.to_string())?;
        if rules.adventure(adventure).is_none() {
            return Err(format!(
                "compiled package closure does not define adventure {adventure}"
            ));
        }
        Ok(rules)
    }

    pub(crate) fn rules_for_package(&self, package: &str) -> Result<D20Ruleset, String> {
        let identity = self
            .packages
            .keys()
            .find(|identity| identity.package().as_str() == package)
            .ok_or_else(|| format!("unknown authored package {package}"))?;
        D20Ruleset::compile(self.package_closure(identity)?).map_err(|error| error.to_string())
    }

    fn package_closure(
        &self,
        root: &RulePackageIdentity,
    ) -> Result<Vec<AdmittedRulePackage>, String> {
        let mut resolved = BTreeSet::new();
        let mut visiting = BTreeSet::new();
        self.visit(root, &mut visiting, &mut resolved)?;
        Ok(resolved
            .into_iter()
            .map(|identity| {
                self.packages
                    .get(&identity)
                    .expect("resolved identities originate in the catalog")
                    .clone()
            })
            .collect())
    }

    fn visit(
        &self,
        identity: &RulePackageIdentity,
        visiting: &mut BTreeSet<RulePackageIdentity>,
        resolved: &mut BTreeSet<RulePackageIdentity>,
    ) -> Result<(), String> {
        if resolved.contains(identity) {
            return Ok(());
        }
        if !visiting.insert(identity.clone()) {
            return Err(format!(
                "authored package dependency cycle reaches {identity}"
            ));
        }
        let package = self
            .packages
            .get(identity)
            .ok_or_else(|| format!("missing authored package dependency {identity}"))?;
        for dependency in package.dependencies() {
            self.visit(dependency.identity(), visiting, resolved)?;
        }
        visiting.remove(identity);
        resolved.insert(identity.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use gameplay_rules::{encode_rule_package, RuleDomainId, RulePackageId, RuleVersion};

    use super::*;
    use crate::{
        admit_d20_candidate, AdventureCandidate, D20PackageEnvelope, D20_CANDIDATE_SCHEMA_VERSION,
    };

    #[test]
    fn builtin_catalog_compiles_selectable_paths_and_content_only_probe() {
        let catalog = AuthoredAdventureCatalog::builtin().unwrap();
        assert_eq!(catalog.default_adventure().as_str(), "wardens-gate");
        assert_eq!(
            catalog
                .adventures()
                .filter(|(_, entry)| entry.selectable)
                .map(|(id, _)| id.as_str())
                .collect::<Vec<_>>(),
            vec!["embers-wake", "wardens-gate"]
        );
        let default = catalog.rules_for(catalog.default_adventure()).unwrap();
        assert!(default
            .adventure(&D20Id::parse("wardens-gate").unwrap())
            .is_some());
        let probe = catalog
            .rules_for(&D20Id::parse("catalog-probe").unwrap())
            .unwrap();
        assert!(probe
            .adventure(&D20Id::parse("catalog-probe").unwrap())
            .is_some());
        let ember = catalog
            .rules_for(&D20Id::parse("embers-wake").unwrap())
            .unwrap();
        assert!(ember
            .adventure(&D20Id::parse("embers-wake").unwrap())
            .is_some());
    }

    #[test]
    fn catalog_projection_quotas_accept_exact_limits_and_reject_one_over() {
        let exact = catalog_json(vec![package(
            "exact",
            adventures(
                0,
                MAX_D20_SELECTABLE_ADVENTURES,
                true,
                MAX_D20_ADVENTURE_ENTRIES,
            ),
        )]);
        let catalog = AuthoredAdventureCatalog::decode(&exact).unwrap();
        assert_eq!(
            catalog
                .adventures()
                .filter(|(_, entry)| entry.selectable)
                .count(),
            MAX_D20_SELECTABLE_ADVENTURES
        );
        assert!(catalog
            .adventures()
            .all(|(_, entry)| entry.details.len() == MAX_D20_ADVENTURE_ENTRIES));

        let too_many_choices = catalog_json(vec![
            package(
                "choices-a",
                adventures(0, MAX_D20_SELECTABLE_ADVENTURES, true, 1),
            ),
            package(
                "choices-b",
                adventures(MAX_D20_SELECTABLE_ADVENTURES, 1, false, 1),
            ),
        ]);
        assert!(AuthoredAdventureCatalog::decode(&too_many_choices)
            .unwrap_err()
            .contains(&format!(
                "{} selectable adventures; maximum is {MAX_D20_SELECTABLE_ADVENTURES}",
                MAX_D20_SELECTABLE_ADVENTURES + 1
            )));

        let too_many_details = catalog_json(vec![package(
            "details",
            adventures(0, 1, true, MAX_D20_ADVENTURE_ENTRIES + 1),
        )]);
        assert!(AuthoredAdventureCatalog::decode(&too_many_details)
            .unwrap_err()
            .contains(&format!(
                "startDetails contains {} entries; maximum is {MAX_D20_ADVENTURE_ENTRIES}",
                MAX_D20_ADVENTURE_ENTRIES + 1
            )));
    }

    fn adventures(
        start: usize,
        count: usize,
        first_is_default: bool,
        details: usize,
    ) -> Vec<AdventureCandidate> {
        (start..start + count)
            .map(|index| AdventureCandidate {
                id: D20Id::parse(format!("adventure-{index}")).unwrap(),
                title: format!("Adventure {index}"),
                default: first_is_default && index == start,
                selectable: true,
                hero: D20Id::parse("hero").unwrap(),
                characters: Vec::new(),
                camp_storage: D20Id::parse("camp").unwrap(),
                storage: Vec::new(),
                items: Vec::new(),
                encounters: Vec::new(),
                start_source: "Catalog".to_owned(),
                start_text: "Choose a path.".to_owned(),
                start_details: vec!["Detail".to_owned(); details],
            })
            .collect()
    }

    fn package(name: &str, adventures: Vec<AdventureCandidate>) -> String {
        let admitted = admit_d20_candidate(
            D20PackageEnvelope {
                domain: RuleDomainId::parse("rusty-d20").unwrap(),
                package: RulePackageId::parse(name).unwrap(),
                version: RuleVersion::new(1).unwrap(),
                dependencies: Vec::new(),
                sources: Vec::new(),
                provenance: Vec::new(),
            },
            D20RulesCandidate {
                schema_version: D20_CANDIDATE_SCHEMA_VERSION,
                adventures,
                ..D20RulesCandidate::default()
            },
        )
        .unwrap();
        String::from_utf8(encode_rule_package(&admitted)).unwrap()
    }

    fn catalog_json(packages: Vec<String>) -> String {
        serde_json::json!({
            "schemaVersion": AUTHORED_CATALOG_SCHEMA_VERSION,
            "packages": packages,
        })
        .to_string()
    }
}
