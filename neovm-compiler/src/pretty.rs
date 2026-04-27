use std::fmt::Write;

use crate::hir::{BindingMode, HirConst, HirDeclaration, HirExpr, HirExprKind, HirItem, HirModule};
use crate::regir::{RegFunction, RegInstKind, RegTerminator};
use crate::ssa::{SsaConst, SsaFunction, SsaInstKind, SsaTerminator, SsaValueKind};
use crate::surface::{SurfaceAtom, SurfaceForm, SurfaceKind};
use crate::syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxTree};

pub fn dump_syntax(tree: &SyntaxTree) -> String {
    let mut out = String::new();
    dump_syntax_node(&tree.root(), 0, &mut out);
    out
}

pub fn dump_surface(forms: &[SurfaceForm]) -> String {
    let mut out = String::new();
    for form in forms {
        dump_surface_form(form, 0, &mut out);
    }
    out
}

pub fn dump_hir(module: &HirModule) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "module lexical_binding={}", module.lexical_binding);
    for item in &module.items {
        match item {
            HirItem::Expr(expr) => {
                let _ = writeln!(out, "expr");
                dump_hir_expr(expr, 1, &mut out);
            }
            HirItem::Defun(defun) => {
                let _ = writeln!(out, "defun {} ({})", defun.name, defun.params.join(" "));
                dump_declarations(&defun.declarations, 1, &mut out);
                dump_hir_expr(&defun.body, 1, &mut out);
            }
        }
    }
    out
}

pub fn dump_ssa(function: &SsaFunction) -> String {
    let mut out = String::new();
    let name = function.name.as_deref().unwrap_or("<anonymous>");
    let _ = writeln!(out, "ssa {name}");
    for (block_id, block) in function.blocks.iter() {
        let params = block
            .params
            .iter()
            .map(|value| value_name(function, *value))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(out, "{block_id:?}({params}):");
        for inst in &block.instructions {
            if let Some(result) = inst.result {
                let _ = write!(out, "  {} = ", value_name(function, result));
            } else {
                let _ = write!(out, "  ");
            }
            dump_ssa_inst(&inst.kind, function, &mut out);
            let effects = inst
                .effects
                .as_slice()
                .iter()
                .map(|effect| format!("{effect:?}"))
                .collect::<Vec<_>>()
                .join(",");
            if !effects.is_empty() {
                let _ = write!(out, " ; effects={effects}");
            }
            let _ = writeln!(out);
        }
        let _ = write!(out, "  ");
        dump_terminator(&block.terminator, function, &mut out);
        let _ = writeln!(out);
    }
    out
}

pub fn dump_regir(function: &RegFunction) -> String {
    let mut out = String::new();
    let name = function.name.as_deref().unwrap_or("<anonymous>");
    let _ = writeln!(out, "regir {name}");
    for (block_id, block) in function.blocks.iter() {
        let _ = writeln!(out, "{block_id:?}:");
        for inst in &block.instructions {
            let _ = write!(out, "  ");
            dump_reg_inst(&inst.kind, function, &mut out);
            let _ = writeln!(out);
        }
        let _ = write!(out, "  ");
        dump_reg_terminator(&block.terminator, function, &mut out);
        let _ = writeln!(out);
    }
    out
}

