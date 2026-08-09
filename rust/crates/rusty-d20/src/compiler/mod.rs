mod collector;
mod definitions;
mod error;
mod ids;
mod mechanics;
mod validation;

use crate::D20RulesCandidate;
use rusty_engine::gameplay_rules::{
    resolve_rule_packages, AdmittedRulePackage, RuleDiagnosticReport,
};

use collector::DefinitionCollector;
pub use definitions::*;
pub use error::D20CompileError;
pub(crate) use ids::{
    armor_item_id, damage_kind_id, defense_stat_id, equipment_slot_id, implement_item_id,
    loadout_capacity_id, mechanics_effect_id, resistance_source_id, vitality_track_id,
    vulnerability_source_id,
};

pub const MAX_D20_DEFINITIONS_PER_KIND: usize = 64;
pub const MAX_D20_ADVENTURES_PER_PACKAGE: usize = 16;
pub const MAX_D20_ADVENTURE_ENTRIES: usize = 64;
pub const MAX_D20_AUTHORED_TEXT_BYTES: usize = 512;
pub const MAX_D20_DUNGEON_WIDTH: u16 = 24;
pub const MAX_D20_DUNGEON_HEIGHT: u16 = 24;
pub const MAX_D20_DUNGEON_CELLS: usize =
    MAX_D20_DUNGEON_WIDTH as usize * MAX_D20_DUNGEON_HEIGHT as usize;
pub const MAX_D20_DAMAGE_DICE: u8 = 32;
pub const MAX_D20_DAMAGE_DIE_SIDES: u16 = 1_000;
pub const MAX_D20_EFFECT_DURATION_TURNS: u16 = 10_000;
pub const MAX_D20_EXPERIENCE: u32 = 1_000_000_000;
pub const MAX_D20_ACTION_TAGS: usize = 16;
pub const MAX_D20_ACTIVATION_COSTS: usize = 4;
pub const MAX_D20_CONDITION_CLAUSES: usize = 8;
pub const MAX_D20_IMPLEMENT_TAGS: usize = 16;
pub const MAX_D20_TACTICAL_RANGE: u16 = 32;
pub const MAX_D20_FORCED_MOVEMENT: u16 = 6;
pub const MAX_D20_TACTICAL_BOARD_WIDTH: u16 = 16;
pub const MAX_D20_TACTICAL_BOARD_HEIGHT: u16 = 16;
pub const MAX_D20_TACTICAL_BOARD_CELLS: usize =
    MAX_D20_TACTICAL_BOARD_WIDTH as usize * MAX_D20_TACTICAL_BOARD_HEIGHT as usize;
pub const MAX_D20_ACTION_TARGETS: u16 = 12;
pub const MAX_D20_PARTY_MEMBERS: usize = 4;
pub const MAX_D20_ENCOUNTER_PARTICIPANTS: usize = 12;

const VITALITY_TRACK: &str = "vitality";

impl D20Ruleset {
    pub fn compile(packages: Vec<AdmittedRulePackage>) -> Result<Self, D20CompileError> {
        let packages = resolve_rule_packages(packages).map_err(D20CompileError::PackageSet)?;
        let fingerprint = packages
            .packages()
            .iter()
            .map(|package| format!("{}={}", package.identity(), package.fingerprint().as_str()))
            .collect::<Vec<_>>()
            .join("|");

        let mut collector = DefinitionCollector::default();
        for package in packages.packages() {
            let candidate: D20RulesCandidate =
                match serde_json::from_value(package.payload().clone()) {
                    Ok(candidate) => candidate,
                    Err(error) => {
                        collector.push_diagnostic(
                            package,
                            None,
                            "D20_INVALID_PAYLOAD",
                            "$/payload",
                            format!(
                                "payload does not match the strict d20 candidate schema: {error}"
                            ),
                        );
                        continue;
                    }
                };
            collector.include(package, candidate);
        }
        collector.validate_references();

        let report = RuleDiagnosticReport::new(std::mem::take(&mut collector.diagnostics))
            .map_err(D20CompileError::DiagnosticContract)?;
        if report.has_errors() {
            return Err(D20CompileError::Diagnostics(report));
        }
        collector.finish(fingerprint)
    }
}
