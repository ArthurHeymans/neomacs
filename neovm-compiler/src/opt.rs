use std::collections::HashMap;

use crate::effects::{Effect, Effects};
use crate::ids::{BlockId, ValueId};
use crate::ssa::{SsaConst, SsaFunction, SsaInstKind, SsaModule, SsaTerminator};

/// Result of running an optimization pass.
pub struct OptOutput {
    pub changed: bool,
}

/// Run the default optimization pipeline on an SSA module.
pub fn optimize_ssa_module(module: &mut SsaModule) -> bool {
    let mut changed = false;
    for (_, function) in module.functions.iter_mut() {
        if optimize_ssa_function(function) {
            changed = true;
        }
    }
    changed
}

pub fn optimize_ssa_function(function: &mut SsaFunction) -> bool {
    // Verify SSA is valid before optimization (debug only)
    #[cfg(debug_assertions)]
    {
        let diags = crate::verify::verify_ssa(function);
        assert!(
            diags.is_empty(),
            "SSA is invalid before optimization: {diags:?}"
        );
    }
    let mut any_changed = false;
    let mut changed = true;
    while changed {
        changed = false;
        if constant_folding(function).changed {
            changed = true;
            any_changed = true;
        }
        if dead_code_elimination(function).changed {
            changed = true;
            any_changed = true;
        }
        if simplify_cfg(function) {
            changed = true;
            any_changed = true;
        }
        if block_merging(function).changed {
            changed = true;
            any_changed = true;
        }
        if common_subexpression_elimination(function).changed {
            changed = true;
            any_changed = true;
        }
    }
    #[cfg(debug_assertions)]
    {
        // Verify SSA after optimization.  Unreachable blocks may have
        // uses whose definitions no longer dominate, which is expected
        // after CFG simplifications remove edges.  The verifier skips
        // unreachable blocks with the `compute_reachable` helper.
        let diags = crate::verify::verify_ssa(function);
        if !diags.is_empty() {
            eprintln!("warning: optimization produced SSA with dominance issues: {diags:?}");
        }
    }
    any_changed
}

fn is_special_form_name(name: &str) -> bool {
    matches!(name,
        "and" | "or" | "if" | "cond" | "while" | "let" | "let*" | "setq"
        | "quote" | "function" | "progn" | "prog1" | "prog2"
        | "condition-case" | "unwind-protect" | "catch" | "throw"
        | "defun" | "defvar" | "defconst" | "defmacro" | "defalias"
        | "lambda" | "setf" | "interactive" | "letrec"
        | "cl-block" | "cl-return" | "cl-return-from"
    )
}

fn is_pure(effects: &Effects) -> bool {
    effects.as_slice().iter().all(|e| matches!(e, Effect::Pure))
}

fn compute_use_counts(function: &SsaFunction) -> HashMap<ValueId, u32> {
    let mut counts: HashMap<ValueId, u32> = HashMap::new();
    let bump = |counts: &mut HashMap<ValueId, u32>, v: ValueId| {
        *counts.entry(v).or_insert(0) += 1;
    };
    for block in function.blocks.values() {
        for inst in &block.instructions {
            for v in crate::liveness::inst_uses(&inst.kind) {
                bump(&mut counts, v);
            }
        }
        match &block.terminator {
            SsaTerminator::Return(Some(v)) => bump(&mut counts, *v),
            SsaTerminator::Jump { args, .. } => {
                for v in args {
                    bump(&mut counts, *v);
                }
            }
            SsaTerminator::BranchIfNil {
                test,
                then_args,
                else_args,
                ..
            } => {
                bump(&mut counts, *test);
                for v in then_args {
                    bump(&mut counts, *v);
                }
                for v in else_args {
                    bump(&mut counts, *v);
                }
            }
            _ => {}
        }
    }
    counts
}

fn apply_value_substitution(function: &mut SsaFunction, subst: &HashMap<ValueId, ValueId>) {
    if subst.is_empty() {
        return;
    }
    let remap = |v: ValueId| -> ValueId {
        let mut current = v;
        while let Some(&replacement) = subst.get(&current) {
            current = replacement;
        }
        current
    };
    for block in function.blocks.values_mut() {
        for inst in &mut block.instructions {
            remap_inst(inst, remap);
        }
        remap_terminator(&mut block.terminator, remap);
    }
}

fn remap_inst(inst: &mut crate::ssa::SsaInst, remap: impl Fn(ValueId) -> ValueId) {
    use SsaInstKind::*;
    match &mut inst.kind {
        CallNamed { args, .. } => {
            for a in args.iter_mut() {
                *a = remap(*a);
            }
        }
        Funcall { callee, args } => {
            *callee = remap(*callee);
            for a in args.iter_mut() {
                *a = remap(*a);
            }
        }
        Apply { callee, args } => {
            *callee = remap(*callee);
            for a in args.iter_mut() {
                *a = remap(*a);
            }
        }
        LexicalSet { value, .. } => {
            *value = remap(*value);
        }
        MakeLexicalCell { initial } => {
            *initial = remap(*initial);
        }
        LexicalCellGet { cell } => {
            *cell = remap(*cell);
        }
        LexicalCellSet { cell, value } => {
            *cell = remap(*cell);
            *value = remap(*value);
        }
        SymbolSet { value, .. } => {
            *value = remap(*value);
        }
        BindLexical { value, .. } => {
            *value = remap(*value);
        }
        BindDynamic { value, .. } => {
            *value = remap(*value);
        }
        CatchBegin { tag } => {
            *tag = remap(*tag);
        }
        CatchEnd { body_result } => {
            if let Some(v) = body_result {
                *v = remap(*v);
            }
        }
        Throw { tag, value } => {
            *tag = remap(*tag);
            *value = remap(*value);
        }
        ConditionCaseHandlerResult { value } => {
            *value = remap(*value);
        }
        ConditionCaseEnd { body_result } => {
            if let Some(v) = body_result {
                *v = remap(*v);
            }
        }
        UnwindProtectEnd { body_result } => {
            if let Some(v) = body_result {
                *v = remap(*v);
            }
        }
        Lambda { captures, .. } => {
            for c in captures.iter_mut() {
                *c = remap(*c);
            }
        }
        _ => {}
    }
}

fn remap_terminator(term: &mut SsaTerminator, remap: impl Fn(ValueId) -> ValueId) {
    match term {
        SsaTerminator::Return(Some(v)) => *v = remap(*v),
        SsaTerminator::Jump { args, .. } => {
            for a in args.iter_mut() {
                *a = remap(*a);
            }
        }
        SsaTerminator::BranchIfNil {
            test,
            then_args,
            else_args,
            ..
        } => {
            *test = remap(*test);
            for a in then_args.iter_mut() {
                *a = remap(*a);
            }
            for a in else_args.iter_mut() {
                *a = remap(*a);
            }
        }
        _ => {}
    }
}

/// Remove blocks that are no longer reachable after branch folding.
fn simplify_cfg(function: &mut SsaFunction) -> bool {
    let preds = crate::verify::predecessor_map(function);
    let mut changed = false;
    for (bid, block) in function.blocks.iter_mut() {
        if Some(bid) == function.entry {
            continue;
        }
        let n_preds = preds.get(&bid).map_or(0, |p| p.len());
        if n_preds == 0 && !matches!(block.terminator, SsaTerminator::Unreachable) {
            // Don't clear blocks that contain exception handler setup instructions
            // (CatchEnd, ConditionCaseEnd, etc.) — the CLIF codegen needs these
            // to populate handler blocks even if the SSA block is unreachable,
            // because the handler can still be reached via throw/signal.
            let has_exception_setup = block.instructions.iter().any(|inst| {
                matches!(
                    inst.kind,
                    SsaInstKind::CatchEnd { .. }
                        | SsaInstKind::ConditionCaseEnd { .. }
                        | SsaInstKind::UnwindProtectEnd { .. }
                )
            });
            if !has_exception_setup {
                block.instructions.clear();
                block.terminator = SsaTerminator::Unreachable;
                changed = true;
            }
        }
    }
    changed
}

fn const_to_bool(c: &SsaConst) -> Option<bool> {
    match c {
        SsaConst::Nil => Some(false),
        _ => Some(true),
    }
}

