use crate::effects::Effects;
use crate::ids::{BlockId, PrimaryMap, ValueId};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SsaFunction {
    pub blocks: PrimaryMap<BlockId, SsaBlock>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SsaBlock {
    pub params: Vec<ValueId>,
    pub instructions: Vec<SsaInst>,
    pub terminator: SsaTerminator,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SsaInst {
    pub result: Option<ValueId>,
    pub kind: SsaInstKind,
    pub effects: Effects,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SsaInstKind {
    Placeholder,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SsaTerminator {
    Return(Option<ValueId>),
    Jump {
        target: BlockId,
        args: Vec<ValueId>,
    },
    BranchIfNil {
        test: ValueId,
        then_target: BlockId,
        then_args: Vec<ValueId>,
        else_target: BlockId,
        else_args: Vec<ValueId>,
    },
    Unreachable,
}
