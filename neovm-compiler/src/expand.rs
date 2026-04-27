use crate::diagnostic::Diagnostic;
use crate::surface::SurfaceForm;

#[derive(Clone, Debug, PartialEq)]
pub struct ExpandOutput {
    pub forms: Vec<SurfaceForm>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Macroexpansion boundary.
///
/// This is intentionally a no-op for the first implementation slice. It keeps
/// the pipeline shape honest while making unsupported macro expansion explicit
/// in later milestones instead of mixing expansion logic into HIR lowering.
pub fn expand_forms(forms: Vec<SurfaceForm>) -> ExpandOutput {
    ExpandOutput {
        forms,
        diagnostics: Vec::new(),
    }
}