pub fn constant_folding(function: &mut SsaFunction) -> OptOutput {
    let mut const_map: HashMap<ValueId, SsaConst> = HashMap::new();
    let subst: HashMap<ValueId, ValueId> = HashMap::new();
    let mut changed = false;

    // Collect all existing constants.
    for (_block_id, block) in function.blocks.iter() {
        for inst in block.instructions.iter() {
            if let SsaInstKind::Const(c) = &inst.kind
                && let Some(result) = inst.result {
                    const_map.insert(result, c.clone());
                }
        }
    }

    // Fold BranchIfNil where test is a known constant.
    for (_bid, block) in function.blocks.iter_mut() {
        let test_val = match &block.terminator {
            SsaTerminator::BranchIfNil { test, .. } => *test,
            _ => continue,
        };
        let resolved = subst.get(&test_val).copied().unwrap_or(test_val);
        let c = match const_map.get(&resolved) {
            Some(c) => c,
            None => continue,
        };
        let is_nil = matches!(c, SsaConst::Nil);
        if let SsaTerminator::BranchIfNil {
                then_target,
                then_args,
                else_target,
                else_args,
                ..
            } = &block.terminator {
            let (target, args) = if is_nil {
                (*then_target, then_args.clone())
            } else {
                (*else_target, else_args.clone())
            };
            block.terminator = SsaTerminator::Jump { target, args };
            changed = true;
        }
    }

    // Fold instructions with all-constant operands.
    for (_bid, block) in function.blocks.iter_mut() {
        for inst in block.instructions.iter_mut() {
            let Some(result) = inst.result else { continue };
            if const_map.contains_key(&result) {
                continue;
            }
            if let Some(folded) = try_fold_inst(&inst.kind, &const_map) {
                const_map.insert(result, folded.clone());
                inst.kind = SsaInstKind::Const(folded);
                inst.effects = Effects::pure();
                changed = true;
            }
        }
    }

    // Apply any pending substitutions.
    if !subst.is_empty() {
        apply_value_substitution(function, &subst);
    }

    OptOutput { changed }
}

fn try_fold_inst(kind: &SsaInstKind, const_map: &HashMap<ValueId, SsaConst>) -> Option<SsaConst> {
    match kind {
        SsaInstKind::CallNamed { name, args } => {
            let const_args: Vec<&SsaConst> = args
                .iter()
                .map(|a| const_map.get(a))
                .collect::<Option<Vec<_>>>()?;
            try_fold_call_named(name, &const_args)
        }
        _ => None,
    }
}

fn fold_cmp(a: &SsaConst, b: &SsaConst, cmp: impl FnOnce(std::cmp::Ordering) -> bool) -> Option<SsaConst> {
    let ordering = match (a, b) {
        (SsaConst::Int(a), SsaConst::Int(b)) => a.cmp(b),
        (SsaConst::Char(a), SsaConst::Char(b)) => a.cmp(b),
        (SsaConst::Int(a), SsaConst::Char(b)) => a.cmp(b),
        (SsaConst::Char(a), SsaConst::Int(b)) => a.cmp(b),
        (SsaConst::Float(a), SsaConst::Float(b)) => f64::total_cmp(a, b),
        (SsaConst::Int(a), SsaConst::Float(b)) => f64::total_cmp(&(*a as f64), b),
        (SsaConst::Float(a), SsaConst::Int(b)) => f64::total_cmp(a, &(*b as f64)),
        _ => return None,
    };
    Some(if cmp(ordering) { SsaConst::True } else { SsaConst::Nil })
}

fn fold_binary_arith(
    a: &SsaConst,
    b: &SsaConst,
    int_op: impl FnOnce(i64, i64) -> i64,
    float_op: impl FnOnce(f64, f64) -> f64,
) -> Option<SsaConst> {
    let a_int = a.as_int();
    let b_int = b.as_int();
    let a_float = match a { SsaConst::Float(f) => Some(*f), _ => None };
    let b_float = match b { SsaConst::Float(f) => Some(*f), _ => None };
    match (a_int, b_int, a_float, b_float) {
        (Some(a), Some(b), _, _) => Some(SsaConst::Int(int_op(a, b))),
        (_, _, Some(a), Some(b)) => Some(SsaConst::Float(float_op(a, b))),
        (Some(a), _, _, Some(b)) => Some(SsaConst::Float(float_op(a as f64, b))),
        (_, Some(b), Some(a), _) => Some(SsaConst::Float(float_op(a, b as f64))),
        _ => None,
    }
}

