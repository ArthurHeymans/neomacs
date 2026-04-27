use crate::diagnostic::Diagnostic;
use crate::hir::{HirExpr, HirExprKind, HirItem, HirModule};
use crate::regir::RegFunction;
use crate::ssa::SsaFunction;
use crate::surface::SurfaceForm;

pub fn verify_surface(_forms: &[SurfaceForm]) -> Vec<Diagnostic> {
    Vec::new()
}

pub fn verify_hir(module: &HirModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for item in &module.items {
        match item {
            HirItem::Expr(expr) => verify_hir_expr(expr, &mut diagnostics),
            HirItem::Defun(defun) => verify_hir_expr(&defun.body, &mut diagnostics),
        }
    }
    diagnostics
}

pub fn verify_ssa(_function: &SsaFunction) -> Vec<Diagnostic> {
    Vec::new()
}

pub fn verify_regir(_function: &RegFunction) -> Vec<Diagnostic> {
    Vec::new()
}

fn verify_hir_expr(expr: &HirExpr, diagnostics: &mut Vec<Diagnostic>) {
    if expr.span.end < expr.span.start {
        diagnostics.push(Diagnostic::error("HIR expression has invalid span").with_span(expr.span));
    }
    match &expr.kind {
        HirExprKind::LexicalSet { value, .. } | HirExprKind::SymbolSet { value, .. } => {
            verify_hir_expr(value, diagnostics);
        }
        HirExprKind::If {
            test,
            then_expr,
            else_expr,
        } => {
            verify_hir_expr(test, diagnostics);
            verify_hir_expr(then_expr, diagnostics);
            verify_hir_expr(else_expr, diagnostics);
        }
        HirExprKind::Progn(exprs) => {
            for expr in exprs {
                verify_hir_expr(expr, diagnostics);
            }
        }
        HirExprKind::Let { bindings, body, .. } => {
            for binding in bindings {
                verify_hir_expr(&binding.init, diagnostics);
            }
            verify_hir_expr(body, diagnostics);
        }
        HirExprKind::Lambda { body, .. } => verify_hir_expr(body, diagnostics),
        HirExprKind::Declare(_) => {}
        HirExprKind::Catch { tag, body } => {
            verify_hir_expr(tag, diagnostics);
            verify_hir_expr(body, diagnostics);
        }
        HirExprKind::Throw { tag, value } => {
            verify_hir_expr(tag, diagnostics);
            verify_hir_expr(value, diagnostics);
        }
        HirExprKind::ConditionCase { body, handlers, .. } => {
            verify_hir_expr(body, diagnostics);
            for handler in handlers {
                verify_hir_expr(&handler.body, diagnostics);
            }
        }
        HirExprKind::UnwindProtect { body, cleanup } => {
            verify_hir_expr(body, diagnostics);
            verify_hir_expr(cleanup, diagnostics);
        }
        HirExprKind::Funcall { callee, args } | HirExprKind::Apply { callee, args } => {
            verify_hir_expr(callee, diagnostics);
            for arg in args {
                verify_hir_expr(arg, diagnostics);
            }
        }
        HirExprKind::CallNamed { args, .. } => {
            for arg in args {
                verify_hir_expr(arg, diagnostics);
            }
        }
        HirExprKind::CallValue { callee, args } => {
            verify_hir_expr(callee, diagnostics);
            for arg in args {
                verify_hir_expr(arg, diagnostics);
            }
        }
        HirExprKind::Const(_)
        | HirExprKind::Quote(_)
        | HirExprKind::FunctionQuote(_)
        | HirExprKind::LexicalGet(_)
        | HirExprKind::SymbolGet(_) => {}
    }
}
