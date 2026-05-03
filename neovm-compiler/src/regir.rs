use crate::hir::LambdaList;
use crate::ids::{FunctionId, PrimaryMap, RegBlockId, RegId, SafepointId};
use crate::safepoint::SafepointTable;
use crate::ssa::{SsaConst, SsaLambdaTemplate};
use crate::surface::SurfaceForm;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RegModule {
    pub functions: PrimaryMap<FunctionId, RegFunction>,
    pub entry: Option<FunctionId>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RegFunction {
    pub name: Option<String>,
    pub lambda_list: LambdaList,
    pub entry_params: Vec<RegId>,
    pub registers: PrimaryMap<RegId, Reg>,
    pub blocks: PrimaryMap<RegBlockId, RegBlock>,
    pub entry: Option<RegBlockId>,
    pub safepoints: SafepointTable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reg {
    pub kind: RegKind,
    pub name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegKind {
    LispValue,
    MachineWord,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegBlock {
    pub instructions: Vec<RegInst>,
    pub terminator: RegTerminator,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegInst {
    pub kind: RegInstKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RegInstKind {
    LoadConst {
        dst: RegId,
        value: SsaConst,
    },
    Quote {
        dst: RegId,
        form: SurfaceForm,
    },
    FunctionQuote {
        dst: RegId,
        form: SurfaceForm,
    },
    Lambda {
        dst: RegId,
        template: SsaLambdaTemplate,
        captures: Vec<RegId>,
    },
    Move {
        dst: RegId,
        src: RegId,
    },
    LexicalGet {
        dst: RegId,
        name: String,
    },
    LexicalSet {
        dst: RegId,
        name: String,
        src: RegId,
    },
    MakeLexicalCell {
        dst: RegId,
        initial: RegId,
    },
    LexicalCellGet {
        dst: RegId,
        cell: RegId,
    },
    LexicalCellSet {
        dst: RegId,
        cell: RegId,
        src: RegId,
    },
    SymbolGet {
        dst: RegId,
        name: String,
    },
    SymbolSet {
        dst: RegId,
        name: String,
        src: RegId,
    },
    BindLexical {
        name: String,
        src: RegId,
    },
    BindDynamic {
        name: String,
        src: RegId,
    },
    UnbindDynamic {
        count: usize,
    },
    DeclareSpecial {
        names: Vec<String>,
    },
    CallNamed {
        dst: RegId,
        name: String,
        args: Vec<RegId>,
    },
    Funcall {
        dst: RegId,
        callee: RegId,
        args: Vec<RegId>,
    },
    Apply {
        dst: RegId,
        callee: RegId,
        args: Vec<RegId>,
    },
    CatchBegin {
        tag: RegId,
    },
    CatchEnd {
        dst: RegId,
    },
    Throw {
        tag: RegId,
        value: RegId,
    },
    ConditionCaseBegin {
        var: Option<String>,
    },
    ConditionCaseGetVar {
        dst: RegId,
    },
    ConditionCaseHandler {
        pattern: SurfaceForm,
    },
    ConditionCaseEnd {
        dst: RegId,
        body_result: Option<RegId>,
    },
    UnwindProtectBegin,
    UnwindProtectCleanup,
    UnwindProtectEnd {
        dst: RegId,
        body_result: Option<RegId>,
    },
    Safepoint {
        id: SafepointId,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum RegTerminator {
    Return(Option<RegId>),
    Jump {
        target: RegBlockId,
    },
    BranchIfNil {
        test: RegId,
        then_target: RegBlockId,
        else_target: RegBlockId,
    },
    Unreachable,
}

impl Default for RegTerminator {
    fn default() -> Self {
        Self::Unreachable
    }
}