fn try_fold_call_named(name: &str, args: &[&SsaConst]) -> Option<SsaConst> {
    match name {
        "+" if args.len() == 2 => fold_binary_arith(args[0], args[1],
            |a, b| a.wrapping_add(b), |a, b| a + b),
        "-" if args.len() == 2 => fold_binary_arith(args[0], args[1],
            |a, b| a.wrapping_sub(b), |a, b| a - b),
        "-" if args.len() == 1 => match args[0] {
            SsaConst::Int(a) => Some(SsaConst::Int(a.wrapping_neg())),
            SsaConst::Float(f) => Some(SsaConst::Float(-f)),
            _ => None,
        },
        "*" if args.len() == 2 => fold_binary_arith(args[0], args[1],
            |a, b| a.wrapping_mul(b), |a, b| a * b),
        "/" if args.len() == 2 => {
            if let (SsaConst::Int(a), SsaConst::Int(b)) = (args[0], args[1]) {
                if *b == 0 { return None; }
                Some(SsaConst::Int(a.wrapping_div(*b)))
            } else if let (SsaConst::Float(a), SsaConst::Float(b)) = (args[0], args[1]) {
                if *b == 0.0 { return None; }
                Some(SsaConst::Float(a / b))
            } else { None }
        }
        "=" if args.len() == 1 => Some(SsaConst::True),
        "=" if args.len() == 2 => fold_cmp(args[0], args[1], |o| o.is_eq()),
        "<" if args.len() == 1 => Some(SsaConst::True),
        "<" if args.len() == 2 => fold_cmp(args[0], args[1], |o| o.is_lt()),
        ">" if args.len() == 1 => Some(SsaConst::True),
        ">" if args.len() == 2 => fold_cmp(args[0], args[1], |o| o.is_gt()),
        "<=" if args.len() == 1 => Some(SsaConst::True),
        "<=" if args.len() == 2 => fold_cmp(args[0], args[1], |o| o.is_le()),
        ">=" if args.len() == 1 => Some(SsaConst::True),
        ">=" if args.len() == 2 => fold_cmp(args[0], args[1], |o| o.is_ge()),
        "eq" | "eql" if args.len() == 2 => match (args[0], args[1]) {
            (SsaConst::Int(a), SsaConst::Int(b)) => Some(if a == b {
                SsaConst::True
            } else {
                SsaConst::Nil
            }),
            (SsaConst::Symbol(a), SsaConst::Symbol(b)) => Some(if a == b {
                SsaConst::True
            } else {
                SsaConst::Nil
            }),
            (SsaConst::Char(a), SsaConst::Char(b)) => Some(if a == b {
                SsaConst::True
            } else {
                SsaConst::Nil
            }),
            (SsaConst::Nil, SsaConst::Nil) | (SsaConst::True, SsaConst::True) => Some(SsaConst::True),
            (SsaConst::Float(a), SsaConst::Float(b)) => {
                Some(if a.to_bits() == b.to_bits() { SsaConst::True } else { SsaConst::Nil })
            }
            // Different types are never eq.
            (SsaConst::Nil, _) | (SsaConst::True, _)
            | (_, SsaConst::Nil) | (_, SsaConst::True)
            | (SsaConst::Int(_), _) | (SsaConst::Float(_), _)
            | (SsaConst::Char(_), _) | (SsaConst::Symbol(_), _) => Some(SsaConst::Nil),
            _ => None,
        },
        "equal" if args.len() == 2 => match (args[0], args[1]) {
            (SsaConst::Int(a), SsaConst::Int(b)) => Some(if a == b { SsaConst::True } else { SsaConst::Nil }),
            (SsaConst::Float(a), SsaConst::Float(b)) => Some(if a == b { SsaConst::True } else { SsaConst::Nil }),
            (SsaConst::String(a), SsaConst::String(b)) => Some(if a == b { SsaConst::True } else { SsaConst::Nil }),
            (SsaConst::Nil, SsaConst::Nil) | (SsaConst::True, SsaConst::True) => Some(SsaConst::True),
            (SsaConst::Symbol(a), SsaConst::Symbol(b)) => Some(if a == b { SsaConst::True } else { SsaConst::Nil }),
            // Different types are never equal.
            (SsaConst::Nil, _) | (SsaConst::True, _)
            | (_, SsaConst::Nil) | (_, SsaConst::True) => Some(SsaConst::Nil),
            (SsaConst::Int(_), _) | (SsaConst::Float(_), _)
            | (SsaConst::String(_), _) | (SsaConst::Symbol(_), _) => Some(SsaConst::Nil),
            _ => None,
        },
        "1+" if args.len() == 1 => match args[0] {
            SsaConst::Int(a) => Some(SsaConst::Int(a.wrapping_add(1))),
            SsaConst::Float(f) => Some(SsaConst::Float(f + 1.0)),
            SsaConst::Char(c) => Some(SsaConst::Int(c + 1)),
            _ => None,
        },
        "1-" if args.len() == 1 => match args[0] {
            SsaConst::Int(a) => Some(SsaConst::Int(a.wrapping_sub(1))),
            SsaConst::Float(f) => Some(SsaConst::Float(f - 1.0)),
            SsaConst::Char(c) => Some(SsaConst::Int(c - 1)),
            _ => None,
        },
        "null" | "not" if args.len() == 1 => match const_to_bool(args[0]) {
            Some(false) => Some(SsaConst::True),
            Some(true) => Some(SsaConst::Nil),
            None => None,
        },
        "/=" if args.len() == 1 => Some(SsaConst::True),
        "/=" if args.len() >= 2 => {
            let ints: Vec<i64> = args.iter().map(|a| a.as_int()).collect::<Option<Vec<_>>>()?;
            let mut seen = std::collections::HashSet::new();
            let all_distinct = ints.iter().all(|i| seen.insert(*i));
            Some(if all_distinct { SsaConst::True } else { SsaConst::Nil })
        }
        "max" if !args.is_empty() => {
            if let Some(ints) = args.iter().map(|a| a.as_int()).collect::<Option<Vec<_>>>() {
                return Some(SsaConst::Int(ints.into_iter().max().unwrap()));
            }
            let floats: Vec<f64> = args.iter().map(|a| to_f64(a)).collect::<Option<Vec<_>>>()?;
            Some(SsaConst::Float(floats.into_iter().fold(f64::NEG_INFINITY, f64::max)))
        }
        "min" if !args.is_empty() => {
            if let Some(ints) = args.iter().map(|a| a.as_int()).collect::<Option<Vec<_>>>() {
                return Some(SsaConst::Int(ints.into_iter().min().unwrap()));
            }
            let floats: Vec<f64> = args.iter().map(|a| to_f64(a)).collect::<Option<Vec<_>>>()?;
            Some(SsaConst::Float(floats.into_iter().fold(f64::INFINITY, f64::min)))
        }
        "+" if args.is_empty() => Some(SsaConst::Int(0)),
        "+" if args.len() == 1 => match args[0] {
            SsaConst::Int(n) => Some(SsaConst::Int(*n)),
            SsaConst::Float(f) => Some(SsaConst::Float(*f)),
            SsaConst::Char(c) => Some(SsaConst::Int(*c)),
            _ => None,
        },
        "+" if args.is_empty() => Some(SsaConst::Int(0)),
        "+" if args.len() == 1 => match args[0] {
            SsaConst::Int(n) => Some(SsaConst::Int(*n)),
            SsaConst::Float(f) => Some(SsaConst::Float(*f)),
            SsaConst::Char(c) => Some(SsaConst::Int(*c)),
            _ => None,
        },
        "+" if args.len() >= 2 => {
            if let Some(ints) = args.iter().map(|a| a.as_int()).collect::<Option<Vec<i64>>>() {
                return Some(SsaConst::Int(ints.into_iter().sum()));
            }
            if let Some(floats) = args.iter().map(|a| to_f64(a)).collect::<Option<Vec<f64>>>() {
                return Some(SsaConst::Float(floats.into_iter().sum()));
            }
            None
        }
        "*" if args.is_empty() => Some(SsaConst::Int(1)),
        "*" if args.len() == 1 => match args[0] {
            SsaConst::Int(n) => Some(SsaConst::Int(*n)),
            SsaConst::Float(f) => Some(SsaConst::Float(*f)),
            SsaConst::Char(c) => Some(SsaConst::Int(*c)),
            _ => None,
        },
        "*" if args.len() >= 2 => {
            if let Some(ints) = args.iter().map(|a| a.as_int()).collect::<Option<Vec<i64>>>() {
                return Some(SsaConst::Int(ints.into_iter().product()));
            }
            if let Some(floats) = args.iter().map(|a| to_f64(a)).collect::<Option<Vec<f64>>>() {
                return Some(SsaConst::Float(floats.into_iter().product()));
            }
            None
        }
        "logand" => {
            let ints: Vec<i64> = args.iter().map(|a| a.as_int()).collect::<Option<Vec<_>>>()?;
            Some(SsaConst::Int(ints.into_iter().fold(!0i64, |a, b| a & b)))
        }
        "logior" => {
            let ints: Vec<i64> = args.iter().map(|a| a.as_int()).collect::<Option<Vec<_>>>()?;
            Some(SsaConst::Int(ints.into_iter().fold(0, |a, b| a | b)))
        }
        "logxor" => {
            let ints: Vec<i64> = args.iter().map(|a| a.as_int()).collect::<Option<Vec<_>>>()?;
            Some(SsaConst::Int(ints.into_iter().fold(0, |a, b| a ^ b)))
        }
        "ash" if args.len() == 2 => {
            let value = args[0].as_int()?;
            let count = args[1].as_int()?;
            if count >= 64 {
                Some(SsaConst::Int(0))
            } else if count >= 0 {
                Some(SsaConst::Int(value.wrapping_shl(count as u32)))
            } else if count <= -64 {
                Some(SsaConst::Int(if value < 0 { -1 } else { 0 }))
            } else {
                Some(SsaConst::Int(value >> (-count)))
            }
        }
        "string-trim" if args.len() >= 1 => match args[0] {
            SsaConst::String(s) => {
                let chars: &[char] = &[' ', '\t', '\n', '\r'];
                let trimmed = if let Some(SsaConst::String(trim_chars)) = args.get(1) {
                    let set: Vec<char> = trim_chars.chars().collect();
                    s.trim_matches(set.as_slice()).to_string()
                } else {
                    s.trim_matches(chars).to_string()
                };
                Some(SsaConst::String(trimmed))
            }
            _ => None,
        },
        "string-trim-left" if args.len() >= 1 => match args[0] {
            SsaConst::String(s) => {
                let chars: &[char] = &[' ', '\t', '\n', '\r'];
                let trimmed = if let Some(SsaConst::String(trim_chars)) = args.get(1) {
                    let set: Vec<char> = trim_chars.chars().collect();
                    s.trim_start_matches(set.as_slice()).to_string()
                } else {
                    s.trim_start_matches(chars).to_string()
                };
                Some(SsaConst::String(trimmed))
            }
            _ => None,
        },
        "string-trim-right" if args.len() >= 1 => match args[0] {
            SsaConst::String(s) => {
                let chars: &[char] = &[' ', '\t', '\n', '\r'];
                let trimmed = if let Some(SsaConst::String(trim_chars)) = args.get(1) {
                    let set: Vec<char> = trim_chars.chars().collect();
                    s.trim_end_matches(set.as_slice()).to_string()
                } else {
                    s.trim_end_matches(chars).to_string()
                };
                Some(SsaConst::String(trimmed))
            }
            _ => None,
        },
        "downcase" if args.len() == 1 => match args[0] {
            SsaConst::String(s) if s.is_ascii() => Some(SsaConst::String(s.to_ascii_lowercase())),
            _ => None,
        },
        "upcase" if args.len() == 1 => match args[0] {
            SsaConst::String(s) if s.is_ascii() => Some(SsaConst::String(s.to_ascii_uppercase())),
            _ => None,
        },
        "capitalize" if args.len() == 1 => match args[0] {
            SsaConst::String(s) if s.is_ascii() => {
                let mut c = s.chars();
                let capitalized: String = match c.next() {
                    Some(ch) => ch.to_ascii_uppercase().to_string() + c.as_str(),
                    None => String::new(),
                };
                Some(SsaConst::String(capitalized))
            }
            _ => None,
        },
        "length" | "safe-length" if args.len() == 1 => match args[0] {
            SsaConst::String(s) => Some(SsaConst::Int(s.len() as i64)),
            SsaConst::Nil => Some(SsaConst::Int(0)),
            _ => None,
        },
        "copy-list" if args.len() == 1 => match args[0] {
            SsaConst::Nil => Some(SsaConst::Nil),
            _ => None,
        },
        "proper-list-p" if args.len() == 1 => match args[0] {
            SsaConst::Nil => Some(SsaConst::Int(0)),
            _ => None,
        },
        "length=" if args.len() == 2 => match (args[0], args[1].as_int()) {
            (SsaConst::String(s), Some(n)) => {
                Some(if s.len() as i64 == n { SsaConst::True } else { SsaConst::Nil })
            }
            (SsaConst::Nil, Some(n)) => Some(if 0 == n { SsaConst::True } else { SsaConst::Nil }),
            _ => None,
        },
        "length<" if args.len() == 2 => match (args[0], args[1].as_int()) {
            (SsaConst::String(s), Some(n)) => {
                Some(if (s.len() as i64) < n { SsaConst::True } else { SsaConst::Nil })
            }
            (SsaConst::Nil, Some(n)) => Some(if 0 < n { SsaConst::True } else { SsaConst::Nil }),
            _ => None,
        },
        "length>" if args.len() == 2 => match (args[0], args[1].as_int()) {
            (SsaConst::String(s), Some(n)) => {
                Some(if (s.len() as i64) > n { SsaConst::True } else { SsaConst::Nil })
            }
            (SsaConst::Nil, Some(n)) => Some(if 0 > n { SsaConst::True } else { SsaConst::Nil }),
            _ => None,
        },
        "string-bytes" if args.len() == 1 => match args[0] {
            SsaConst::String(s) => Some(SsaConst::Int(s.len() as i64)),
            _ => None,
        },
        "string-width" if args.len() == 1 => match args[0] {
            SsaConst::String(s) if s.is_ascii() => Some(SsaConst::Int(s.len() as i64)),
            _ => None,
        },
        "float" if args.len() == 1 => match args[0] {
            SsaConst::Int(n) => Some(SsaConst::Float(*n as f64)),
            SsaConst::Float(f) => Some(SsaConst::Float(*f)),
            _ => None,
        },
        "number-to-string" if args.len() >= 1 => {
            let n = args[0].as_int()?;
            let base = args.get(1).map(|b| b.as_int().unwrap_or(10)).unwrap_or(10);
            if base == 10 {
                return Some(SsaConst::String(n.to_string()));
            }
            if (2..=36).contains(&base) {
                // Use manual radix conversion (Rust std doesn't have i64::to_str_radix)
                let mut result = String::new();
                let mut remaining = n.abs();
                let neg = n < 0;
                let base = base as u32;
                let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
                if remaining == 0 {
                    result.push('0');
                } else {
                    while remaining > 0 {
                        let digit = (remaining % base as i64) as usize;
                        result.push(digits[digit] as char);
                        remaining /= base as i64;
                    }
                    if neg { result.push('-'); }
                    result = result.chars().rev().collect();
                }
                return Some(SsaConst::String(result));
            }
            None
        },
        "substring" | "substring-no-properties" if args.len() >= 2 => {
            let s = match args[0] { SsaConst::String(s) => s.as_str(), _ => return None };
            let start = args[1].as_int()? as usize;
            let end = if args.len() >= 3 {
                args[2].as_int()? as usize
            } else {
                s.len()
            };
            if start > end || start > s.len() || end > s.len() { return None; }
            Some(SsaConst::String(s[start..end].to_string()))
        }
        "aref" if args.len() == 2 => {
            let index = args[1].as_int()? as usize;
            match args[0] {
                SsaConst::String(s) => s.chars().nth(index).map(|c| SsaConst::Char(c as i64)),
                _ => None,
            }
        },
        "elt" if args.len() == 2 => {
            let index = args[1].as_int()? as usize;
            match args[0] {
                SsaConst::String(s) => s.chars().nth(index).map(|c| SsaConst::Char(c as i64)),
                _ => None,
            }
        },
        "identity" if args.len() == 1 => Some(args[0].clone()),
        "symbol-name" if args.len() == 1 => match args[0] {
            SsaConst::Symbol(s) => Some(SsaConst::String(s.clone())),
            SsaConst::Nil => Some(SsaConst::String("nil".into())),
            SsaConst::True => Some(SsaConst::String("t".into())),
            _ => None,
        },
        "make-string" if args.len() == 2 => {
            let count = args[0].as_int()? as usize;
            let ch = match args[1] {
                SsaConst::Char(c) => char::from_u32(*c as u32).unwrap_or(' '),
                SsaConst::Int(n) => char::from_u32(*n as u32).unwrap_or(' '),
                _ => return None,
            };
            let s: String = std::iter::repeat(ch).take(count.min(4096)).collect();
            Some(SsaConst::String(s))
        }
        "make-list" if args.len() == 2 => {
            let _count = args[0].as_int()?;
            // Can't represent a repeated list constant in SsaConst.
            // Only fold the 0-length case (returns nil).
            if _count == 0 {
                Some(SsaConst::Nil)
            } else { None }
        }
        "copy-sequence" if args.len() == 1 => match args[0] {
            SsaConst::String(s) => Some(SsaConst::String(s.clone())),
            SsaConst::Nil => Some(SsaConst::Nil),
            _ => None,
        },
        "string-to-char" if args.len() == 1 => match args[0] {
            SsaConst::String(s) if s.is_empty() => Some(SsaConst::Int(0)),
            SsaConst::String(s) => Some(SsaConst::Char(s.chars().next()? as i64)),
            _ => None,
        },
        "char-to-string" if args.len() == 1 => match args[0] {
            SsaConst::Char(c) => Some(SsaConst::String(char::from_u32(*c as u32)?.to_string())),
            _ => None,
        },
        "char-code" if args.len() == 1 => match args[0] {
            SsaConst::Char(c) => Some(SsaConst::Int(*c)),
            SsaConst::String(s) => s.chars().next().map(|ch| SsaConst::Int(ch as i64)),
            _ => None,
        },
        "upcase-initials" if args.len() == 1 => match args[0] {
            SsaConst::String(s) => {
                let mut result = String::with_capacity(s.len());
                let mut at_word_start = true;
                for ch in s.chars() {
                    if at_word_start && ch.is_lowercase() {
                        result.extend(ch.to_uppercase());
                    } else {
                        result.push(ch);
                    }
                    at_word_start = !ch.is_alphanumeric();
                }
                Some(SsaConst::String(result))
            }
            _ => None,
        },
        "sequencep" if args.len() == 1 => match args[0] {
            SsaConst::String(_) => Some(SsaConst::True),
            _ => Some(SsaConst::Nil),
        },
        "string-to-number" if args.len() >= 1 => {
            let s = match args[0] { SsaConst::String(s) => s.as_str(), _ => return None };
            let base = args.get(1).map(|b| match b { SsaConst::Int(n) => *n, _ => 0 }).unwrap_or(10);
            // Auto-detect #x #o #b prefixes when base is 10 (default).
            let (effective_base, digits) = if base == 10 {
                if let Some(hex) = s.strip_prefix("#x") {
                    (16, hex)
                } else if let Some(oct) = s.strip_prefix("#o") {
                    (8, oct)
                } else if let Some(bin) = s.strip_prefix("#b") {
                    (2, bin)
                } else {
                    (10, s)
                }
            } else {
                (base, s)
            };
            if effective_base == 10 {
                if let Ok(n) = digits.parse::<i64>() {
                    return Some(SsaConst::Int(n));
                } else if let Ok(f) = digits.parse::<f64>() {
                    return Some(SsaConst::Float(f));
                }
            } else if (2..=36).contains(&effective_base) {
                if let Ok(n) = i64::from_str_radix(digits, effective_base as u32) {
                    return Some(SsaConst::Int(n));
                }
            }
            None
        },
        "append" | "nconc" if args.is_empty() => Some(SsaConst::Nil),
        "concat" if args.is_empty() => Some(SsaConst::String(String::new())),
        "concat" if !args.is_empty() => {
            if args.iter().all(|a| matches!(a, SsaConst::String(_))) {
                let s: String = args.iter()
                    .filter_map(|a| match a { SsaConst::String(s) => Some(s.as_str()), _ => None })
                    .collect();
                Some(SsaConst::String(s))
            } else { None }
        }
        "lsh" if args.len() == 2 => {
            let value = args[0].as_int()?;
            let count = args[1].as_int()?;
            let u = value as u64;
            if count >= 64 {
                Some(SsaConst::Int(0))
            } else if count >= 0 {
                Some(SsaConst::Int((u.wrapping_shl(count as u32)) as i64))
            } else if count <= -64 {
                Some(SsaConst::Int(0))
            } else {
                Some(SsaConst::Int((u.wrapping_shr((-count) as u32)) as i64))
            }
        }
        "sin" if args.len() == 1 => match args[0] {
            SsaConst::Float(f) => Some(SsaConst::Float(f.sin())),
            SsaConst::Int(n) => Some(SsaConst::Float((*n as f64).sin())),
            _ => None,
        },
        "tan" if args.len() == 1 => match args[0] {
            SsaConst::Float(f) => Some(SsaConst::Float(f.tan())),
            SsaConst::Int(n) => Some(SsaConst::Float((*n as f64).tan())),
            _ => None,
        },
        "cos" if args.len() == 1 => match args[0] {
            SsaConst::Float(f) => Some(SsaConst::Float(f.cos())),
            SsaConst::Int(n) => Some(SsaConst::Float((*n as f64).cos())),
            _ => None,
        },
        "log" if args.len() == 1 => match args[0] {
            SsaConst::Float(f) if *f > 0.0 => Some(SsaConst::Float(f.ln())),
            SsaConst::Int(n) if *n > 0 => Some(SsaConst::Float((*n as f64).ln())),
            _ => None,
        },
        "log" if args.len() == 2 => {
            let x = to_f64(args[0])?;
            let base = to_f64(args[1])?;
            if x > 0.0 && base > 0.0 && base != 1.0 {
                Some(SsaConst::Float(x.ln() / base.ln()))
            } else { None }
        },
        "log10" if args.len() == 1 => match args[0] {
            SsaConst::Float(f) if *f > 0.0 => Some(SsaConst::Float(f.log10())),
            SsaConst::Int(n) if *n > 0 => Some(SsaConst::Float((*n as f64).log10())),
            _ => None,
        },
        "exp" if args.len() == 1 => match args[0] {
            SsaConst::Float(f) => Some(SsaConst::Float(f.exp())),
            SsaConst::Int(n) => Some(SsaConst::Float((*n as f64).exp())),
            _ => None,
        },
        "asin" if args.len() == 1 => match args[0] {
            SsaConst::Float(f) if (-1.0..=1.0).contains(f) => Some(SsaConst::Float(f.asin())),
            SsaConst::Int(n) if *n >= -1 && *n <= 1 => Some(SsaConst::Float((*n as f64).asin())),
            _ => None,
        },
        "acos" if args.len() == 1 => match args[0] {
            SsaConst::Float(f) if (-1.0..=1.0).contains(f) => Some(SsaConst::Float(f.acos())),
            SsaConst::Int(n) if *n >= -1 && *n <= 1 => Some(SsaConst::Float((*n as f64).acos())),
            _ => None,
        },
        "atan" if args.len() == 1 => match args[0] {
            SsaConst::Float(f) => Some(SsaConst::Float(f.atan())),
            SsaConst::Int(n) => Some(SsaConst::Float((*n as f64).atan())),
            _ => None,
        },
        "atan" if args.len() == 2 => match (args[0], args[1]) {
            (SsaConst::Float(y), SsaConst::Float(x)) => Some(SsaConst::Float(y.atan2(*x))),
            (SsaConst::Float(y), SsaConst::Int(x)) => Some(SsaConst::Float(y.atan2(*x as f64))),
            (SsaConst::Int(y), SsaConst::Float(x)) => Some(SsaConst::Float((*y as f64).atan2(*x))),
            (SsaConst::Int(y), SsaConst::Int(x)) => Some(SsaConst::Float((*y as f64).atan2(*x as f64))),
            _ => None,
        },
        "sqrt" if args.len() == 1 => match args[0] {
            SsaConst::Float(f) if *f >= 0.0 => Some(SsaConst::Float(f.sqrt())),
            SsaConst::Int(n) if *n >= 0 => Some(SsaConst::Float((*n as f64).sqrt())),
            _ => None,
        },
        "expt" if args.len() == 2 => {
            // If both are integers, try exact integer pow
            if let (Some(base), Some(exp)) = (args[0].as_int(), args[1].as_int()) {
                if exp >= 0 {
                    return base.checked_pow(exp as u32).map(SsaConst::Int);
                } else if exp >= i32::MIN as i64 {
                    return Some(SsaConst::Float((base as f64).powi(exp as i32)));
                }
            }
            // Float path
            let base = to_f64(args[0])?;
            let exp = to_f64(args[1])?;
            Some(SsaConst::Float(base.powf(exp)))
        }
        "abs" if args.len() == 1 => match args[0] {
            SsaConst::Int(a) => Some(SsaConst::Int(a.wrapping_abs())),
            SsaConst::Float(f) => Some(SsaConst::Float(f.abs())),
            _ => None,
        },
        "truncate" if args.len() == 1 => match args[0] {
            SsaConst::Int(a) => Some(SsaConst::Int(*a)),
            SsaConst::Float(f) => Some(SsaConst::Int(f.trunc() as i64)),
            _ => None,
        },
        "truncate" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            if b == 0 { return None; }
            Some(SsaConst::Int(a / b))
        }
        "floor" if args.len() == 1 => match args[0] {
            SsaConst::Int(a) => Some(SsaConst::Int(*a)),
            SsaConst::Float(f) => Some(SsaConst::Int(f.floor() as i64)),
            _ => None,
        },
        "floor" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            if b == 0 { return None; }
            let q = a / b;
            let r = a % b;
            Some(SsaConst::Int(if r != 0 && (a ^ b) < 0 { q - 1 } else { q }))
        }
        "ceiling" if args.len() == 1 => match args[0] {
            SsaConst::Int(a) => Some(SsaConst::Int(*a)),
            SsaConst::Float(f) => Some(SsaConst::Int(f.ceil() as i64)),
            _ => None,
        },
        "ceiling" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            if b == 0 { return None; }
            let q = a / b;
            let r = a % b;
            Some(SsaConst::Int(if r != 0 && (a ^ b) >= 0 { q + 1 } else { q }))
        }
        "round" | "fround" if args.len() == 1 => match args[0] {
            SsaConst::Int(a) => Some(SsaConst::Int(*a)),
            SsaConst::Float(f) => Some(SsaConst::Int(f.round() as i64)),
            _ => None,
        },
        "round" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            if b == 0 { return None; }
            // Round to nearest, ties to even (banker's rounding)
            let q = a / b;
            let r = a % b;
            let half = b.abs() / 2;
            let r_abs = r.abs();
            if r_abs > half || (r_abs == half && q % 2 != 0) {
                Some(SsaConst::Int(if (a ^ b) >= 0 { q + 1 } else { q - 1 }))
            } else {
                Some(SsaConst::Int(q))
            }
        }
        "ffloor" if args.len() == 1 => match args[0] {
            SsaConst::Float(f) => Some(SsaConst::Float(f.floor())),
            SsaConst::Int(n) => Some(SsaConst::Float(*n as f64)),
            _ => None,
        },
        "ffloor" if args.len() == 2 => {
            let a = to_f64(args[0])?;
            let b = to_f64(args[1])?;
            if b == 0.0 { return None; }
            Some(SsaConst::Float((a / b).floor()))
        }
        "fceiling" if args.len() == 1 => match args[0] {
            SsaConst::Float(f) => Some(SsaConst::Float(f.ceil())),
            SsaConst::Int(n) => Some(SsaConst::Float(*n as f64)),
            _ => None,
        },
        "fceiling" if args.len() == 2 => {
            let a = to_f64(args[0])?;
            let b = to_f64(args[1])?;
            if b == 0.0 { return None; }
            Some(SsaConst::Float((a / b).ceil()))
        }
        "ftruncate" if args.len() == 1 => match args[0] {
            SsaConst::Float(f) => Some(SsaConst::Float(f.trunc())),
            SsaConst::Int(n) => Some(SsaConst::Float(*n as f64)),
            _ => None,
        },
        "ftruncate" if args.len() == 2 => {
            let a = to_f64(args[0])?;
            let b = to_f64(args[1])?;
            if b == 0.0 { return None; }
            Some(SsaConst::Float((a / b).trunc()))
        }
        "lognot" if args.len() == 1 => {
            Some(SsaConst::Int(!args[0].as_int()?))
        }
        "integerp" if args.len() == 1 => Some(match args[0] {
            SsaConst::Int(_) | SsaConst::Char(_) => SsaConst::True,
            _ => SsaConst::Nil,
        }),
        "floatp" if args.len() == 1 => Some(match args[0] {
            SsaConst::Float(_) => SsaConst::True,
            _ => SsaConst::Nil,
        }),
        "stringp" if args.len() == 1 => Some(match args[0] {
            SsaConst::String(_) => SsaConst::True,
            _ => SsaConst::Nil,
        }),
        "string=" if args.len() == 2 => match (args[0], args[1]) {
            (SsaConst::String(a), SsaConst::String(b)) => {
                Some(if a == b { SsaConst::True } else { SsaConst::Nil })
            }
            _ => None,
        },
        "string-equal" if args.len() == 2 => match (args[0], args[1]) {
            (SsaConst::String(a), SsaConst::String(b)) if a == b => {
                Some(SsaConst::True)
            }
            _ => None,
        },
        "string<" | "string-lessp" if args.len() == 2 => match (args[0], args[1]) {
            (SsaConst::String(a), SsaConst::String(b)) => {
                Some(if a < b { SsaConst::True } else { SsaConst::Nil })
            }
            _ => None,
        },
        "string>" | "string-greaterp" if args.len() == 2 => match (args[0], args[1]) {
            (SsaConst::String(a), SsaConst::String(b)) => {
                Some(if a > b { SsaConst::True } else { SsaConst::Nil })
            }
            _ => None,
        },
        "string-prefix-p" if args.len() >= 2 => match (args[0], args[1]) {
            (SsaConst::String(prefix), SsaConst::String(s)) => {
                let ignore_case = args.get(2).map(|c| c.as_int() == Some(0)).unwrap_or(false);
                if ignore_case {
                    Some(if s.to_lowercase().starts_with(&prefix.to_lowercase()) {
                        SsaConst::True
                    } else {
                        SsaConst::Nil
                    })
                } else {
                    Some(if s.starts_with(prefix.as_str()) {
                        SsaConst::True
                    } else {
                        SsaConst::Nil
                    })
                }
            }
            _ => None,
        },
        "string-suffix-p" if args.len() >= 2 => match (args[0], args[1]) {
            (SsaConst::String(suffix), SsaConst::String(s)) => {
                let ignore_case = args.get(2).map(|c| c.as_int() == Some(0)).unwrap_or(false);
                if ignore_case {
                    Some(if s.to_lowercase().ends_with(&suffix.to_lowercase()) {
                        SsaConst::True
                    } else {
                        SsaConst::Nil
                    })
                } else {
                    Some(if s.ends_with(suffix.as_str()) {
                        SsaConst::True
                    } else {
                        SsaConst::Nil
                    })
                }
            }
            _ => None,
        },
        "string-remove-prefix" if args.len() == 2 => match (args[0], args[1]) {
            (SsaConst::String(prefix), SsaConst::String(s)) => {
                Some(SsaConst::String(s.strip_prefix(prefix.as_str()).unwrap_or(s).to_string()))
            }
            _ => None,
        },
        "string-remove-suffix" if args.len() == 2 => match (args[0], args[1]) {
            (SsaConst::String(suffix), SsaConst::String(s)) => {
                Some(SsaConst::String(s.strip_suffix(suffix.as_str()).unwrap_or(s).to_string()))
            }
            _ => None,
        },
        "zerop" if args.len() == 1 => match args[0] {
            SsaConst::Int(a) => Some(if *a == 0 { SsaConst::True } else { SsaConst::Nil }),
            SsaConst::Float(f) => Some(if *f == 0.0 || *f == -0.0 { SsaConst::True } else { SsaConst::Nil }),
            _ => None,
        },
        "rem" | "%" if args.len() == 2 => {
            if let (Some(a), Some(b)) = (args[0].as_int(), args[1].as_int()) {
                if b == 0 { return None; }
                return Some(SsaConst::Int(a.wrapping_rem(b)));
            }
            let a = to_f64(args[0])?;
            let b = to_f64(args[1])?;
            if b == 0.0 { return None; }
            Some(SsaConst::Float(a % b))
        }
        "mod" if args.len() == 2 => {
            if let (Some(a), Some(b)) = (args[0].as_int(), args[1].as_int()) {
                if b == 0 { return None; }
                let r = a.wrapping_rem(b);
                return Some(SsaConst::Int(if r == 0 || (a ^ b) >= 0 { r } else { r.wrapping_add(b) }));
            }
            let a = to_f64(args[0])?;
            let b = to_f64(args[1])?;
            if b == 0.0 { return None; }
            Some(SsaConst::Float(a - b * (a / b).floor()))
        }
        "consp" if args.len() == 1 => Some(match args[0] {
            SsaConst::Value(_) => return None,
            _ => SsaConst::Nil,
        }),
        "listp" if args.len() == 1 => Some(match args[0] {
            SsaConst::Value(_) => return None,
            SsaConst::Nil => SsaConst::True,
            _ => SsaConst::Nil,
        }),
        "atom" if args.len() == 1 => Some(match args[0] {
            SsaConst::Value(_) => return None,
            _ => SsaConst::True,
        }),
        "numberp" if args.len() == 1 => Some(match args[0] {
            SsaConst::Int(_) | SsaConst::Float(_) | SsaConst::Char(_) => SsaConst::True,
            SsaConst::Value(_) => return None,
            _ => SsaConst::Nil,
        }),
        "symbolp" if args.len() == 1 => Some(match args[0] {
            SsaConst::Symbol(_) | SsaConst::Nil | SsaConst::True => SsaConst::True,
            SsaConst::Value(_) => return None,
            _ => SsaConst::Nil,
        }),
        "vectorp" | "hash-table-p" if args.len() == 1 => Some(match args[0] {
            SsaConst::Value(_) => return None,
            _ => SsaConst::Nil,
        }),
        "nlistp" if args.len() == 1 => Some(match args[0] {
            SsaConst::Nil => SsaConst::Nil,
            SsaConst::Value(_) => return None,
            _ => SsaConst::True,
        }),
        "functionp" | "subrp" | "compiled-function-p" | "macrop" if args.len() == 1 => Some(match args[0] {
            SsaConst::Value(_) => return None,
            _ => SsaConst::Nil,
        }),
        "car-safe" | "cdr-safe" if args.len() == 1 => Some(match args[0] {
            SsaConst::Value(_) => return None,
            _ => SsaConst::Nil,
        }),
        "char-equal" if args.len() == 2 => match (args[0], args[1]) {
            (SsaConst::Char(a), SsaConst::Char(b)) => {
                let ca = char::from_u32(*a as u32)?;
                let cb = char::from_u32(*b as u32)?;
                Some(if ca == cb || ca.to_ascii_lowercase() == cb.to_ascii_lowercase() {
                    SsaConst::True
                } else {
                    SsaConst::Nil
                })
            }
            _ => None,
        },
        "char-or-string-p" if args.len() == 1 => Some(match args[0] {
            SsaConst::Char(_) | SsaConst::String(_) => SsaConst::True,
            SsaConst::Value(_) => return None,
            _ => SsaConst::Nil,
        }),
        "natnump" | "wholenump" if args.len() == 1 => {
            let n = args[0].as_int()?;
            Some(if n >= 0 { SsaConst::True } else { SsaConst::Nil })
        }
        "evenp" | "cl-evenp" if args.len() == 1 => {
            let n = args[0].as_int()?;
            Some(if n & 1 == 0 { SsaConst::True } else { SsaConst::Nil })
        }
        "oddp" | "cl-oddp" if args.len() == 1 => {
            let n = args[0].as_int()?;
            Some(if n & 1 != 0 { SsaConst::True } else { SsaConst::Nil })
        }
        "minusp" | "cl-minusp" if args.len() == 1 => {
            let n = args[0].as_int()?;
            Some(if n < 0 { SsaConst::True } else { SsaConst::Nil })
        }
        "plusp" | "cl-plusp" if args.len() == 1 => {
            let n = args[0].as_int()?;
            Some(if n > 0 { SsaConst::True } else { SsaConst::Nil })
        }
        "special-form-p" if args.len() == 1 => {
            let name = match args[0] {
                SsaConst::Symbol(s) => s.as_str(),
                _ => return Some(SsaConst::Nil),
            };
            Some(if is_special_form_name(name) {
                SsaConst::True
            } else {
                SsaConst::Nil
            })
        }
        "reverse" | "nreverse" if args.len() == 1 => match args[0] {
            SsaConst::String(s) => {
                Some(SsaConst::String(s.chars().rev().collect()))
            }
            SsaConst::Nil => Some(SsaConst::Nil),
            _ => None,
        },
        "subseq" | "cl-subseq" if args.len() >= 2 => {
            let s = match args[0] { SsaConst::String(s) => s.as_str(), _ => return None };
            let start = args[1].as_int()? as usize;
            let end = if args.len() >= 3 {
                args[2].as_int()? as usize
            } else {
                s.len()
            };
            if start > end || start > s.len() || end > s.len() { return None; }
            Some(SsaConst::String(s[start..end].to_string()))
        }
        "booleanp" if args.len() == 1 => match args[0] {
            SsaConst::Nil | SsaConst::True => Some(SsaConst::True),
            _ => Some(SsaConst::Nil),
        },
        "fixnump" if args.len() == 1 => match args[0] {
            SsaConst::Int(_) => Some(SsaConst::True),
            _ => Some(SsaConst::Nil),
        },
        "string-or-null-p" if args.len() == 1 => match args[0] {
            SsaConst::Nil | SsaConst::String(_) => Some(SsaConst::True),
            _ => Some(SsaConst::Nil),
        },
        "number-or-marker-p" | "integer-or-marker-p" if args.len() == 1 => match args[0] {
            SsaConst::Int(_) | SsaConst::Float(_) => Some(SsaConst::True),
            _ => Some(SsaConst::Nil),
        },
        "always" if args.len() >= 1 => Some(SsaConst::True),
        "bignump" if args.len() == 1 => Some(SsaConst::Nil),
        "arrayp" if args.len() == 1 => match args[0] {
            SsaConst::String(_) => Some(SsaConst::True),
            _ => Some(SsaConst::Nil),
        },
        "bool-vector-p" | "recordp" | "char-table-p" | "autoloadp"
            if args.len() == 1 => Some(SsaConst::Nil),
        "bare-symbol-p" if args.len() == 1 => match args[0] {
            SsaConst::Symbol(_) => Some(SsaConst::True),
            _ => Some(SsaConst::Nil),
        },
        "keywordp" if args.len() == 1 => match args[0] {
            SsaConst::Symbol(s) => Some(if s.starts_with(':') { SsaConst::True } else { SsaConst::Nil }),
            _ => Some(SsaConst::Nil),
        },
        "ignore" => Some(SsaConst::Nil),
        "delete" | "delq" | "remove" | "remq" | "delete-dups" | "cl-remove-duplicates"
        | "cl-remove-if" | "cl-remove-if-not" | "cl-delete-if" | "cl-delete-if-not"
            if args.len() >= 1 => {
            // delete/remove/delq/remq: list is args[1]
            // delete-dups: list is args[0]
            if matches!(args.last()?, SsaConst::Nil) {
                Some(SsaConst::Nil)
            } else {
                None
            }
        },
        "assoc" | "assq" | "rassoc" | "rassq" | "member" | "memq" | "memql"
            if args.len() >= 2 => {
            // All alist/list search functions return nil when the list/alist is nil
            if matches!(args[1], SsaConst::Nil) {
                Some(SsaConst::Nil)
            } else {
                None
            }
        },
        "cl-endp" if args.len() == 1 => match args[0] {
            SsaConst::Nil => Some(SsaConst::True),
            _ => None, // non-nil may signal error, let runtime handle
        },
        "sort" | "cl-sort" | "cl-stable-sort" if args.len() == 2 => match args[0] {
            SsaConst::Nil => Some(SsaConst::Nil),
            _ => None,
        },
        "cl-subst" | "cl-subst-if" | "cl-subst-if-not"
        | "cl-nsubst" | "cl-nsubst-if" | "cl-nsubst-if-not"
        | "cl-substitute" | "cl-substitute-if" | "cl-substitute-if-not"
        | "cl-nsubstitute" | "cl-nsubstitute-if" | "cl-nsubstitute-if-not"
            if args.len() >= 3 => {
            // (cl-subst NEW OLD TREE) — tree is args[2]
            if matches!(args[2], SsaConst::Nil) {
                Some(SsaConst::Nil)
            } else {
                None
            }
        },
        "cl-intersection" | "cl-nintersection"
        | "cl-union" | "cl-nunion"
        | "cl-set-difference" | "cl-nset-difference"
        | "cl-set-exclusive-or" | "cl-nset-exclusive-or"
            if args.len() >= 2 => {
            // (cl-intersection LIST1 LIST2) — if LIST1 is nil, result is nil
            if matches!(args[0], SsaConst::Nil) {
                Some(SsaConst::Nil)
            } else {
                None
            }
        },
        "cl-sublis" | "cl-nsublis" if args.len() >= 2 => {
            // (cl-sublis ALIST TREE) — tree is args[1]
            if matches!(args[1], SsaConst::Nil) {
                Some(SsaConst::Nil)
            } else {
                None
            }
        },
        "mapcar" | "mapc" | "cl-mapcar" | "cl-mapc" | "mapconcat"
            if args.len() >= 2 => {
            // (mapcar FN nil) → nil, (mapconcat FN nil SEP) → ""
            if matches!(args[1], SsaConst::Nil) {
                if name == "mapconcat" {
                    Some(SsaConst::String(String::new()))
                } else {
                    Some(SsaConst::Nil)
                }
            } else {
                None
            }
        },
        "cl-merge" if args.len() >= 4 => {
            // (cl-merge TYPE SEQ1 SEQ2 PRED) — if both seqs nil, return nil
            if matches!(args[1], SsaConst::Nil) && matches!(args[2], SsaConst::Nil) {
                Some(SsaConst::Nil)
            } else {
                None
            }
        },
        "cl-count" | "cl-count-if" | "cl-count-if-not" if args.len() >= 2 => {
            // (cl-count ITEM SEQ) — seq is args[1]; nil seq → 0
            if matches!(args[1], SsaConst::Nil) {
                Some(SsaConst::Int(0))
            } else {
                None
            }
        },
        "nconc" if args.len() >= 1 => {
            // (nconc) → nil, (nconc nil ...) → nil
            // If all args are nil, result is nil
            if args.iter().all(|a| matches!(a, SsaConst::Nil)) {
                Some(SsaConst::Nil)
            } else {
                None
            }
        },
        "car" | "cdr" | "caar" | "cadr" | "cdar" | "cddr"
        | "caaar" | "caadr" | "cadar" | "caddr" | "cdaar" | "cdadr" | "cddar" | "cdddr"
        | "nth" | "nthcdr" | "last" | "butlast" | "nbutlast"
            if args.len() >= 1 => {
            if matches!(args[0], SsaConst::Nil) {
                Some(SsaConst::Nil)
            } else {
                None
            }
        },
        "format" | "format-message" if args.len() == 1 => match args[0] {
            SsaConst::String(s) if !s.contains('%') => Some(SsaConst::String(s.clone())),
            _ => None,
        },
        "prin1-to-string" | "princ-to-string" if args.len() == 1 => match args[0] {
            SsaConst::Int(n) => Some(SsaConst::String(n.to_string())),
            SsaConst::Float(f) => Some(SsaConst::String(f.to_string())),
            SsaConst::Char(c) => Some(SsaConst::String(char::from_u32(*c as u32)?.to_string())),
            SsaConst::String(s) => Some(SsaConst::String(format!("\"{s}\""))),
            SsaConst::Symbol(s) => Some(SsaConst::String(s.clone())),
            SsaConst::Nil => Some(SsaConst::String("nil".into())),
            SsaConst::True => Some(SsaConst::String("t".into())),
            _ => None,
        },
        _ => None,
    }
}

