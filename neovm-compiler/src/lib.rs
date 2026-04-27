//! Standalone long-term Elisp compiler pipeline for NeoVM.
//!
//! The crate currently implements the front of the pipeline:
//!
//! ```text
//! .el source -> reader -> surface AST -> expansion boundary -> HIR
//! ```
//!
//! Later milestones will fill in SSA CFG, Register IR, safepoints, and the
//! register interpreter.

pub mod ast;
pub mod diagnostic;
pub mod effects;
pub mod expand;
pub mod hir;
pub mod ids;
pub mod interp;
pub mod lower;
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
use source::{SourceFile, SourceId};
use surface::SurfaceForm;
use syntax::SyntaxTree;

/// Output from the currently implemented compiler front-end.
#[derive(Clone, Debug, PartialEq)]
pub struct CompileArtifact {
    pub source: SourceFile,
    pub syntax: SyntaxTree,
    pub surface: Vec<SurfaceForm>,
    pub hir: Option<HirModule>,
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

    let hir = if diagnostics.iter().any(Diagnostic::is_error) {
        None
    } else {
        let hir_output =
            hir::lower_expanded_forms(&source, expand_output.forms.clone(), source.lexical_binding);
        diagnostics.extend(hir_output.diagnostics);
        Some(hir_output.module)
    };

    CompileArtifact {
        source,
        syntax: reader_output.syntax,
        surface: expand_output.forms,
        hir,
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
    }

    #[test]
    fn parse_errors_prevent_hir() {
        let artifact = compile_source("bad.el", "(if x 1");

        assert!(artifact.has_errors());
        assert!(artifact.hir.is_none());
    }
}