fn dump_surface_form(form: &SurfaceForm, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    match &form.kind {
        SurfaceKind::Atom(atom) => {
            let _ = writeln!(out, "{pad}atom {}", atom_name(atom));
        }
        SurfaceKind::List(items) => {
            let _ = writeln!(out, "{pad}list");
            for item in items {
                dump_surface_form(item, indent + 1, out);
            }
        }
        SurfaceKind::DottedList(items, tail) => {
            let _ = writeln!(out, "{pad}dotted-list");
            for item in items {
                dump_surface_form(item, indent + 1, out);
            }
            let _ = writeln!(out, "{pad}  dot-tail");
            dump_surface_form(tail, indent + 2, out);
        }
        SurfaceKind::Vector(items) => {
            let _ = writeln!(out, "{pad}vector");
            for item in items {
                dump_surface_form(item, indent + 1, out);
            }
        }
        SurfaceKind::Quote(inner) => dump_prefixed_surface("quote", inner, indent, out),
        SurfaceKind::FunctionQuote(inner) => {
            dump_prefixed_surface("function-quote", inner, indent, out)
        }
        SurfaceKind::Backquote(inner) => dump_prefixed_surface("backquote", inner, indent, out),
        SurfaceKind::Comma(inner) => dump_prefixed_surface("comma", inner, indent, out),
        SurfaceKind::CommaAt(inner) => dump_prefixed_surface("comma-at", inner, indent, out),
    }
}

fn dump_prefixed_surface(name: &str, inner: &SurfaceForm, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    let _ = writeln!(out, "{pad}{name}");
    dump_surface_form(inner, indent + 1, out);
}

fn dump_syntax_node(node: &SyntaxNode, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    let _ = writeln!(out, "{pad}{:?}", node.kind());
    for child in node.children_with_tokens() {
        match child {
            SyntaxElement::Node(node) => dump_syntax_node(&node, indent + 1, out),
            SyntaxElement::Token(token) => {
                if token.kind() == SyntaxKind::Whitespace {
                    continue;
                }
                let pad = "  ".repeat(indent + 1);
                let _ = writeln!(out, "{pad}{:?} {:?}", token.kind(), token.text());
            }
        }
    }
}

fn dump_ssa_inst(kind: &SsaInstKind, function: &SsaFunction, out: &mut String) {
    match kind {
        SsaInstKind::Const(value) => {
            let _ = write!(out, "const {}", ssa_const_name(value));
        }
        SsaInstKind::Quote(_) => {
            let _ = write!(out, "quote <surface>");
        }
        SsaInstKind::FunctionQuote(_) => {
            let _ = write!(out, "function-quote <surface>");
        }
        SsaInstKind::Lambda { template, .. } => {
            let _ = write!(out, "lambda ({})", template.params.join(" "));
            if !template.captures.is_empty() {
                let _ = write!(out, " captures ({})", template.captures.join(" "));
            }
            let _ = write!(out, " <hir>");
        }
        SsaInstKind::LexicalGet(name) => {
            let _ = write!(out, "lexical-get {name}");
        }
        SsaInstKind::LexicalSet { name, value } => {
            let _ = write!(out, "lexical-set {name}, {}", value_name(function, *value));
        }
        SsaInstKind::SymbolGet(name) => {
            let _ = write!(out, "symbol-get {name}");
        }
        SsaInstKind::SymbolSet { name, value } => {
            let _ = write!(out, "symbol-set {name}, {}", value_name(function, *value));
        }
        SsaInstKind::BindLexical { name, value } => {
            let _ = write!(out, "bind-lexical {name}, {}", value_name(function, *value));
        }
        SsaInstKind::BindDynamic { name, value } => {
            let _ = write!(out, "bind-dynamic {name}, {}", value_name(function, *value));
        }
        SsaInstKind::UnbindDynamic { count } => {
            let _ = write!(out, "unbind-dynamic {count}");
        }
        SsaInstKind::DeclareSpecial(names) => {
            let _ = write!(out, "declare-special ({})", names.join(" "));
        }
        SsaInstKind::CallNamed { name, args } => {
            let _ = write!(out, "call-named {name} {}", value_names(function, args));
        }
        SsaInstKind::Funcall { callee, args } => {
            let _ = write!(
                out,
                "funcall {} {}",
                value_name(function, *callee),
                value_names(function, args)
            );
        }
        SsaInstKind::Apply { callee, args } => {
            let _ = write!(
                out,
                "apply {} {}",
                value_name(function, *callee),
                value_names(function, args)
            );
        }
        SsaInstKind::CatchBegin { tag } => {
            let _ = write!(out, "catch-begin {}", value_name(function, *tag));
        }
        SsaInstKind::CatchEnd => {
            let _ = write!(out, "catch-end");
        }
        SsaInstKind::Throw { tag, value } => {
            let _ = write!(
                out,
                "throw {}, {}",
                value_name(function, *tag),
                value_name(function, *value)
            );
        }
        SsaInstKind::ConditionCaseBegin { var } => {
            let _ = write!(
                out,
                "condition-case-begin {}",
                var.as_deref().unwrap_or("nil")
            );
        }
        SsaInstKind::ConditionCaseHandler { .. } => {
            let _ = write!(out, "condition-case-handler <pattern>");
        }
        SsaInstKind::ConditionCaseEnd => {
            let _ = write!(out, "condition-case-end");
        }
        SsaInstKind::UnwindProtectBegin => {
            let _ = write!(out, "unwind-protect-begin");
        }
        SsaInstKind::UnwindProtectCleanup => {
            let _ = write!(out, "unwind-protect-cleanup");
        }
        SsaInstKind::UnwindProtectEnd => {
            let _ = write!(out, "unwind-protect-end");
        }
    }
}