pub fn dead_code_elimination(function: &mut SsaFunction) -> OptOutput {
    let mut changed = false;
    let mut loop_changed = true;
    while loop_changed {
        loop_changed = false;
        let use_counts = compute_use_counts(function);
        for (block_id, block) in function.blocks.iter_mut() {
            let mut new_instructions = Vec::new();
            for inst in &block.instructions {
                let is_used = inst
                    .result
                    .is_none_or(|r| *use_counts.get(&r).unwrap_or(&0) > 0);
                if !is_used && is_pure(&inst.effects) {
                    loop_changed = true;
                    changed = true;
                } else {
                    new_instructions.push(inst.clone());
                }
            }
            // Re-index value locations after removing dead instructions.
            for (new_idx, inst) in new_instructions.iter().enumerate() {
                if let Some(result) = inst.result {
                    function.values[result].kind = crate::ssa::SsaValueKind::InstResult {
                        block: block_id,
                        inst: new_idx,
                    };
                }
            }
            if loop_changed || new_instructions.len() != block.instructions.len() {
                block.instructions = new_instructions;
            }
        }
    }
    OptOutput { changed }
}

pub fn block_merging(function: &mut SsaFunction) -> OptOutput {
    let mut changed = false;
    loop {
        let preds = crate::verify::predecessor_map(function);
        let mut merge: Option<(BlockId, BlockId)> = None;
        for (bid, block) in function.blocks.iter() {
            if let SsaTerminator::Jump { target, args } = &block.terminator
                && args.is_empty()
                    && let Some(pred_list) = preds.get(target)
                        && pred_list.len() == 1 && pred_list[0] == bid
                            && function.blocks[*target].params.is_empty() {
                                merge = Some((bid, *target));
                                break;
                            }
        }
        let (src, dst) = match merge {
            Some(m) => m,
            None => break,
        };
        let dst_terminator = function.blocks[dst].terminator.clone();
        let dst_instructions = std::mem::take(&mut function.blocks[dst].instructions);
        let src_block = &mut function.blocks[src];
        src_block.terminator = dst_terminator;
        src_block.instructions.extend(dst_instructions);
        // Re-index value locations for moved instructions
        let src_id = src;
        for (new_idx, inst) in src_block.instructions.iter().enumerate() {
            if let Some(result) = inst.result {
                function.values[result].kind = crate::ssa::SsaValueKind::InstResult {
                    block: src_id,
                    inst: new_idx,
                };
            }
        }
        // Mark merged block as unreachable (PrimaryMap has no remove)
        function.blocks[dst].instructions.clear();
        function.blocks[dst].terminator = SsaTerminator::Unreachable;
        // Redirect any other jumps to the merged block so they point to src
        for (_, block) in function.blocks.iter_mut() {
            if let SsaTerminator::Jump { target, .. } = &mut block.terminator
                && *target == dst {
                    *target = src;
                }
            if let SsaTerminator::BranchIfNil {
                then_target,
                else_target,
                ..
            } = &mut block.terminator
            {
                if *then_target == dst {
                    *then_target = src;
                }
                if *else_target == dst {
                    *else_target = src;
                }
            }
        }
        changed = true;
    }
    OptOutput { changed }
}

