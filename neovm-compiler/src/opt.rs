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

/// Run the default optimization pipeline on a single SSA function.
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
        let diags = crate::verify::verify_ssa(function);
        assert!(
            diags.is_empty(),
            "optimization produced invalid SSA: {diags:?}"
        );
    }
    any_changed
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
            block.instructions.clear();
            block.terminator = SsaTerminator::Unreachable;
            changed = true;
        }
    }
    changed
}

fn const_to_bool(c: &SsaConst) -> Option<bool> {
    match c {
        SsaConst::Nil => Some(false),
        SsaConst::True => Some(true),
        SsaConst::Int(0) => Some(false),
        SsaConst::Int(_) => Some(true),
        _ => None,
    }
}

pub fn constant_folding(function: &mut SsaFunction) -> OptOutput {
    let mut const_map: HashMap<ValueId, SsaConst> = HashMap::new();
    let subst: HashMap<ValueId, ValueId> = HashMap::new();
    let mut changed = false;

    // Collect all existing constants.
    for (_block_id, block) in function.blocks.iter() {
        for (_inst_idx, inst) in block.instructions.iter().enumerate() {
            if let SsaInstKind::Const(c) = &inst.kind {
                if let Some(result) = inst.result {
                    const_map.insert(result, c.clone());
                }
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
        match &block.terminator {
            SsaTerminator::BranchIfNil {
                then_target,
                then_args,
                else_target,
                else_args,
                ..
            } => {
                let (target, args) = if is_nil {
                    (*then_target, then_args.clone())
                } else {
                    (*else_target, else_args.clone())
                };
                block.terminator = SsaTerminator::Jump { target, args };
                changed = true;
            }
            _ => {}
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

fn try_fold_call_named(name: &str, args: &[&SsaConst]) -> Option<SsaConst> {
    match name {
        "+" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            Some(SsaConst::Int(a.wrapping_add(b)))
        }
        "-" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            Some(SsaConst::Int(a.wrapping_sub(b)))
        }
        "-" if args.len() == 1 => {
            let a = args[0].as_int()?;
            Some(SsaConst::Int(a.wrapping_neg()))
        }
        "*" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            Some(SsaConst::Int(a.wrapping_mul(b)))
        }
        "/" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            if b == 0 {
                return None;
            }
            Some(SsaConst::Int(a.wrapping_div(b)))
        }
        "=" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            Some(if a == b {
                SsaConst::True
            } else {
                SsaConst::Nil
            })
        }
        "<" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            Some(if a < b { SsaConst::True } else { SsaConst::Nil })
        }
        ">" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            Some(if a > b { SsaConst::True } else { SsaConst::Nil })
        }
        "<=" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            Some(if a <= b {
                SsaConst::True
            } else {
                SsaConst::Nil
            })
        }
        ">=" if args.len() == 2 => {
            let (a, b) = (args[0].as_int()?, args[1].as_int()?);
            Some(if a >= b {
                SsaConst::True
            } else {
                SsaConst::Nil
            })
        }
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
        "1+" if args.len() == 1 => {
            let a = args[0].as_int()?;
            Some(SsaConst::Int(a.wrapping_add(1)))
        }
        "1-" if args.len() == 1 => {
            let a = args[0].as_int()?;
            Some(SsaConst::Int(a.wrapping_sub(1)))
        }
        "null" | "not" if args.len() == 1 => match const_to_bool(args[0]) {
            Some(false) => Some(SsaConst::True),
            Some(true) => Some(SsaConst::Nil),
            None => None,
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
                    .map_or(true, |r| *use_counts.get(&r).unwrap_or(&0) > 0);
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
            if let SsaTerminator::Jump { target, args } = &block.terminator {
                if args.is_empty() {
                    if let Some(pred_list) = preds.get(target) {
                        if pred_list.len() == 1 && pred_list[0] == bid {
                            if function.blocks[*target].params.is_empty() {
                                merge = Some((bid, *target));
                                break;
                            }
                        }
                    }
                }
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
            if let SsaTerminator::Jump { target, .. } = &mut block.terminator {
                if *target == dst {
                    *target = src;
                }
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
                    let resolved: Vec<ValueId> = args.iter().map(|a| resolve(a)).collect();
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