fn dump_reg_inst(kind: &RegInstKind, function: &RegFunction, out: &mut String) {
    match kind {
        RegInstKind::LoadConst { dst, value } => {
            let _ = write!(
                out,
                "{} = const {}",
                reg_name(function, *dst),
                ssa_const_name(value)
            );
        }
        RegInstKind::Quote { dst, .. } => {
            let _ = write!(out, "{} = quote <surface>", reg_name(function, *dst));
        }
        RegInstKind::FunctionQuote { dst, .. } => {
            let _ = write!(
                out,
                "{} = function-quote <surface>",
                reg_name(function, *dst)
            );
        }
        RegInstKind::Lambda {
            dst,
            template,
            captures,
        } => {
            let _ = write!(
                out,
                "{} = lambda ({})",
                reg_name(function, *dst),
                template.params.join(" ")
            );
            if !captures.is_empty() {
                let names = captures
                    .iter()
                    .map(|capture| reg_name(function, *capture))
                    .collect::<Vec<_>>()
                    .join(" ");
                let _ = write!(out, " captures ({names})");
            }
            let _ = write!(out, " <hir>");
        }
        RegInstKind::Move { dst, src } => {
            let _ = write!(
                out,
                "move {}, {}",
                reg_name(function, *dst),
                reg_name(function, *src)
            );
        }
        RegInstKind::LexicalGet { dst, name } => {
            let _ = write!(out, "{} = lexical-get {name}", reg_name(function, *dst));
        }
        RegInstKind::LexicalSet { dst, name, src } => {
            let _ = write!(
                out,
                "{} = lexical-set {name}, {}",
                reg_name(function, *dst),
                reg_name(function, *src)
            );
        }
        RegInstKind::SymbolGet { dst, name } => {
            let _ = write!(out, "{} = symbol-get {name}", reg_name(function, *dst));
        }
        RegInstKind::SymbolSet { dst, name, src } => {
            let _ = write!(
                out,
                "{} = symbol-set {name}, {}",
                reg_name(function, *dst),
                reg_name(function, *src)
            );
        }
        RegInstKind::BindLexical { name, src } => {
            let _ = write!(out, "bind-lexical {name}, {}", reg_name(function, *src));
        }
        RegInstKind::BindDynamic { name, src } => {
            let _ = write!(out, "bind-dynamic {name}, {}", reg_name(function, *src));
        }
        RegInstKind::UnbindDynamic { count } => {
            let _ = write!(out, "unbind-dynamic {count}");
        }
        RegInstKind::DeclareSpecial { names } => {
            let _ = write!(out, "declare-special ({})", names.join(" "));
        }
        RegInstKind::CallNamed { dst, name, args } => {
            let _ = write!(
                out,
                "{} = call-named {name} {}",
                reg_name(function, *dst),
                reg_names(function, args)
            );
        }
        RegInstKind::Funcall { dst, callee, args } => {
            let _ = write!(
                out,
                "{} = funcall {} {}",
                reg_name(function, *dst),
                reg_name(function, *callee),
                reg_names(function, args)
            );
        }
        RegInstKind::Apply { dst, callee, args } => {
            let _ = write!(
                out,
                "{} = apply {} {}",
                reg_name(function, *dst),
                reg_name(function, *callee),
                reg_names(function, args)
            );
        }
        RegInstKind::CatchBegin { tag } => {
            let _ = write!(out, "catch-begin {}", reg_name(function, *tag));
        }
        RegInstKind::CatchEnd => {
            let _ = write!(out, "catch-end");
        }
        RegInstKind::Throw { tag, value } => {
            let _ = write!(
                out,
                "throw {}, {}",
                reg_name(function, *tag),
                reg_name(function, *value)
            );
        }
        RegInstKind::ConditionCaseBegin { var } => {
            let _ = write!(
                out,
                "condition-case-begin {}",
                var.as_deref().unwrap_or("nil")
            );
        }
        RegInstKind::ConditionCaseHandler { .. } => {
            let _ = write!(out, "condition-case-handler <pattern>");
        }
        RegInstKind::ConditionCaseEnd => {
            let _ = write!(out, "condition-case-end");
        }
        RegInstKind::UnwindProtectBegin => {
            let _ = write!(out, "unwind-protect-begin");
        }
        RegInstKind::UnwindProtectCleanup => {
            let _ = write!(out, "unwind-protect-cleanup");
        }
        RegInstKind::UnwindProtectEnd => {
            let _ = write!(out, "unwind-protect-end");
        }
        RegInstKind::Safepoint { id } => {
            let roots = function.safepoints.entries[*id]
                .live_roots
                .iter()
                .map(|reg| reg_name(function, *reg))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = write!(out, "safepoint {id:?} roots=({roots})");
        }
    }
}