#[derive(Clone, Hash, PartialEq, Eq)]
enum CseKey {
    CallNamed { name: String, args: Vec<ValueId> },
    BinOp { op: String, a: ValueId, b: ValueId },
}

pub fn common_subexpression_elimination(function: &mut SsaFunction) -> OptOutput {
    let mut subst: HashMap<ValueId, ValueId> = HashMap::new();
    let mut changed = false;

    for block in function.blocks.values() {
        let mut seen: HashMap<CseKey, ValueId> = HashMap::new();
        for inst in &block.instructions {
            let Some(result) = inst.result else { continue };
            if !is_pure(&inst.effects) {
                continue;
            }
            let key = match &inst.kind {
                SsaInstKind::CallNamed { name, args } if args.len() == 2 => {
                    let resolve = |v: &ValueId| -> ValueId {
                        let mut current = *v;
                        while let Some(&r) = subst.get(&current) {
                            current = r;
                        }
                        current
                    };
                    Some(CseKey::BinOp {
                        op: name.clone(),
                        a: resolve(&args[0]),
                        b: resolve(&args[1]),
                    })
                }
                SsaInstKind::CallNamed { name, args } => {
                    let resolve = |v: &ValueId| -> ValueId {
                        let mut current = *v;
                        while let Some(&r) = subst.get(&current) {
                            current = r;
                        }
                        current
                    };
                    let resolved: Vec<ValueId> = args.iter().map(resolve).collect();
                    Some(CseKey::CallNamed {
                        name: name.clone(),
                        args: resolved,
                    })
                }
                _ => None,
            };
            if let Some(key) = key {
                if let Some(&existing) = seen.get(&key) {
                    subst.insert(result, existing);
                    changed = true;
                } else {
                    seen.insert(key, result);
                }
            }
        }
    }

    if !subst.is_empty() {
        apply_value_substitution(function, &subst);
    }

    OptOutput { changed }
}

