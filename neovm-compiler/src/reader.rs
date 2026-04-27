use logos::Logos;
use rowan::GreenNodeBuilder;
use rowan::ast::AstNode;

use crate::ast::{PrefixForm, Root};
use crate::diagnostic::Diagnostic;
use crate::source::{SourceFile, Span};
use crate::surface::{SurfaceAtom, SurfaceForm, SurfaceKind};
use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxTree};

#[derive(Clone, Debug, PartialEq)]
pub struct ReaderOutput {
    pub syntax: SyntaxTree,
    pub forms: Vec<SurfaceForm>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, PartialEq)]
struct Token {
    kind: TokenKind,
    span: Span,
    text: String,
}

#[derive(Logos, Clone, Debug, PartialEq)]
enum TokenKind {
    #[regex(r"[ \t\r\n\x0c\x0b]+")]
    Whitespace,
    #[regex(r";[^\n]*", allow_greedy = true)]
    Comment,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("#(")]
    HashLParen,
    #[token("#'")]
    FunctionQuote,
    #[token("'")]
    Quote,
    #[token("`")]
    Backquote,
    #[token(",@")]
    CommaAt,
    #[token(",")]
    Comma,
    #[regex(r#""([^"\\]|\\.)*""#, parse_string)]
    String(String),
    #[regex(
        r#"\?(\\[^\s()\[\]'"`,;]+|[^\s()\[\]'"`,;])"#,
        parse_char,
        priority = 4
    )]
    Char(i64),
    #[regex(r"[+-]?[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?", parse_float)]
    Float(f64),
    #[regex(r"[+-]?[0-9]+", parse_int, priority = 4)]
    Int(i64),
    #[regex(r#"[^\s()\[\]'"`,;]+"#, |lexer| lexer.slice().to_string(), priority = 1)]
    Symbol(String),
}

impl TokenKind {
    fn syntax_kind(&self) -> SyntaxKind {
        match self {
            Self::Whitespace => SyntaxKind::Whitespace,
            Self::Comment => SyntaxKind::Comment,
            Self::LParen => SyntaxKind::LParen,
            Self::RParen => SyntaxKind::RParen,
            Self::LBracket => SyntaxKind::LBracket,
            Self::RBracket => SyntaxKind::RBracket,
            Self::HashLParen => SyntaxKind::HashLParen,
            Self::FunctionQuote | Self::Quote | Self::Backquote | Self::Comma | Self::CommaAt => {
                SyntaxKind::Prefix
            }
            Self::String(_) => SyntaxKind::String,
            Self::Char(_) => SyntaxKind::Char,
            Self::Float(_) => SyntaxKind::Float,
            Self::Int(_) => SyntaxKind::Int,
            Self::Symbol(name) if name == "." => SyntaxKind::Dot,
            Self::Symbol(_) => SyntaxKind::Symbol,
        }
    }

    fn is_trivia(&self) -> bool {
        matches!(self, Self::Whitespace | Self::Comment)
    }
}

pub fn read_source(source: &SourceFile) -> ReaderOutput {
    let mut diagnostics = Vec::new();
    let tokens = lex_source(source, &mut diagnostics);
    let mut parser = Parser {
        source,
        tokens,
        pos: 0,
        builder: GreenNodeBuilder::new(),
        diagnostics,
    };
    parser.parse_root();
    let syntax = SyntaxTree::new(parser.builder.finish());
    let surface = extract_surface_forms(source, &syntax);
    diagnostics = parser.diagnostics;
    diagnostics.extend(surface.diagnostics);
    ReaderOutput {
        syntax,
        forms: surface.forms,
        diagnostics,
    }
}

fn lex_source(source: &SourceFile, diagnostics: &mut Vec<Diagnostic>) -> Vec<Token> {
    let mut lexer = TokenKind::lexer(&source.text);
    let mut tokens = Vec::new();
    while let Some(result) = lexer.next() {
        let range = lexer.span();
        let span = source.span(range.start, range.end);
        match result {
            Ok(kind) => tokens.push(Token {
                kind,
                span,
                text: lexer.slice().to_string(),
            }),
            Err(()) => diagnostics.push(Diagnostic::error("invalid reader token").with_span(span)),
        }
    }
    tokens
}

fn parse_string(lexer: &mut logos::Lexer<'_, TokenKind>) -> String {
    let slice = lexer.slice();
    let body = &slice[1..slice.len() - 1];
    decode_escapes(body)
}

fn parse_char(lexer: &mut logos::Lexer<'_, TokenKind>) -> Option<i64> {
    parse_char_code(lexer.slice())
}

