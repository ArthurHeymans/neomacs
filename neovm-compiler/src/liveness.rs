use std::collections::{HashMap, HashSet};

use crate::ids::{BlockId, ValueId};
use crate::ssa::{SsaFunction, SsaInstKind, SsaTerminator};

#[derive(Clone, Debug, Default)]
pub struct SsaSafepointLiveness {
    roots_by_inst: HashMap<(BlockId, usize), Vec<ValueId>>,
}

impl SsaSafepointLiveness {
    pub fn compute(function: &SsaFunction) -> Self {
        let mut live_in = HashMap::<BlockId, HashSet<ValueId>>::new();
        for (block_id, _) in function.blocks.iter() {
            live_in.insert(block_id, HashSet::new());
        }

        let mut changed = true;
        while changed {
            changed = false;
            for (block_id, block) in function.blocks.iter().rev() {
                let mut live = terminator_live_in(function, &block.terminator, &live_in);
                for inst in block.instructions.iter().rev() {
                    if let Some(result) = inst.result {
                        live.remove(&result);
                    }
                    for value in inst_uses(&inst.kind) {
                        live.insert(value);
                    }
                }
                if live_in.get(&block_id) != Some(&live) {
                    live_in.insert(block_id, live);
                    changed = true;
                }
            }
        }

        let mut roots_by_inst = HashMap::new();
        for (block_id, block) in function.blocks.iter() {
            let mut live = terminator_live_in(function, &block.terminator, &live_in);
            for (index, inst) in block.instructions.iter().enumerate().rev() {
                let uses = inst_uses(&inst.kind);
                let mut roots = live.clone();
                if let Some(result) = inst.result {
                    roots.remove(&result);
                }
                roots.extend(uses.iter().copied());
                roots_by_inst.insert((block_id, index), sorted_values(roots));

                if let Some(result) = inst.result {
                    live.remove(&result);
                }
                live.extend(uses);
            }
        }

        Self { roots_by_inst }
    }

    pub fn roots_for(&self, block: BlockId, inst: usize) -> &[ValueId] {
        self.roots_by_inst
            .get(&(block, inst))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

fn terminator_live_in(
    function: &SsaFunction,
    terminator: &SsaTerminator,
    live_in: &HashMap<BlockId, HashSet<ValueId>>,
) -> HashSet<ValueId> {
    match terminator {
        SsaTerminator::Return(value) => value.iter().copied().collect(),
        SsaTerminator::Jump { target, args } => edge_live(function, *target, args, live_in),
        SsaTerminator::BranchIfNil {
            test,
            then_target,
            then_args,
            else_target,
            else_args,
        } => {
            let mut live = edge_live(function, *then_target, then_args, live_in);
            live.extend(edge_live(function, *else_target, else_args, live_in));
            live.insert(*test);
            live
        }
        SsaTerminator::Unreachable => HashSet::new(),
    }
}

fn edge_live(
    function: &SsaFunction,
    target: BlockId,
    args: &[ValueId],
    live_in: &HashMap<BlockId, HashSet<ValueId>>,
) -> HashSet<ValueId> {
    let mut live = HashSet::new();
    let Some(target_live) = live_in.get(&target) else {
        return live;
    };
    let Some(target_block) = function.blocks.get(target) else {
        return live;
    };
    for value in target_live {
        if let Some(index) = target_block.params.iter().position(|param| param == value) {
            if let Some(arg) = args.get(index) {
                live.insert(*arg);
            }
        } else {
            live.insert(*value);
        }
    }
    live
}

pub fn inst_uses(kind: &SsaInstKind) -> Vec<ValueId> {
    match kind {
        SsaInstKind::LexicalSet { value, .. }
        | SsaInstKind::SymbolSet { value, .. }
        | SsaInstKind::BindLexical { value, .. }
        | SsaInstKind::BindDynamic { value, .. }
        | SsaInstKind::CatchBegin { tag: value } => vec![*value],
        SsaInstKind::Throw { tag, value } => vec![*tag, *value],
        SsaInstKind::CallNamed { args, .. } => args.clone(),
        SsaInstKind::Funcall { callee, args } | SsaInstKind::Apply { callee, args } => {
            let mut uses = Vec::with_capacity(args.len() + 1);
            uses.push(*callee);
            uses.extend(args);
            uses
        }
        SsaInstKind::Const(_)
        | SsaInstKind::Quote(_)
        | SsaInstKind::FunctionQuote(_)
        | SsaInstKind::Lambda(_)
        | SsaInstKind::LexicalGet(_)
        | SsaInstKind::SymbolGet(_)
        | SsaInstKind::UnbindDynamic { .. }
        | SsaInstKind::DeclareSpecial(_)
        | SsaInstKind::CatchEnd
        | SsaInstKind::ConditionCaseBegin { .. }
        | SsaInstKind::ConditionCaseHandler { .. }
        | SsaInstKind::ConditionCaseEnd
        | SsaInstKind::UnwindProtectBegin
        | SsaInstKind::UnwindProtectCleanup
        | SsaInstKind::UnwindProtectEnd => Vec::new(),
    }
}

fn sorted_values(values: HashSet<ValueId>) -> Vec<ValueId> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort();
    values
}
