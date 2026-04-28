//! Standalone long-term Elisp compiler pipeline for NeoVM.
//!
//! The crate currently implements the first inspectable compiler pipeline:
//!
//! ```text
//! .el source -> reader -> surface AST -> expansion boundary -> HIR -> SSA -> RegIR
//! ```
//!
//! Cranelift is the planned native backend. Register IR remains the
//! Elisp-semantic VM IR; Cranelift lowering is an optional backend layer.

use std::path::Path;

pub mod ast;
pub mod clif;
pub mod diagnostic;
pub mod effects;
pub mod expand;
pub mod expand_eval;
pub mod expand_value;
pub mod hir;
pub mod ids;
pub mod interp;
pub mod liveness;
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
use interp::InterpResult;
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

#[derive(Clone, Debug, PartialEq)]
pub struct ExecuteArtifact {
    pub compile: CompileArtifact,
    pub result: InterpResult,
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
            let ssa_output = lower::hir_to_ssa_module(&hir_output.module);
            diagnostics.extend(ssa_output.diagnostics);
            if !diagnostics.iter().any(Diagnostic::is_error) {
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

pub fn execute_source(
    name: impl Into<String>,
    text: impl Into<String>,
    args: &[i64],
) -> ExecuteArtifact {
    let compile = compile_source(name, text);
    let mut diagnostics = compile.diagnostics.clone();
    let mut value = None;

    if !diagnostics.iter().any(Diagnostic::is_error) {
        match &compile.regir {
            Some(regir) => {
                diagnostics.extend(verify::verify_regir_module(regir));
                if !diagnostics.iter().any(Diagnostic::is_error) {
                    let result = interp::execute_module_with_args(regir, args);
                    value = result.value;
                    diagnostics.extend(result.diagnostics);
                }
            }
            None => diagnostics.push(Diagnostic::error(
                "execution requires a successfully lowered Register IR module",
            )),
        }
    }

    ExecuteArtifact {
        compile,
        result: InterpResult { value, diagnostics },
    }
}

pub fn execute_file(path: impl AsRef<Path>, args: &[i64]) -> std::io::Result<ExecuteArtifact> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path)?;
    Ok(execute_source(path.display().to_string(), text, args))
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
    fn executes_source_entry_for_runtime_free_subset() {
        let artifact = execute_source(
            "expr.el",
            ";;; -*- lexical-binding: t; -*-\n(if nil 1 2)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.as_i64(), Some(2));
    }

    #[test]
    fn execution_reports_runtime_dependent_operations() {
        let artifact = execute_source("call.el", ";;; -*- lexical-binding: t; -*-\n(foo 1 2)", &[]);
        assert!(artifact.result.value.is_none());
        assert!(artifact.result.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("unsupported Register IR interpreter operation")
        }));
    }

    #[test]
    fn e2e_recursive_fibonacci() {
        let artifact = execute_source("fib.el",
            ";;; -*- lexical-binding: t; -*-\n(defun fib (n) (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))\n(fib 10)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.as_i64(), Some(55));
    }

    #[test]
    fn e2e_list_length_and_append() {
        let artifact = execute_source("list.el", "\
;;; -*- lexical-binding: t; -*-
(defun my-length (xs) (if (null xs) 0 (+ 1 (my-length (cdr xs)))))
(defun my-append (a b) (if (null a) b (cons (car a) (my-append (cdr a) b))))
(my-length (my-append (list 1 2 3) (list 4 5)))", &[]);
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.as_i64(), Some(5));
    }

    #[test]
    fn e2e_string_concat() {
        let artifact = execute_source("str.el",
            ";;; -*- lexical-binding: t; -*-\n(concat \"hello\" \" \" \"world\")",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert!(matches!(artifact.result.value, Some(interp::RuntimeValue::Val(crate::expand_value::MacroValue::String(ref s))) if s == "hello world"));
    }

    #[test]
    fn e2e_let_star_bindings() {
        let artifact = execute_source("let.el", "\
;;; -*- lexical-binding: t; -*-
(defun test (n)
  (let* ((a (+ n 1))
         (b (* a 2))
         (c (+ a b)))
    c))
(test 5)", &[]);
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.as_i64(), Some(18));
    }

    #[test]
    fn e2e_multiple_defuns() {
        let artifact = execute_source("multi.el", "\
;;; -*- lexical-binding: t; -*-
(defun double (x) (* x 2))
(defun add-one (x) (+ x 1))
(defun compose (x) (double (add-one x)))
(compose 5)", &[]);
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.as_i64(), Some(12));
    }

    #[test]
    fn e2e_funcall_with_symbol() {
        let artifact = execute_source("funcall.el", "\
;;; -*- lexical-binding: t; -*-
(defun add1 (x) (+ x 1))
(funcall 'add1 5)", &[]);
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.as_i64(), Some(6));
    }

    #[test]
    fn e2e_nth_and_access() {
        let artifact = execute_source("nth.el", "\
;;; -*- lexical-binding: t; -*-
(defun fifth (xs) (nth 4 xs))
(fifth (list 10 20 30 40 50))", &[]);
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.as_i64(), Some(50));
    }

    #[test]
    fn e2e_progn() {
        let artifact = execute_source("progn.el", "\
;;; -*- lexical-binding: t; -*-
(progn (+ 1 2) (+ 3 4))", &[]);
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.as_i64(), Some(7));
    }

    #[test]
    fn e2e_cond_form() {
        let artifact = execute_source("cond.el", "\
;;; -*- lexical-binding: t; -*-
(defun classify (n)
  (cond ((< n 0) -1) ((= n 0) 0) (t 1)))
(list (classify (- 0 5)) (classify 0) (classify 42))", &[]);
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert!(artifact.result.value.is_some());
    }
}
