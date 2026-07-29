use std::collections::{BTreeMap, BTreeSet};

use gameplay_rules::{decode_canonical_rule_package, AdmittedRulePackage, RulePackageIdentity};
use serde::Deserialize;

use crate::{D20Id, D20RulesCandidate, D20Ruleset};

const AUTHORED_CATALOG_SCHEMA_VERSION: u32 = 1;
const MAX_AUTHORED_CATALOG_BYTES: usize = 2_000_000;
const MAX_AUTHORED_CATALOG_PACKAGES: usize = 64;
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
    adventure_packages: BTreeMap<D20Id, RulePackageIdentity>,
    default_adventure: D20Id,
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
        let mut adventure_packages = BTreeMap::new();
        let mut default_adventure = None;
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
                if adventure_packages
                    .insert(adventure.id.clone(), package.identity().clone())
                    .is_some()
                {
                    return Err(format!(
                        "duplicate authored adventure identity {}",
                        adventure.id
                    ));
                }
                if adventure.default && default_adventure.replace(adventure.id.clone()).is_some() {
                    return Err("authored catalog contains multiple default adventures".to_owned());
                }
            }
        }
        let default_adventure = default_adventure
            .ok_or_else(|| "authored catalog has no default adventure".to_owned())?;
        Ok(Self {
            packages,
            adventure_packages,
            default_adventure,
        })
    }

    pub(crate) fn default_adventure(&self) -> &D20Id {
        &self.default_adventure
    }

    pub(crate) fn rules_for(&self, adventure: &D20Id) -> Result<D20Ruleset, String> {
        let package = self
            .adventure_packages
            .get(adventure)
            .ok_or_else(|| format!("unknown authored adventure {adventure}"))?;
        let packages = self.package_closure(package)?;
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
    use super::*;

    #[test]
    fn builtin_catalog_compiles_default_and_content_only_probe() {
        let catalog = AuthoredAdventureCatalog::builtin().unwrap();
        assert_eq!(catalog.default_adventure().as_str(), "wardens-gate");
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
    }
}
