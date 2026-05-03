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
    #[token("\"", lex_string)]
    String(String),
    #[regex(
        r#"\?(?:\\[CMASH]-)*(?:\\.|[^\s()\[\]'"`,;])"#,
        parse_char,
        priority = 4
    )]
    Char(i64),
    #[regex(r"[+-]?[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?", parse_float)]
    Float(f64),
    #[regex(r"[+-]?[0-9]+", parse_int, priority = 4)]
    Int(i64),
    #[regex(r#"[^\s()\[\]'"`,;]+"#, lex_symbol, priority = 1)]
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
    let mut raw_tokens = Vec::new();
    while let Some(result) = lexer.next() {
        let range = lexer.span();
        let span = source.span(range.start, range.end);
        match result {
            Ok(kind) => raw_tokens.push(Token {
                kind,
                span,
                text: lexer.slice().to_string(),
            }),
            Err(()) => diagnostics.push(Diagnostic::error("invalid reader token").with_span(span)),
        }
    }

    // Post-process: Logos can't match certain character literal forms:
    // 1. ?( ?) ?[ ?] ?' ?` ?, — delimiter chars claimed by other tokens
    // 2. ?; — semicolon claimed by Comment
    // 3. ?" — double quote claimed by String
    // 4. ?\C-\; — compound modifier + escaped semicolon, where Logos's
    //    regex can't extend the match past \C-\ to consume \;
    let mut tokens = Vec::with_capacity(raw_tokens.len());
    let mut i = 0;
    while i < raw_tokens.len() {
        let tok = &raw_tokens[i];

        // Case 4: Char(?\C-\) followed by Comment(;...) — compound escape.
        // The Char regex matched ?\C-\ but \; was consumed by the Comment.
        // The Comment text is ;...] — take only the ; as the escaped char,
        // then re-lex the remainder (e.g. the ] in ;]) as new tokens.
        if matches!(&tok.kind, TokenKind::Char(_))
            && tok.text.ends_with('\\')
            && i + 1 < raw_tokens.len()
        {
            let next = &raw_tokens[i + 1];
            if matches!(&next.kind, TokenKind::Comment) && next.text.starts_with(';') {
                let full_text = format!("{};", tok.text);
                let full_value = parse_char_code(&full_text);
                if let Some(value) = full_value {
                    tokens.push(Token {
                        kind: TokenKind::Char(value),
                        span: tok.span,
                        text: full_text,
                    });
                    // Re-lex any remaining comment text after the leading ;
                    let remainder = &next.text[1..];
                    if !remainder.is_empty() {
                        let mut sub_lexer = TokenKind::lexer(remainder);
                        while let Some(sub_result) = sub_lexer.next() {
                            if let Ok(sub_kind) = sub_result {
                                // Re-lexed tokens don't have accurate source spans,
                                // but for structural tokens like ] this is fine.
                                tokens.push(Token {
                                    kind: sub_kind,
                                    span: next.span,
                                    text: sub_lexer.slice().to_string(),
                                });
                            }
                        }
                    }
                    i += 2;
                    continue;
                }
            }
        }

        // Cases 1-3: bare `?` Symbol followed by a delimiter/Comment/String.
        if matches!(&tok.kind, TokenKind::Symbol(s) if s == "?") && i + 1 < raw_tokens.len() {
            let next = &raw_tokens[i + 1];
            let ch: Option<char> = match &next.kind {
                TokenKind::LParen => Some('('),
                TokenKind::RParen => Some(')'),
                TokenKind::LBracket => Some('['),
                TokenKind::RBracket => Some(']'),
                TokenKind::Quote => Some('\''),
                TokenKind::Backquote => Some('`'),
                TokenKind::Comma => Some(','),
                TokenKind::Comment if next.text.starts_with(';') => Some(';'),
                TokenKind::String(_) => Some('"'),
                _ => None,
            };
            if let Some(ch) = ch {
                let span = tok.span;
                tokens.push(Token {
                    kind: TokenKind::Char(ch as i64),
                    span,
                    text: format!("?{ch}"),
                });
                i += 2;
                continue;
            }
        }
        // Case 5: Escaped chars in symbol names. Elisp allows `\` followed
        // by any delimiter character within a symbol name. The Symbol regex
        // stops at delimiters, so `\`` becomes Symbol(`\`) + Backquote.
        // Merge them into a single Symbol whose name has the backslash removed
        // (the escaped char IS the symbol character).
        // Also handles `\,@` where CommaAt(`,@`) is the next token — the
        // escaped char is `,` and `@` continues the symbol name.
        if matches!(&tok.kind, TokenKind::Symbol(s) if s.ends_with('\\'))
            && i + 1 < raw_tokens.len()
        {
            let next = &raw_tokens[i + 1];
            let escaped_text: Option<&str> = match &next.kind {
                TokenKind::Backquote => Some("`"),
                TokenKind::Comma => Some(","),
                TokenKind::CommaAt => Some(",@"),
                TokenKind::Quote => Some("'"),
                TokenKind::LParen => Some("("),
                TokenKind::RParen => Some(")"),
                TokenKind::LBracket => Some("["),
                TokenKind::RBracket => Some("]"),
                TokenKind::Comment if next.text.starts_with(';') => Some(";"),
                TokenKind::String(_) => Some("\""),
                _ => None,
            };
            if let Some(escaped) = escaped_text {
                // Remove trailing backslash from symbol name, append the escaped text
                let base = &tok.text[..tok.text.len() - 1];
                let decoded = format!("{base}{escaped}");
                tokens.push(Token {
                    kind: TokenKind::Symbol(decoded.clone()),
                    span: tok.span,
                    text: decoded,
                });
                i += 2;
                continue;
            }
        }
        // Case 6: #x, #o, #b numeric literals.
        // These are lexed as Symbol("#x1f") etc. Convert to Int/Float.
        if let TokenKind::Symbol(ref name) = tok.kind {
            if let Some(value) = try_parse_hash_number(name) {
                tokens.push(Token {
                    kind: TokenKind::Int(value),
                    span: tok.span,
                    text: name.clone(),
                });
                i += 1;
                continue;
            }
        }
        tokens.push(raw_tokens[i].clone());
        i += 1;
    }
    tokens
}

