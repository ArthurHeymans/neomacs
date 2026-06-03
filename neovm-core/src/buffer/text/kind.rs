use strum::{EnumString, IntoStaticStr};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum BufferTextBackendKind {
    #[default]
    GapBuffer,
    PieceTree,
    Rope,
}

impl BufferTextBackendKind {
    pub fn symbol_name(self) -> &'static str {
        self.into()
    }

    pub fn is_implemented(self) -> bool {
        matches!(self, Self::GapBuffer)
    }
}