fn parse_float(lexer: &mut logos::Lexer<'_, TokenKind>) -> Option<f64> {
    lexer.slice().parse().ok()
}

fn parse_int(lexer: &mut logos::Lexer<'_, TokenKind>) -> Option<i64> {
    lexer.slice().parse().ok()
}

fn decode_escapes(body: &str) -> String {
    let mut out = String::new();
    let mut chars = body.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let Some(escaped) = chars.next() else {
                out.push(ch);
                break;
            };
            out.push(match escaped {
                'a' => '\x07',
                'b' => '\x08',
                'f' => '\x0c',
                'n' => '\n',
                'r' => '\r',
                's' => ' ',
                't' => '\t',
                'v' => '\x0b',
                'e' => '\x1b',
                '"' => '"',
                '\\' => '\\',
                other => other,
            });
        } else {
            out.push(ch);
        }
    }
    out
}

struct Parser<'a> {
    source: &'a SourceFile,
    tokens: Vec<Token>,
    pos: usize,
    builder: GreenNodeBuilder<'static>,
    diagnostics: Vec<Diagnostic>,
}

impl Parser<'_> {
    fn parse_root(&mut self) {
        self.builder.start_node(SyntaxKind::Root.into());
        while !self.is_eof() {
            self.bump_trivia();
            if self.is_eof() {
                break;
            }
            if !self.parse_form() {
                self.error_current("unexpected token");
                if self.is_eof() {
                    break;
                }
                self.bump();
            }
        }
        self.builder.finish_node();
    }

    fn parse_form(&mut self) -> bool {
        loop {
            self.bump_trivia();
            let Some(token) = self.peek().cloned() else {
                return false;
            };
            match token.kind {
                TokenKind::Whitespace | TokenKind::Comment => {
                    self.bump();
                    continue;
                }
                _ => break,
            }
        }
        let Some(token) = self.peek().cloned() else {
            return false;
        };
        match token.kind {
            TokenKind::LParen => self.parse_list(token.span),
            TokenKind::LBracket => self.parse_vector(token.span),
            TokenKind::HashLParen => self.parse_invalid_hash_list(token.span),
            TokenKind::Quote => self.parse_prefixed(token.span, SyntaxKind::Quote),
            TokenKind::FunctionQuote => self.parse_prefixed(token.span, SyntaxKind::FunctionQuote),
            TokenKind::Backquote => self.parse_prefixed(token.span, SyntaxKind::Backquote),
            TokenKind::Comma => self.parse_prefixed(token.span, SyntaxKind::Comma),
            TokenKind::CommaAt => self.parse_prefixed(token.span, SyntaxKind::CommaAt),
            TokenKind::RParen | TokenKind::RBracket => {
                self.error(token.span, "unexpected closing delimiter");
                false
            }
            TokenKind::Whitespace | TokenKind::Comment => {
                unreachable!("trivia handled by loop above")
            }
            TokenKind::String(_)
            | TokenKind::Char(_)
            | TokenKind::Float(_)
            | TokenKind::Int(_)
            | TokenKind::Symbol(_) => {
                self.bump();
                true
            }
        }
    }

    fn parse_prefixed(&mut self, prefix_span: Span, syntax_kind: SyntaxKind) -> bool {
        self.builder.start_node(syntax_kind.into());
        self.bump();
        if !self.parse_form() {
            self.error(prefix_span, "expected form after reader prefix");
            self.builder.finish_node();
            return false;
        }
        self.builder.finish_node();
        true
    }

    fn parse_list(&mut self, start_span: Span) -> bool {
        self.builder.start_node(SyntaxKind::List.into());
        self.bump();
        loop {
            self.bump_trivia();
            let Some(token) = self.peek().cloned() else {
                self.error(start_span, "unterminated list");
                self.builder.finish_node();
                return false;
            };
            match token.kind {
                TokenKind::RParen => {
                    self.bump();
                    self.builder.finish_node();
                    return true;
                }
                TokenKind::Symbol(ref name) if name == "." => {
                    self.bump();
                    if !self.parse_form() {
                        self.error(token.span, "expected dotted-list tail");
                        self.builder.finish_node();
                        return false;
                    }
                    self.bump_trivia();
                    let Some(close) = self.peek().cloned() else {
                        self.error(start_span, "unterminated dotted list");
                        self.builder.finish_node();
                        return false;
                    };
                    if !matches!(close.kind, TokenKind::RParen) {
                        self.error(close.span, "expected ')' after dotted-list tail");
                        self.builder.finish_node();
                        return false;
                    }
                    self.bump();
                    self.builder.finish_node();
                    return true;
                }
                _ => {
                    if !self.parse_form() {
                        self.error_current("unexpected token in list");
                        self.bump();
                    }
                }
            }
        }
    }

    fn parse_vector(&mut self, start_span: Span) -> bool {
        self.builder.start_node(SyntaxKind::Vector.into());
        self.bump();
        loop {
            self.bump_trivia();
            let Some(token) = self.peek().cloned() else {
                self.error(start_span, "unterminated vector");
                self.builder.finish_node();
                return false;
            };
            if matches!(token.kind, TokenKind::RBracket) {
                self.bump();
                self.builder.finish_node();
                return true;
            }
            if !self.parse_form() {
                self.error_current("unexpected token in vector");
                self.bump();
            }
        }
    }

    fn parse_invalid_hash_list(&mut self, start_span: Span) -> bool {
        self.builder.start_node(SyntaxKind::Error.into());
        self.error(start_span, "invalid read syntax: #(");
        self.bump();
        loop {
            self.bump_trivia();
            let Some(token) = self.peek().cloned() else {
                self.error(start_span, "unterminated invalid #(...) form");
                self.builder.finish_node();
                return false;
            };
            if matches!(token.kind, TokenKind::RParen) {
                self.bump();
                self.builder.finish_node();
                return false;
            }
            if !self.parse_form() {
                if self.is_eof() {
                    self.builder.finish_node();
                    return false;
                }
                self.bump();
            }
        }
    }

    fn bump_trivia(&mut self) {
        while self.peek().is_some_and(|token| token.kind.is_trivia()) {
            self.bump();
        }
    }

    fn bump(&mut self) {
        if self.is_eof() {
            return;
        }
        let token = &self.tokens[self.pos];
        self.builder
            .token(token.kind.syntax_kind().into(), token.text.as_str());
        self.pos += 1;
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn error_current(&mut self, message: impl Into<String>) {
        let span = self
            .peek()
            .map(|token| token.span)
            .unwrap_or_else(|| self.source.eof_span());
        self.error(span, message);
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(message).with_span(span));
    }
}

