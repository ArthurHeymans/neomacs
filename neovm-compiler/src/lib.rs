//! Standalone long-term Elisp compiler pipeline for NeoVM.
//!
//! The crate implements the inspectable compiler pipeline:
//!
//! ```text
//! .el source -> reader -> surface AST -> expansion boundary -> HIR -> SSA -> RegIR
//! ```
//!
//! Cranelift is the planned native backend. Register IR remains the
//! Elisp-semantic VM IR; Cranelift lowering is an optional backend layer.

pub mod ast;
pub mod clif;
pub mod compile_value;
pub mod diagnostic;
pub mod effects;
pub mod expand;
pub mod expand_eval;
pub mod expand_value;
pub mod hir;
pub mod ids;
pub mod jit;
pub mod liveness;
pub mod lower;
pub mod opt;
pub mod pretty;
pub mod reader;
pub mod regir;
pub mod safepoint;
pub mod source;
pub mod ssa;
pub mod surface;
pub mod syntax;
pub mod verify;

use diagnostic::Diagnostic;
use hir::HirModule;
use regir::RegModule;
use source::{SourceFile, SourceId};
use ssa::SsaModule;
use surface::SurfaceForm;
use syntax::SyntaxTree;

/// Output from the currently implemented compiler front-end.
#[derive(Clone, Debug, PartialEq)]
pub struct CompileArtifact {
    pub source: SourceFile,
    pub syntax: SyntaxTree,
    pub surface: Vec<SurfaceForm>,
    pub hir: Option<HirModule>,
    pub ssa: Option<SsaModule>,
    pub regir: Option<RegModule>,
    pub diagnostics: Vec<Diagnostic>,
}

impl CompileArtifact {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.is_error())
    }
}

/// Compile a source string through the implemented front-end stages.
///
/// This function intentionally does not depend on `neovm-core`; all values and
/// symbols are compiler-owned representations.
pub fn compile_source(name: impl Into<String>, text: impl Into<String>) -> CompileArtifact {
    let source = SourceFile::new(SourceId::new(0), Some(name.into()), text.into());
    let reader_output = reader::read_source(&source);
    let expand_output = expand::expand_forms(reader_output.forms);

    let mut diagnostics = Vec::new();
    diagnostics.extend(reader_output.diagnostics);
    diagnostics.extend(expand_output.diagnostics);

    let mut hir = None;
    let mut ssa = None;
    let mut regir = None;

    if !diagnostics.iter().any(Diagnostic::is_error) {
        let hir_output =
            hir::lower_expanded_forms(&source, expand_output.forms.clone(), source.lexical_binding);
        diagnostics.extend(hir_output.diagnostics);
        if !diagnostics.iter().any(Diagnostic::is_error) {
            let mut ssa_output = lower::hir_to_ssa_module(&hir_output.module);
            diagnostics.extend(ssa_output.diagnostics);
            if !diagnostics.iter().any(Diagnostic::is_error) {
                opt::optimize_ssa_module(&mut ssa_output.value);
                let regir_output = lower::ssa_module_to_regir(&ssa_output.value);
                diagnostics.extend(regir_output.diagnostics);
                regir = Some(regir_output.value);
            }
            ssa = Some(ssa_output.value);
        }
        hir = Some(hir_output.module);
    }

    CompileArtifact {
        source,
        syntax: reader_output.syntax,
        surface: expand_output.forms,
        hir,
        ssa,
        regir,
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_simple_defun_to_hir() {
        let artifact = compile_source(
            "sample.el",
            ";;; -*- lexical-binding: t; -*-\n(defun add2 (x y) (+ x y))",
        );

        assert_eq!(artifact.diagnostics, Vec::new());
        assert_eq!(artifact.surface.len(), 1);
        let hir = artifact.hir.expect("HIR should be present");
        assert!(hir.lexical_binding);
        assert_eq!(hir.items.len(), 1);
        assert_eq!(artifact.ssa.expect("SSA module").functions.len(), 1);
        assert_eq!(artifact.regir.expect("RegIR module").functions.len(), 1);
    }

    #[test]
    fn parse_errors_prevent_hir() {
        let artifact = compile_source("bad.el", "(if x 1");

        assert!(artifact.has_errors());
        assert!(artifact.hir.is_none());
        assert!(artifact.ssa.is_none());
        assert!(artifact.regir.is_none());
    }

    #[test]
    fn condition_case_no_error_regir() {
        let artifact = compile_source(
            "cc.el",
            ";;; -*- lexical-binding: t; -*-\n(defun safe-div (a b) (condition-case err (/ a b) (arith-error 0)))\n(safe-div 10 3)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let ssa = artifact.ssa.expect("ssa");
        let ssa_dump = crate::pretty::dump_ssa_module(&ssa);
        println!("=== SSA ===\n{}", ssa_dump);
        let regir = artifact.regir.expect("regir");
        let dump = crate::pretty::dump_regir_module(&regir);
        println!("=== RegIR ===\n{}", dump);
    }
}
