use gameplay_mechanics::CatalogError;
use gameplay_rules::{RuleDiagnosticError, RuleDiagnosticReport, RulePackageSetError};

#[derive(Debug)]
pub enum D20CompileError {
    PackageSet(RulePackageSetError),
    Diagnostics(RuleDiagnosticReport),
    DiagnosticContract(RuleDiagnosticError),
    MechanicsCatalog(CatalogError),
}

impl std::fmt::Display for D20CompileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "d20 rules compilation failed: {self:?}")
    }
}

impl std::error::Error for D20CompileError {}
