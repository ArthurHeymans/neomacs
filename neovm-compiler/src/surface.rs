use crate::source::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceForm {
    pub kind: SurfaceKind,
    pub span: Span,
}

impl SurfaceForm {
    pub fn new(kind: SurfaceKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn symbol_name(&self) -> Option<&str> {
        match &self.kind {
            SurfaceKind::Atom(SurfaceAtom::Symbol(name)) => Some(name),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceKind {
    Atom(SurfaceAtom),
    List(Vec<SurfaceForm>),
    DottedList(Vec<SurfaceForm>, Box<SurfaceForm>),
    Vector(Vec<SurfaceForm>),
    Quote(Box<SurfaceForm>),
    FunctionQuote(Box<SurfaceForm>),
    Backquote(Box<SurfaceForm>),
    Comma(Box<SurfaceForm>),
    CommaAt(Box<SurfaceForm>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceAtom {
    Nil,
    True,
    Symbol(String),
    Int(i64),
    Float(f64),
    String(String),
    Char(i64),
}

impl SurfaceAtom {
    pub fn symbol(name: impl Into<String>) -> Self {
        let name = name.into();
        match name.as_str() {
            "nil" => Self::Nil,
            "t" => Self::True,
            _ => Self::Symbol(name),
        }
    }
}
