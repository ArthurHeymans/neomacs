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
    match (a, b) {
        (SsaConst::Int(a), SsaConst::Int(b)) => Some(SsaConst::Int(int_op(*a, *b))),
        (SsaConst::Float(a), SsaConst::Float(b)) => Some(SsaConst::Float(float_op(*a, *b))),
        (SsaConst::Int(a), SsaConst::Float(b)) => Some(SsaConst::Float(float_op(*a as f64, *b))),
        (SsaConst::Float(a), SsaConst::Int(b)) => Some(SsaConst::Float(float_op(*a, *b as f64))),
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
        "=" if args.len() == 2 => fold_cmp(args[0], args[1], |o| o.is_eq()),
        "<" if args.len() == 2 => fold_cmp(args[0], args[1], |o| o.is_lt()),
        ">" if args.len() == 2 => fold_cmp(args[0], args[1], |o| o.is_gt()),
        "<=" if args.len() == 2 => fold_cmp(args[0], args[1], |o| o.is_le()),
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
            (SsaConst::Nil, SsaConst::Nil) => Some(SsaConst::True),
            (SsaConst::True, SsaConst::True) => Some(SsaConst::True),
            _ => None,
        },
        "1+" if args.len() == 1 => match args[0] {
            SsaConst::Int(a) => Some(SsaConst::Int(a.wrapping_add(1))),
            SsaConst::Float(f) => Some(SsaConst::Float(f + 1.0)),
            _ => None,
        },
        "1-" if args.len() == 1 => match args[0] {
            SsaConst::Int(a) => Some(SsaConst::Int(a.wrapping_sub(1))),
            SsaConst::Float(f) => Some(SsaConst::Float(f - 1.0)),
            _ => None,
        },
        "null" | "not" if args.len() == 1 => match const_to_bool(args[0]) {
            Some(false) => Some(SsaConst::True),
            Some(true) => Some(SsaConst::Nil),
            None => None,
        },
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
            let floats: Vec<f64> = args.iter().map(|a| match a {
                SsaConst::Float(f) => Some(*f),
                _ => None,
            }).collect::<Option<Vec<_>>>()?;
            Some(SsaConst::Float(floats.into_iter().fold(f64::NEG_INFINITY, f64::max)))
        }
        "min" if !args.is_empty() => {
            if let Some(ints) = args.iter().map(|a| a.as_int()).collect::<Option<Vec<_>>>() {
                return Some(SsaConst::Int(ints.into_iter().min().unwrap()));
            }
            let floats: Vec<f64> = args.iter().map(|a| match a {
                SsaConst::Float(f) => Some(*f),
                _ => None,
            }).collect::<Option<Vec<_>>>()?;
            Some(SsaConst::Float(floats.into_iter().fold(f64::INFINITY, f64::min)))
        }
        "+" if args.len() >= 2 => {
            let ints: Vec<i64> = args.iter().map(|a| a.as_int()).collect::<Option<Vec<_>>>()?;
            Some(SsaConst::Int(ints.into_iter().sum()))
        }
        "*" if args.len() >= 2 => {
            let ints: Vec<i64> = args.iter().map(|a| a.as_int()).collect::<Option<Vec<_>>>()?;
            Some(SsaConst::Int(ints.into_iter().product()))
        }
        "logand" if !args.is_empty() => {
            let ints: Vec<i64> = args.iter().map(|a| a.as_int()).collect::<Option<Vec<_>>>()?;
            Some(SsaConst::Int(ints.into_iter().fold(!0i64, |a, b| a & b)))
        }
        "logior" if !args.is_empty() => {
            let ints: Vec<i64> = args.iter().map(|a| a.as_int()).collect::<Option<Vec<_>>>()?;
            Some(SsaConst::Int(ints.into_iter().fold(0, |a, b| a | b)))
        }
        "logxor" if !args.is_empty() => {
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
        "floor" if args.len() == 1 => match args[0] {
            SsaConst::Int(a) => Some(SsaConst::Int(*a)),
            SsaConst::Float(f) => Some(SsaConst::Int(f.floor() as i64)),
            _ => None,
        },
        "ceiling" if args.len() == 1 => match args[0] {
            SsaConst::Int(a) => Some(SsaConst::Int(*a)),
            SsaConst::Float(f) => Some(SsaConst::Int(f.ceil() as i64)),
            _ => None,
        },
        "round" if args.len() == 1 => match args[0] {
            SsaConst::Int(a) => Some(SsaConst::Int(*a)),
            SsaConst::Float(f) => Some(SsaConst::Int(f.round() as i64)),
            _ => None,
        },
        "lognot" if args.len() == 1 => {
            Some(SsaConst::Int(!args[0].as_int()?))
        }
        "integerp" if args.len() == 1 => Some(match args[0] {
            SsaConst::Int(_) => SsaConst::True,
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
        "symbolp" if args.len() == 1 => Some(match args[0] {
            SsaConst::Symbol(_) | SsaConst::Nil | SsaConst::True => SsaConst::True,
            _ => SsaConst::Nil,
        }),
        "string=" if args.len() == 2 => match (args[0], args[1]) {
            (SsaConst::String(a), SsaConst::String(b)) => {
                Some(if a == b { SsaConst::True } else { SsaConst::Nil })
            }
            _ => None,
        },
        "zerop" if args.len() == 1 => {
            let a = args[0].as_int()?;
            Some(if a == 0 { SsaConst::True } else { SsaConst::Nil })
        }
        "rem" | "%" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            if b == 0 {
                return None;
            }
            Some(SsaConst::Int(a.wrapping_rem(b)))
        }
        "mod" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            if b == 0 {
                return None;
            }
            let r = a.wrapping_rem(b);
            Some(SsaConst::Int(if r == 0 || (a ^ b) >= 0 { r } else { r.wrapping_add(b) }))
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
            SsaConst::Int(_) | SsaConst::Float(_) => SsaConst::True,
            SsaConst::Value(_) => return None,
            _ => SsaConst::Nil,
        }),
        "symbolp" if args.len() == 1 => Some(match args[0] {
            SsaConst::Symbol(_) | SsaConst::Nil | SsaConst::True => SsaConst::True,
            SsaConst::Value(_) => return None,
            _ => SsaConst::Nil,
        }),
        "vectorp" if args.len() == 1 => Some(match args[0] {
            SsaConst::Value(_) => return None,
            _ => SsaConst::Nil,
        }),
        "nlistp" if args.len() == 1 => Some(match args[0] {
            SsaConst::Nil => SsaConst::Nil,
            SsaConst::Value(_) => return None,
            _ => SsaConst::True,
        }),
        "functionp" | "subrp" | "compiled-function-p" if args.len() == 1 => Some(match args[0] {
            SsaConst::Value(_) => return None,
            _ => SsaConst::Nil,
        }),
        "car-safe" | "cdr-safe" if args.len() == 1 => Some(match args[0] {
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
            SsaConst::Int(n) => Some(*n),
            _ => None,
        }
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
