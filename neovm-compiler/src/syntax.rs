use num_enum::{FromPrimitive, IntoPrimitive};
use rowan::{GreenNode, Language, SyntaxKind as RowanSyntaxKind};

#[repr(u16)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, IntoPrimitive, FromPrimitive,
)]
pub enum SyntaxKind {
    Root,
    List,
    Vector,
    DottedList,
    Quote,
    FunctionQuote,
    Backquote,
    Comma,
    CommaAt,
    #[num_enum(default)]
    Error,
    LParen,
    RParen,
    LBracket,
    RBracket,
    HashLParen,
    Dot,
    Prefix,
    Symbol,
    Int,
    Float,
    String,
    Char,
    Whitespace,
    Comment,
}

impl From<SyntaxKind> for RowanSyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(u16::from(kind))
    }
}

impl From<RowanSyntaxKind> for SyntaxKind {
    fn from(kind: RowanSyntaxKind) -> Self {
        kind.0.into()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ElispLanguage {}

impl Language for ElispLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: RowanSyntaxKind) -> Self::Kind {
        raw.into()
    }

    fn kind_to_raw(kind: Self::Kind) -> RowanSyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<ElispLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<ElispLanguage>;
pub type SyntaxElement = rowan::SyntaxElement<ElispLanguage>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxTree {
    green: GreenNode,
}

impl SyntaxTree {
    pub fn new(green: GreenNode) -> Self {
        Self { green }
    }

    pub fn root(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    pub fn debug_dump(&self) -> String {
        format!("{:#?}", self.root())
    }
}
