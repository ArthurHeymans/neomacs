use crate::diagnostic::Diagnostic;
use crate::regir::RegFunction;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterpResult {
    pub diagnostics: Vec<Diagnostic>,
}

pub fn execute(_function: &RegFunction) -> InterpResult {
    InterpResult {
        diagnostics: vec![Diagnostic::note(
            "Register IR interpreter is not implemented yet",
        )],
    }
}
