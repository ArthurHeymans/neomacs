use crate::diagnostic::Diagnostic;
use crate::hir::HirModule;
use crate::regir::RegFunction;
use crate::ssa::SsaFunction;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LowerOutput<T> {
    pub value: T,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn hir_to_ssa(_module: &HirModule) -> LowerOutput<SsaFunction> {
    LowerOutput {
        value: SsaFunction::default(),
        diagnostics: vec![Diagnostic::note(
            "HIR to SSA lowering is not implemented yet",
        )],
    }
}

pub fn ssa_to_regir(_function: &SsaFunction) -> LowerOutput<RegFunction> {
    LowerOutput {
        value: RegFunction::default(),
        diagnostics: vec![Diagnostic::note(
            "SSA to Register IR lowering is not implemented yet",
        )],
    }
}