impl SsaConst {
    fn as_int(&self) -> Option<i64> {
        match self {
            SsaConst::Int(n) | SsaConst::Char(n) => Some(*n),
            _ => None,
        }
    }
}

fn to_f64(c: &SsaConst) -> Option<f64> {
    match c {
        SsaConst::Float(f) => Some(*f),
        SsaConst::Int(n) | SsaConst::Char(n) => Some(*n as f64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_source;

    #[test]
    fn folds_integer_addition() {
        let artifact = compile_source("fold.el", ";;; -*- lexical-binding: t; -*-\n(+ 1 2)");
        assert_eq!(artifact.diagnostics, Vec::new());
        let ssa = artifact.ssa.unwrap();
        let func = ssa.functions.values().next().unwrap();
        // After optimization, the + call should be folded to Const(3)
        let has_const_3 = func.blocks.values().any(|b| {
            b.instructions
                .iter()
                .any(|inst| matches!(&inst.kind, SsaInstKind::Const(SsaConst::Int(3))))
        });
        assert!(has_const_3, "expected constant 3 after folding");
    }

    #[test]
    fn folds_branch_on_nil() {
        let artifact = compile_source(
            "fold-nil.el",
            ";;; -*- lexical-binding: t; -*-\n(if nil 1 2)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let ssa = artifact.ssa.unwrap();
        let func = ssa.functions.values().next().unwrap();
        // After folding, BranchIfNil should be replaced with Jump
        let has_branch = func
            .blocks
            .values()
            .any(|b| matches!(b.terminator, SsaTerminator::BranchIfNil { .. }));
        assert!(!has_branch, "BranchIfNil should be folded to Jump");
    }

    #[test]
    fn folds_nested_arithmetic() {
        let artifact = compile_source(
            "fold-nested.el",
            ";;; -*- lexical-binding: t; -*-\n(+ (* 2 3) 4)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let ssa = artifact.ssa.unwrap();
        let func = ssa.functions.values().next().unwrap();
        let has_const_10 = func.blocks.values().any(|b| {
            b.instructions
                .iter()
                .any(|inst| matches!(&inst.kind, SsaInstKind::Const(SsaConst::Int(10))))
        });
        assert!(
            has_const_10,
            "expected constant 10 after folding (* 2 3) + 4"
        );
    }

    #[test]
    fn dce_removes_unused_pure_instructions() {
        let artifact = compile_source(
            "dce.el",
            ";;; -*- lexical-binding: t; -*-\n(let ((x (+ 1 2))) 5)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let ssa = artifact.ssa.unwrap();
        let func = ssa.functions.values().next().unwrap();
        // After optimization, (+ 1 2) should be eliminated since x is unused
        let has_add_call = func.blocks.values().any(|b| {
            b.instructions.iter().any(
                |inst| matches!(&inst.kind, SsaInstKind::CallNamed { name, .. } if name == "+"),
            )
        });
        assert!(!has_add_call, "unused + should be eliminated by DCE");
    }
}
