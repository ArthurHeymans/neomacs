use rowan::ast::AstNode;

use crate::syntax::{ElispLanguage, SyntaxElement, SyntaxKind, SyntaxNode};

macro_rules! ast_node {
    ($name:ident, $kind:pat) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub struct $name {
            syntax: SyntaxNode,
        }

        impl AstNode for $name {
            type Language = ElispLanguage;

            fn can_cast(kind: SyntaxKind) -> bool {
                matches!(kind, $kind)
            }

            fn cast(node: SyntaxNode) -> Option<Self> {
                Self::can_cast(node.kind()).then_some(Self { syntax: node })
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.syntax
            }
        }
    };
}

ast_node!(Root, SyntaxKind::Root);
ast_node!(List, SyntaxKind::List);
ast_node!(Vector, SyntaxKind::Vector);
ast_node!(
    PrefixForm,
    SyntaxKind::Quote
        | SyntaxKind::FunctionQuote
        | SyntaxKind::Backquote
        | SyntaxKind::Comma
        | SyntaxKind::CommaAt
);
ast_node!(ErrorForm, SyntaxKind::Error);

impl Root {
    pub fn forms(&self) -> impl Iterator<Item = SyntaxElement> + '_ {
        significant_children(self.syntax())
    }
}

impl List {
    pub fn elements(&self) -> impl Iterator<Item = SyntaxElement> + '_ {
        significant_children(self.syntax())
    }
}

impl Vector {
    pub fn elements(&self) -> impl Iterator<Item = SyntaxElement> + '_ {
        significant_children(self.syntax())
    }
}

impl PrefixForm {
    pub fn prefix_kind(&self) -> SyntaxKind {
        self.syntax().kind()
    }

    pub fn inner(&self) -> Option<SyntaxElement> {
        significant_children(self.syntax()).nth(1)
    }
}

pub fn significant_children(node: &SyntaxNode) -> impl Iterator<Item = SyntaxElement> + '_ {
    node.children_with_tokens()
        .filter(|element| !matches!(element.kind(), SyntaxKind::Whitespace | SyntaxKind::Comment))
}