/// Delimiter bytes that terminate symbol names (without `\` escape).
const DELIMITER_BYTES: &[u8] = b" \t\r\n\x0c\x0b()[]{}'\"`,;";

fn is_symbol_byte(b: u8) -> bool {
    !DELIMITER_BYTES.contains(&b)
}

/// Custom symbol lexer that handles `\` escape sequences within symbol names.
/// In elisp, `\` followed by any character includes that character in the
/// symbol name. So `\,` is a symbol named `,`, `\,.` is a symbol named `,.`,
/// and `\,@` is a symbol named `,@`.
fn lex_symbol(lexer: &mut logos::Lexer<'_, TokenKind>) -> String {
    let initial = lexer.slice();
    let remainder = lexer.remainder();
    let bytes = remainder.as_bytes();
    let mut i = 0;
    let mut raw_end = 0; // extra bytes consumed from remainder

    // If the initial match ended with `\`, or if we encounter `\` during
    // extension, consume the next char (escaped delimiter) + more symbol chars.
    let needs_escape_continuation = initial.as_bytes().last() == Some(&b'\\');

    if !needs_escape_continuation && !bytes.contains(&b'\\') {
        // Fast path: no backslash in initial or remainder, just return as-is
        return initial.to_string();
    }

    // Slow path: need to handle `\` escapes
    let mut decoded = String::new();
    let mut chars = initial.chars();
    let mut pending_backslash = false;

    // Process initial match
    while let Some(ch) = chars.next() {
        if pending_backslash {
            decoded.push(ch);
            pending_backslash = false;
        } else if ch == '\\' {
            // Check if this is an escape (backslash at end of initial, or
            // followed by a delimiter in the remainder)
            if chars.as_str().is_empty() && i < bytes.len() && !is_symbol_byte(bytes[i]) {
                // Backslash at end of initial, next byte is a delimiter — escape it
                decoded.push(bytes[i] as char);
                raw_end += 1;
                i += 1;
                // Continue consuming symbol chars after the escaped delimiter
                while i < bytes.len() {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        decoded.push(bytes[i + 1] as char);
                        i += 2;
                        raw_end += 2;
                    } else if is_symbol_byte(bytes[i]) {
                        decoded.push(bytes[i] as char);
                        i += 1;
                        raw_end += 1;
                    } else {
                        break;
                    }
                }
            } else {
                // Backslash followed by symbol char — it's literal `\` in name
                decoded.push('\\');
                pending_backslash = false;
            }
        } else {
            decoded.push(ch);
        }
    }

    // Process remainder
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            // Backslash escape: next byte is part of the symbol name
            decoded.push(bytes[i + 1] as char);
            i += 2;
            raw_end += 2;
        } else if is_symbol_byte(bytes[i]) {
            decoded.push(bytes[i] as char);
            i += 1;
            raw_end += 1;
        } else {
            break;
        }
    }

    if raw_end > 0 {
        lexer.bump(raw_end);
    }

    decoded
}