#[derive(Clone, Debug, PartialEq)]
struct SurfaceExtractOutput {
    forms: Vec<SurfaceForm>,
    diagnostics: Vec<Diagnostic>,
}

fn extract_surface_forms(source: &SourceFile, syntax: &SyntaxTree) -> SurfaceExtractOutput {
    let mut extractor = SurfaceExtractor {
        source,
        diagnostics: Vec::new(),
    };
    let root = syntax.root();
    let root = Root::cast(root).expect("reader always produces a root node");
    let forms = root
        .forms()
        .filter_map(|element| extractor.extract_form_element(element))
        .collect();
    SurfaceExtractOutput {
        forms,
        diagnostics: extractor.diagnostics,
    }
}

struct SurfaceExtractor<'a> {
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl SurfaceExtractor<'_> {
    fn extract_form(&mut self, node: &SyntaxNode) -> Option<SurfaceForm> {
        match node.kind() {
            SyntaxKind::List => self.extract_list(node),
            SyntaxKind::Vector => self.extract_vector(node),
            SyntaxKind::Quote
            | SyntaxKind::FunctionQuote
            | SyntaxKind::Backquote
            | SyntaxKind::Comma
            | SyntaxKind::CommaAt => self.extract_prefixed(node),
            SyntaxKind::Root
            | SyntaxKind::DottedList
            | SyntaxKind::Error
            | SyntaxKind::LParen
            | SyntaxKind::RParen
            | SyntaxKind::LBracket
            | SyntaxKind::RBracket
            | SyntaxKind::HashLParen
            | SyntaxKind::Dot
            | SyntaxKind::Prefix
            | SyntaxKind::Symbol
            | SyntaxKind::Int
            | SyntaxKind::Float
            | SyntaxKind::String
            | SyntaxKind::Char
            | SyntaxKind::Whitespace
            | SyntaxKind::Comment => {
                self.error(node_span(self.source, node), "unexpected syntax node");
                None
            }
        }
    }

    fn extract_form_element(
        &mut self,
        element: rowan::NodeOrToken<SyntaxNode, crate::syntax::SyntaxToken>,
    ) -> Option<SurfaceForm> {
        match element {
            rowan::NodeOrToken::Node(node) => self.extract_form(&node),
            rowan::NodeOrToken::Token(token) => self.extract_atom_token(&token),
        }
    }

    fn extract_list(&mut self, node: &SyntaxNode) -> Option<SurfaceForm> {
        let mut items = Vec::new();
        let mut dotted_tail = None;
        let mut saw_dot = false;
        for element in significant_children(node) {
            match element.kind() {
                SyntaxKind::LParen | SyntaxKind::RParen => {}
                SyntaxKind::Dot => {
                    saw_dot = true;
                }
                _ if saw_dot => {
                    if dotted_tail.is_some() {
                        self.error(
                            element_span(self.source, &element),
                            "extra form after dotted-list tail",
                        );
                    } else {
                        dotted_tail = self.extract_form_element(element);
                    }
                }
                _ => {
                    if let Some(form) = self.extract_form_element(element) {
                        items.push(form);
                    }
                }
            }
        }
        let span = node_span(self.source, node);
        if saw_dot {
            dotted_tail
                .map(|tail| SurfaceForm::new(SurfaceKind::DottedList(items, Box::new(tail)), span))
                .or_else(|| {
                    self.error(span, "missing dotted-list tail");
                    None
                })
        } else {
            Some(SurfaceForm::new(SurfaceKind::List(items), span))
        }
    }

    fn extract_vector(&mut self, node: &SyntaxNode) -> Option<SurfaceForm> {
        let mut items = Vec::new();
        for element in significant_children(node) {
            match element.kind() {
                SyntaxKind::LBracket | SyntaxKind::RBracket => {}
                _ => {
                    if let Some(form) = self.extract_form_element(element) {
                        items.push(form);
                    }
                }
            }
        }
        Some(SurfaceForm::new(
            SurfaceKind::Vector(items),
            node_span(self.source, node),
        ))
    }

    fn extract_prefixed(&mut self, node: &SyntaxNode) -> Option<SurfaceForm> {
        let prefix = PrefixForm::cast(node.clone()).expect("prefix node");
        let Some(inner_element) = prefix.inner() else {
            self.error(node_span(self.source, node), "reader prefix missing form");
            return None;
        };
        let inner = self.extract_form_element(inner_element)?;
        let kind = match prefix.prefix_kind() {
            SyntaxKind::Quote => SurfaceKind::Quote(Box::new(inner)),
            SyntaxKind::FunctionQuote => SurfaceKind::FunctionQuote(Box::new(inner)),
            SyntaxKind::Backquote => SurfaceKind::Backquote(Box::new(inner)),
            SyntaxKind::Comma => SurfaceKind::Comma(Box::new(inner)),
            SyntaxKind::CommaAt => SurfaceKind::CommaAt(Box::new(inner)),
            _ => unreachable!("not a prefix node"),
        };
        Some(SurfaceForm::new(kind, node_span(self.source, node)))
    }

    fn extract_atom_token(&mut self, token: &crate::syntax::SyntaxToken) -> Option<SurfaceForm> {
        let text = token.text();
        let atom = match token.kind() {
            SyntaxKind::Symbol => SurfaceAtom::symbol(text),
            SyntaxKind::Int => SurfaceAtom::Int(text.parse().ok()?),
            SyntaxKind::Float => SurfaceAtom::Float(text.parse().ok()?),
            SyntaxKind::String => SurfaceAtom::String(decode_escapes(&text[1..text.len() - 1])),
            SyntaxKind::Char => SurfaceAtom::Char(parse_char_code(text)?),
            SyntaxKind::Dot
            | SyntaxKind::LParen
            | SyntaxKind::RParen
            | SyntaxKind::LBracket
            | SyntaxKind::RBracket
            | SyntaxKind::HashLParen
            | SyntaxKind::Prefix
            | SyntaxKind::Whitespace
            | SyntaxKind::Comment => return None,
            _ => {
                self.error(
                    token_span(self.source, token),
                    "unexpected token in surface form",
                );
                return None;
            }
        };
        Some(SurfaceForm::new(
            SurfaceKind::Atom(atom),
            token_span(self.source, token),
        ))
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(message).with_span(span));
    }
}

