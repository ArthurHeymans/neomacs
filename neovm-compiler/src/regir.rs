use crate::ids::{PrimaryMap, RegId, SafepointId};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegFunction {
    pub registers: PrimaryMap<RegId, Reg>,
    pub instructions: Vec<RegInst>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reg {
    pub kind: RegKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegKind {
    LispValue,
    MachineWord,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegInst {
    pub kind: RegInstKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegInstKind {
    LoadNil { dst: RegId },
    Move { dst: RegId, src: RegId },
    Safepoint { id: SafepointId },
    Return { src: RegId },
}