fn dump_terminator(terminator: &SsaTerminator, function: &SsaFunction, out: &mut String) {
    match terminator {
        SsaTerminator::Return(Some(value)) => {
            let _ = write!(out, "return {}", value_name(function, *value));
        }
        SsaTerminator::Return(None) => {
            let _ = write!(out, "return");
        }
        SsaTerminator::Jump { target, args } => {
            let _ = write!(out, "jump {target:?}({})", value_names(function, args));
        }
        SsaTerminator::BranchIfNil {
            test,
            then_target,
            then_args,
            else_target,
            else_args,
        } => {
            let _ = write!(
                out,
                "branch-if-nil {} then {then_target:?}({}) else {else_target:?}({})",
                value_name(function, *test),
                value_names(function, then_args),
                value_names(function, else_args)
            );
        }
        SsaTerminator::Unreachable => {
            let _ = write!(out, "unreachable");
        }
    }
}

fn dump_reg_terminator(terminator: &RegTerminator, function: &RegFunction, out: &mut String) {
    match terminator {
        RegTerminator::Return(Some(reg)) => {
            let _ = write!(out, "return {}", reg_name(function, *reg));
        }
        RegTerminator::Return(None) => {
            let _ = write!(out, "return");
        }
        RegTerminator::Jump { target } => {
            let _ = write!(out, "jump {target:?}");
        }
        RegTerminator::BranchIfNil {
            test,
            then_target,
            else_target,
        } => {
            let _ = write!(
                out,
                "branch-if-nil {} then {then_target:?} else {else_target:?}",
                reg_name(function, *test)
            );
        }
        RegTerminator::Unreachable => {
            let _ = write!(out, "unreachable");
        }
    }
}

