use num_enum::{IntoPrimitive, TryFromPrimitive};
use strum::{EnumString, IntoStaticStr};

#[repr(u8)]
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
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

impl BufferTextBackendKind {
    pub fn symbol_name(self) -> &'static str {
        self.into()
    }

    pub fn is_implemented(self) -> bool {
        matches!(self, Self::GapBuffer | Self::PieceTree)
    }
}
