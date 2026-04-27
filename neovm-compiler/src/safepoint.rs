use crate::ids::{PrimaryMap, RegId, SafepointId};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SafepointTable {
    pub entries: PrimaryMap<SafepointId, SafepointEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafepointEntry {
    pub live_roots: Vec<RegId>,
}