fn significant_children(
    node: &SyntaxNode,
) -> impl Iterator<Item = rowan::NodeOrToken<SyntaxNode, crate::syntax::SyntaxToken>> + '_ {
    node.children_with_tokens()
        .filter(|element| !matches!(element.kind(), SyntaxKind::Whitespace | SyntaxKind::Comment))
}

fn node_span(source: &SourceFile, node: &SyntaxNode) -> Span {
    let range = node.text_range();
    source.span(range.start().into(), range.end().into())
}

fn token_span(source: &SourceFile, token: &crate::syntax::SyntaxToken) -> Span {
    let range = token.text_range();
    source.span(range.start().into(), range.end().into())
}

fn element_span(
    source: &SourceFile,
    element: &rowan::NodeOrToken<SyntaxNode, crate::syntax::SyntaxToken>,
) -> Span {
    let range = element.text_range();
    source.span(range.start().into(), range.end().into())
}

fn parse_char_code(text: &str) -> Option<i64> {
    let body = &text[1..];
    if let Some(stripped) = body.strip_prefix('\\') {
        parse_escaped_char_code(stripped)
    } else {
        Some(body.chars().next()? as i64)
    }
}

fn parse_escaped_char_code(text: &str) -> Option<i64> {
    if let Some(rest) = text.strip_prefix("C-M-") {
        return parse_control_meta_char(rest);
    }
    if let Some(rest) = text.strip_prefix("C-\\M-") {
        return parse_control_meta_char(rest);
    }
    if let Some(rest) = text.strip_prefix("M-C-") {
        return parse_control_meta_char(rest);
    }
    if let Some(rest) = text.strip_prefix("M-\\C-") {
        return parse_control_meta_char(rest);
    }
    if let Some(rest) = text.strip_prefix("C-") {
        return Some(control_code(rest.chars().next()?) as i64);
    }
    if let Some(rest) = text.strip_prefix("M-") {
        return Some(0x0800_0000 + rest.chars().next()? as i64);
    }
    Some(match text.chars().next()? {
        'a' => 7,
        'b' => 8,
        'f' => 12,
        'n' => 10,
        'r' => 13,
        's' => 32,
        't' => 9,
        'v' => 11,
        'e' => 27,
        other => other as i64,
    })
}