fn value_names(function: &SsaFunction, values: &[crate::ids::ValueId]) -> String {
    values
        .iter()
        .map(|value| value_name(function, *value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn value_name(function: &SsaFunction, value: crate::ids::ValueId) -> String {
    match &function.values[value].kind {
        SsaValueKind::BlockParam {
            name: Some(name), ..
        } => format!("{value:?}.{name}"),
        _ => format!("{value:?}"),
    }
}

fn reg_names(function: &RegFunction, regs: &[crate::ids::RegId]) -> String {
    regs.iter()
        .map(|reg| reg_name(function, *reg))
        .collect::<Vec<_>>()
        .join(", ")
}

fn reg_name(function: &RegFunction, reg: crate::ids::RegId) -> String {
    match &function.registers[reg].name {
        Some(name) => format!("{reg:?}.{name}"),
        None => format!("{reg:?}"),
    }
}

fn ssa_const_name(value: &SsaConst) -> String {
    match value {
        SsaConst::Nil => "nil".to_string(),
        SsaConst::True => "t".to_string(),
        SsaConst::Int(value) => format!("int {value}"),
        SsaConst::Float(value) => format!("float {value}"),
        SsaConst::String(value) => format!("string {value:?}"),
        SsaConst::Char(value) => format!("char {value}"),
    }
}

fn atom_name(atom: &SurfaceAtom) -> String {
    match atom {
        SurfaceAtom::Nil => "nil".to_string(),
        SurfaceAtom::True => "t".to_string(),
        SurfaceAtom::Symbol(name) => format!("symbol {name}"),
        SurfaceAtom::Int(value) => format!("int {value}"),
        SurfaceAtom::Float(value) => format!("float {value}"),
        SurfaceAtom::String(value) => format!("string {value:?}"),
        SurfaceAtom::Char(value) => format!("char {value:?}"),
    }
}

fn dump_hir_expr(expr: &HirExpr, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    match &expr.kind {
        HirExprKind::Const(value) => {
            let _ = writeln!(out, "{pad}const {}", const_name(value));
        }
        HirExprKind::Quote(_) => {
            let _ = writeln!(out, "{pad}quote <surface>");
        }
        HirExprKind::FunctionQuote(_) => {
            let _ = writeln!(out, "{pad}function-quote <surface>");
        }
        HirExprKind::LexicalGet(name) => {
            let _ = writeln!(out, "{pad}lexical-get {name}");
        }
        HirExprKind::LexicalSet { name, value } => {
            let _ = writeln!(out, "{pad}lexical-set {name}");
            dump_hir_expr(value, indent + 1, out);
        }
        HirExprKind::SymbolGet(name) => {
            let _ = writeln!(out, "{pad}symbol-get {name}");
        }
        HirExprKind::SymbolSet { name, value } => {
            let _ = writeln!(out, "{pad}symbol-set {name}");
            dump_hir_expr(value, indent + 1, out);
        }
        HirExprKind::If {
            test,
            then_expr,
            else_expr,
        } => {
            let _ = writeln!(out, "{pad}if");
            dump_hir_expr(test, indent + 1, out);
            dump_hir_expr(then_expr, indent + 1, out);
            dump_hir_expr(else_expr, indent + 1, out);
        }
        HirExprKind::Progn(exprs) => {
            let _ = writeln!(out, "{pad}progn");
            for expr in exprs {
                dump_hir_expr(expr, indent + 1, out);
            }
        }
        HirExprKind::Let {
            mode,
            sequential,
            declarations,
            bindings,
            body,
        } => {
            let mode = match mode {
                BindingMode::Lexical => "lexical",
                BindingMode::Dynamic => "dynamic",
            };
            let kind = if *sequential { "let*" } else { "let" };
            let _ = writeln!(out, "{pad}{kind} mode={mode}");
            dump_declarations(declarations, indent + 1, out);
            for binding in bindings {
                let binding_mode = match binding.mode {
                    BindingMode::Lexical => "lexical",
                    BindingMode::Dynamic => "dynamic",
                };
                let _ = writeln!(out, "{pad}  bind {} mode={binding_mode}", binding.name);
                dump_hir_expr(&binding.init, indent + 2, out);
            }
            dump_hir_expr(body, indent + 1, out);
        }
        HirExprKind::Lambda {
            params,
            declarations,
            body,
        } => {
            let _ = writeln!(out, "{pad}lambda ({})", params.join(" "));
            dump_declarations(declarations, indent + 1, out);
            dump_hir_expr(body, indent + 1, out);
        }
        HirExprKind::Declare(declarations) => {
            let _ = writeln!(out, "{pad}declare");
            dump_declarations(declarations, indent + 1, out);
        }
        HirExprKind::Catch { tag, body } => {
            let _ = writeln!(out, "{pad}catch");
            dump_hir_expr(tag, indent + 1, out);
            dump_hir_expr(body, indent + 1, out);
        }
        HirExprKind::Throw { tag, value } => {
            let _ = writeln!(out, "{pad}throw");
            dump_hir_expr(tag, indent + 1, out);
            dump_hir_expr(value, indent + 1, out);
        }
        HirExprKind::ConditionCase {
            var,
            body,
            handlers,
        } => {
            let var = var.as_deref().unwrap_or("nil");
            let _ = writeln!(out, "{pad}condition-case {var}");
            dump_hir_expr(body, indent + 1, out);
            for handler in handlers {
                let _ = writeln!(out, "{pad}  handler");
                dump_hir_expr(&handler.body, indent + 2, out);
            }
        }
        HirExprKind::UnwindProtect { body, cleanup } => {
            let _ = writeln!(out, "{pad}unwind-protect");
            dump_hir_expr(body, indent + 1, out);
            dump_hir_expr(cleanup, indent + 1, out);
        }
        HirExprKind::Funcall { callee, args } => {
            let _ = writeln!(out, "{pad}funcall");
            dump_hir_expr(callee, indent + 1, out);
            for arg in args {
                dump_hir_expr(arg, indent + 1, out);
            }
        }
        HirExprKind::Apply { callee, args } => {
            let _ = writeln!(out, "{pad}apply");
            dump_hir_expr(callee, indent + 1, out);
            for arg in args {
                dump_hir_expr(arg, indent + 1, out);
            }
        }
        HirExprKind::CallNamed { name, args } => {
            let _ = writeln!(out, "{pad}call-named {name}");
            for arg in args {
                dump_hir_expr(arg, indent + 1, out);
            }
        }
        HirExprKind::CallValue { callee, args } => {
            let _ = writeln!(out, "{pad}call-value");
            dump_hir_expr(callee, indent + 1, out);
            for arg in args {
                dump_hir_expr(arg, indent + 1, out);
            }
        }
    }
}

fn dump_declarations(declarations: &[HirDeclaration], indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    for declaration in declarations {
        match declaration {
            HirDeclaration::Special(names) => {
                let _ = writeln!(out, "{pad}declare-special ({})", names.join(" "));
            }
            HirDeclaration::Unknown { name, args } => {
                let _ = writeln!(out, "{pad}declare-unknown {name} ({})", args.join(" "));
            }
        }
    }
}

fn const_name(value: &HirConst) -> String {
    match value {
        HirConst::Nil => "nil".to_string(),
        HirConst::True => "t".to_string(),
        HirConst::Int(value) => format!("int {value}"),
        HirConst::Float(value) => format!("float {value}"),
        HirConst::String(value) => format!("string {value:?}"),
        HirConst::Char(value) => format!("char {value:?}"),
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_source;
    use crate::lower::{hir_to_ssa, ssa_to_regir};
    use crate::pretty::{dump_hir, dump_regir, dump_ssa, dump_surface, dump_syntax};
    use crate::verify::{verify_regir, verify_ssa};

    #[test]
    fn dumps_hir_for_simple_defun() {
        let artifact = compile_source(
            "sample.el",
            ";;; -*- lexical-binding: t; -*-\n(defun add2 (x y) (+ x y))",
        );
        let dump = dump_hir(&artifact.hir.expect("HIR"));
        assert!(dump.contains("defun add2"));
        assert!(dump.contains("lexical-get x"));
    }

    #[test]
    fn snapshots_syntax_surface_and_hir_dumps() {
        let artifact = compile_source(
            "sample.el",
            ";;; -*- lexical-binding: t; -*-\n(defun add2 (x y) (+ x y))",
        );
        insta::assert_snapshot!(dump_syntax(&artifact.syntax), @r###"
Root
  Comment ";;; -*- lexical-binding: t; -*-"
  List
    LParen "("
    Symbol "defun"
    Symbol "add2"
    List
      LParen "("
      Symbol "x"
      Symbol "y"
      RParen ")"
    List
      LParen "("
      Symbol "+"
      Symbol "x"
      Symbol "y"
      RParen ")"
    RParen ")"
"###);
        insta::assert_snapshot!(dump_surface(&artifact.surface), @r###"
list
  atom symbol defun
  atom symbol add2
  list
    atom symbol x
    atom symbol y
  list
    atom symbol +
    atom symbol x
    atom symbol y
"###);
        insta::assert_snapshot!(dump_hir(&artifact.hir.expect("HIR")), @r###"
module lexical_binding=true
defun add2 (x y)
  call-named +
    lexical-get x
    lexical-get y
"###);
    }

    #[test]
    fn snapshots_ssa_dump_for_if() {
        let artifact = compile_source(
            "sample.el",
            ";;; -*- lexical-binding: t; -*-\n(defun choose (x y) (if x x y))",
        );
        let hir = artifact.hir.expect("HIR");
        let lowered = hir_to_ssa(&hir);
        assert_eq!(lowered.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&lowered.value), Vec::new());
        insta::assert_snapshot!(dump_ssa(&lowered.value), @r###"
ssa choose
block0(v0.x, v1.y):
  bind-lexical x, v0.x ; effects=Pure
  bind-lexical y, v1.y ; effects=Pure
  v2 = lexical-get x ; effects=ReadLexical
  branch-if-nil v2 then block2() else block1()
block1():
  v4 = lexical-get x ; effects=ReadLexical
  jump block3(v4)
block2():
  v5 = lexical-get y ; effects=ReadLexical
  jump block3(v5)
block3(v3.if.result):
  return v3.if.result
"###);
    }

    #[test]
    fn snapshots_register_ir_dump_for_if_and_call() {
        let artifact = compile_source(
            "sample.el",
            ";;; -*- lexical-binding: t; -*-\n(defun choose (x y) (if x (+ x 1) (funcall f y)))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());
        let regir = ssa_to_regir(&ssa.value);
        assert_eq!(regir.diagnostics, Vec::new());
        assert_eq!(verify_regir(&regir.value), Vec::new());
        insta::assert_snapshot!(dump_regir(&regir.value), @r###"
regir choose
rblock0:
  bind-lexical x, r0.x
  bind-lexical y, r1.y
  r2 = lexical-get x
  branch-if-nil r2 then rblock2 else rblock1
rblock1:
  r4 = lexical-get x
  r5 = const int 1
  r6 = call-named + r4, r5
  safepoint sp0 roots=(r4, r5)
  move r3.if.result, r6
  jump rblock3
rblock2:
  r7 = symbol-get f
  safepoint sp1 roots=()
  r8 = lexical-get y
  r9 = funcall r7 r8
  safepoint sp2 roots=(r7, r8)
  move r3.if.result, r9
  jump rblock3
rblock3:
  return r3.if.result
"###);
    }
}
