use num_enum::{IntoPrimitive, TryFromPrimitive};
use strum::{EnumIter, EnumString, IntoEnumIterator, IntoStaticStr};

#[repr(u8)]
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    EnumIter,
    EnumString,
    IntoPrimitive,
    IntoStaticStr,
    TryFromPrimitive,
)]
#[strum(serialize_all = "kebab-case")]
pub enum BufferTextBackendKind {
    #[default]
    GapBuffer = 0,
    PieceTree = 1,
    Rope = 2,
}

#[repr(u8)]
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    EnumIter,
    EnumString,
    IntoPrimitive,
    IntoStaticStr,
    TryFromPrimitive,
)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum ImplementedBufferTextBackendKind {
    #[default]
    GapBuffer = 0,
    PieceTree = 1,
    Rope = 2,
}

impl BufferTextBackendKind {
    pub fn variants() -> impl Iterator<Item = Self> {
        Self::iter()
    }

    pub fn symbol_name(self) -> &'static str {
        self.into()
    }

    pub fn is_implemented(self) -> bool {
        self.implemented().is_some()
    }

    pub(crate) fn implemented(self) -> Option<ImplementedBufferTextBackendKind> {
        match self {
            Self::GapBuffer => Some(ImplementedBufferTextBackendKind::GapBuffer),
            Self::PieceTree => Some(ImplementedBufferTextBackendKind::PieceTree),
            Self::Rope => Some(ImplementedBufferTextBackendKind::Rope),
        }
    }
}

impl ImplementedBufferTextBackendKind {
    pub(crate) fn variants() -> impl Iterator<Item = Self> {
        Self::iter()
    }

    pub(crate) fn non_gap_variants() -> impl Iterator<Item = Self> {
        Self::iter().filter(|kind| *kind != Self::GapBuffer)
    }

    pub(crate) fn symbol_name(self) -> &'static str {
        self.into()
    }

    pub(crate) fn public_kind(self) -> BufferTextBackendKind {
        self.into()
    }
}

impl TryFrom<BufferTextBackendKind> for ImplementedBufferTextBackendKind {
    type Error = BufferTextBackendKind;

    fn try_from(kind: BufferTextBackendKind) -> Result<Self, Self::Error> {
        kind.implemented().ok_or(kind)
    }
}

impl From<ImplementedBufferTextBackendKind> for BufferTextBackendKind {
    fn from(kind: ImplementedBufferTextBackendKind) -> Self {
        match kind {
            ImplementedBufferTextBackendKind::GapBuffer => Self::GapBuffer,
            ImplementedBufferTextBackendKind::PieceTree => Self::PieceTree,
            ImplementedBufferTextBackendKind::Rope => Self::Rope,
        }
    }
}
