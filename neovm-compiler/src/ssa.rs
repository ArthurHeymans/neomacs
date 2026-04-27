use crate::effects::Effects;
use crate::ids::{BlockId, PrimaryMap, ValueId};
use crate::surface::SurfaceForm;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SsaFunction {
    pub name: Option<String>,
    pub values: PrimaryMap<ValueId, SsaValue>,
    pub blocks: PrimaryMap<BlockId, SsaBlock>,
    pub entry: Option<BlockId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SsaValue {
    pub kind: SsaValueKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SsaValueKind {
    BlockParam {
        block: BlockId,
        index: usize,
        name: Option<String>,
    },
    InstResult {
        block: BlockId,
        inst: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SsaBlock {
    pub params: Vec<ValueId>,
    pub instructions: Vec<SsaInst>,
    pub terminator: SsaTerminator,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SsaInst {
    pub result: Option<ValueId>,
    pub kind: SsaInstKind,
    pub effects: Effects,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SsaInstKind {
    Const(SsaConst),
    Quote(SurfaceForm),
    FunctionQuote(SurfaceForm),
    LexicalGet(String),
    LexicalSet { name: String, value: ValueId },
    SymbolGet(String),
    SymbolSet { name: String, value: ValueId },
    BindLexical { name: String, value: ValueId },
    BindDynamic { name: String, value: ValueId },
    DeclareSpecial(Vec<String>),
    CallNamed { name: String, args: Vec<ValueId> },
    Funcall { callee: ValueId, args: Vec<ValueId> },
    Apply { callee: ValueId, args: Vec<ValueId> },
    CatchBegin { tag: ValueId },
    CatchEnd,
    Throw { tag: ValueId, value: ValueId },
    ConditionCaseBegin { var: Option<String> },
    ConditionCaseHandler { pattern: SurfaceForm },
    ConditionCaseEnd,
    UnwindProtectBegin,
    UnwindProtectCleanup,
    UnwindProtectEnd,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SsaConst {
    Nil,
    True,
    Int(i64),
    Float(f64),
    String(String),
    Char(i64),
}

#[derive(Clone, Debug, PartialEq)]
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

impl Default for SsaTerminator {
    fn default() -> Self {
        Self::Unreachable
    }
}