fn parse_control_meta_char(text: &str) -> Option<i64> {
    Some(0x0800_0000 + control_code(text.chars().next()?) as i64)
}

fn control_code(ch: char) -> u32 {
    (ch as u32) & 0x1f
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceId;

    fn read(text: &str) -> ReaderOutput {
        let source = SourceFile::new(SourceId::new(0), Some("test.el".to_string()), text.into());
        read_source(&source)
    }

    #[test]
    fn reads_lists_and_atoms() {
        let output = read("(+ 1 2)");
        assert_eq!(output.diagnostics, Vec::new());
        assert_eq!(output.forms.len(), 1);
        let SurfaceKind::List(items) = &output.forms[0].kind else {
            panic!("expected list");
        };
        assert_eq!(items[0].symbol_name(), Some("+"));
    }

    #[test]
    fn keeps_trivia_in_lossless_syntax_tree() {
        let output = read("(+ ; comment\n 1 2)");
        assert_eq!(output.diagnostics, Vec::new());
        let root = output.syntax.root();
        let has_comment = root.descendants_with_tokens().any(|element| {
            element
                .into_token()
                .is_some_and(|token| token.kind() == SyntaxKind::Comment)
        });
        let has_whitespace = root.descendants_with_tokens().any(|element| {
            element
                .into_token()
                .is_some_and(|token| token.kind() == SyntaxKind::Whitespace)
        });
        assert!(has_comment);
        assert!(has_whitespace);
    }

    #[test]
    fn reads_quote_syntax() {
        let output = read("'foo #'bar");
        assert_eq!(output.diagnostics, Vec::new());
        assert_eq!(output.forms.len(), 2);
        assert!(matches!(output.forms[0].kind, SurfaceKind::Quote(_)));
        assert!(matches!(
            output.forms[1].kind,
            SurfaceKind::FunctionQuote(_)
        ));
    }

    #[test]
    fn reads_dotted_list() {
        let output = read("(a b . c)");
        assert_eq!(output.diagnostics, Vec::new());
        assert!(matches!(
            output.forms[0].kind,
            SurfaceKind::DottedList(_, _)
        ));
    }

    #[test]
    fn reads_strings_and_chars() {
        let output = read(r#""a\nb" ?x ?\n"#);
        assert_eq!(output.diagnostics, Vec::new());
        assert_eq!(output.forms.len(), 3);
    }

    #[test]
    fn reads_common_character_escape_codes() {
        let output = read(r"?\s ?\C-a ?\M-a ?\C-\M-a");
        assert_eq!(output.diagnostics, Vec::new());
        let values = output
            .forms
            .iter()
            .map(|form| match form.kind {
                SurfaceKind::Atom(SurfaceAtom::Char(value)) => value,
                _ => panic!("expected char"),
            })
            .collect::<Vec<_>>();
        assert_eq!(values, vec![32, 1, 134_217_825, 134_217_729]);
    }

    #[test]
    fn reports_hash_paren_as_invalid_gnu_read_syntax() {
        let output = read("#(1 2)");
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("invalid read syntax: #("))
        );
        assert!(output.forms.is_empty());
    }

    #[test]
    fn reports_unterminated_list() {
        let output = read("(a b");
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.is_error() && diagnostic.message.contains("unterminated list")
        }));
    }
}