fn lex_string(lexer: &mut logos::Lexer<'_, TokenKind>) -> String {
    let remainder = lexer.remainder();
    let bytes = remainder.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                lexer.bump(i + 1);
                let body = &remainder[..i];
                return decode_escapes(body);
            }
            b'\\' => {
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }
    lexer.bump(remainder.len());
    String::new()
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

/// Try to parse #x (hex), #o (octal), #b (binary) numeric literals.
fn try_parse_hash_number(name: &str) -> Option<i64> {
    let rest = name.strip_prefix('#')?;
    let (prefix, digits) =
        if let Some(d) = rest.strip_prefix('x').or_else(|| rest.strip_prefix('X')) {
            (16, d)
        } else if let Some(d) = rest.strip_prefix('o').or_else(|| rest.strip_prefix('O')) {
            (8, d)
        } else if let Some(d) = rest.strip_prefix('b').or_else(|| rest.strip_prefix('B')) {
            (2, d)
        } else {
            return None;
        };
    if digits.is_empty() {
        return None;
    }
    i64::from_str_radix(digits, prefix).ok()
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
        // Parse #(...) as a list — Emacs uses this for string-with-text-properties
        // and byte-code objects. We treat the contents as a normal list.
        self.builder.start_node(SyntaxKind::List.into());
        self.bump(); // consume #(
        loop {
            self.bump_trivia();
            let Some(token) = self.peek().cloned() else {
                self.error(start_span, "unterminated #(...) form");
                self.builder.finish_node();
                return false;
            };
            if matches!(token.kind, TokenKind::RParen) {
                self.bump();
                self.builder.finish_node();
                return true;
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
            SyntaxKind::Int => {
                // Text may be a #x/#o/#b literal that doesn't parse directly.
                let value = text
                    .parse::<i64>()
                    .ok()
                    .or_else(|| try_parse_hash_number(text))?;
                SurfaceAtom::Int(value)
            }
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
        let ch = decode_single_escape(rest)?;
        return Some(control_code(ch) as i64);
    }
    if let Some(rest) = text.strip_prefix("M-") {
        let ch = decode_single_escape(rest)?;
        return Some(0x0800_0000 + ch as i64);
    }
    Some(decode_single_escape_value(text.chars().next()?) as i64)
}

fn decode_single_escape(text: &str) -> Option<char> {
    if let Some(rest) = text.strip_prefix('\\') {
        Some(decode_single_escape_value(rest.chars().next()?))
    } else {
        text.chars().next()
    }
}

fn decode_single_escape_value(ch: char) -> char {
    match ch {
        'a' => '\x07',
        'b' => '\x08',
        'f' => '\x0c',
        'n' => '\n',
        'r' => '\r',
        's' => ' ',
        't' => '\t',
        'v' => '\x0b',
        'e' => '\x1b',
        other => other,
    }
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
    fn parses_hash_paren_as_list() {
        let output = read("#(1 2)");
        assert_eq!(output.diagnostics, Vec::new());
        assert_eq!(output.forms.len(), 1);
    }

    #[test]
    fn reports_unterminated_list() {
        let output = read("(a b");
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.is_error() && diagnostic.message.contains("unterminated list")
        }));
    }

    #[test]
    fn reads_escaped_symbols() {
        // `\,` in elisp is the symbol named ","
        // `'\,` is the quoted symbol ","
        let output = read("'\\,"); // Rust raw string: ' \ ,
        assert_eq!(output.diagnostics, Vec::new());
        assert_eq!(output.forms.len(), 1);
    }

    #[test]
    fn reads_hash_hex_octal_binary_literals() {
        let output = read("#x1f #o37 #b11111");
        assert_eq!(output.diagnostics, Vec::new());
        assert_eq!(output.forms.len(), 3);
        assert_eq!(
            output.forms[0].kind,
            SurfaceKind::Atom(SurfaceAtom::Int(31))
        );
        assert_eq!(
            output.forms[1].kind,
            SurfaceKind::Atom(SurfaceAtom::Int(31))
        );
        assert_eq!(
            output.forms[2].kind,
            SurfaceKind::Atom(SurfaceAtom::Int(31))
        );
    }

    #[test]
    fn reads_hash_hex_uppercase_prefix() {
        let output = read("#XFF #O17 #B1010");
        assert_eq!(output.diagnostics, Vec::new());
        assert_eq!(output.forms.len(), 3);
        assert_eq!(
            output.forms[0].kind,
            SurfaceKind::Atom(SurfaceAtom::Int(255))
        );
        assert_eq!(
            output.forms[1].kind,
            SurfaceKind::Atom(SurfaceAtom::Int(15))
        );
        assert_eq!(
            output.forms[2].kind,
            SurfaceKind::Atom(SurfaceAtom::Int(10))
        );
    }
}
