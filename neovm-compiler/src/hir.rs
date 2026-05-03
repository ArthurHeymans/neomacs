use indexmap::IndexSet;

use crate::diagnostic::Diagnostic;
use crate::source::{SourceFile, Span};
use crate::surface::{SurfaceAtom, SurfaceForm, SurfaceKind};

#[derive(Clone, Debug, PartialEq)]
pub struct HirOutput {
    pub module: HirModule,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HirModule {
    pub lexical_binding: bool,
    pub items: Vec<HirItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HirItem {
    Expr(HirExpr),
    Defun(HirDefun),
}

#[derive(Clone, Debug, PartialEq)]
pub struct HirDefun {
    pub name: String,
    pub params: LambdaList,
    pub declarations: Vec<HirDeclaration>,
    pub body: HirExpr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HirExpr {
    pub kind: HirExprKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HirExprKind {
    Const(HirConst),
    Quote(Box<SurfaceForm>),
    FunctionQuote(Box<SurfaceForm>),
    LexicalGet(String),
    LexicalSet {
        name: String,
        value: Box<HirExpr>,
    },
    SymbolGet(String),
    SymbolSet {
        name: String,
        value: Box<HirExpr>,
    },
    If {
        test: Box<HirExpr>,
        then_expr: Box<HirExpr>,
        else_expr: Box<HirExpr>,
    },
    While {
        test: Box<HirExpr>,
        body: Box<HirExpr>,
    },
    Progn(Vec<HirExpr>),
    Let {
        mode: BindingMode,
        sequential: bool,
        declarations: Vec<HirDeclaration>,
        bindings: Vec<HirBinding>,
        body: Box<HirExpr>,
    },
    Lambda {
        params: LambdaList,
        declarations: Vec<HirDeclaration>,
        body: Box<HirExpr>,
    },
    Declare(Vec<HirDeclaration>),
    Catch {
        tag: Box<HirExpr>,
        body: Box<HirExpr>,
    },
    Throw {
        tag: Box<HirExpr>,
        value: Box<HirExpr>,
    },
    ConditionCase {
        var: Option<String>,
        body: Box<HirExpr>,
        handlers: Vec<HirConditionHandler>,
    },
    UnwindProtect {
        body: Box<HirExpr>,
        cleanup: Box<HirExpr>,
    },
    Funcall {
        callee: Box<HirExpr>,
        args: Vec<HirExpr>,
    },
    Apply {
        callee: Box<HirExpr>,
        args: Vec<HirExpr>,
    },
    CallNamed {
        name: String,
        args: Vec<HirExpr>,
    },
    CallValue {
        callee: Box<HirExpr>,
        args: Vec<HirExpr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum HirConst {
    Nil,
    True,
    Int(i64),
    Float(f64),
    String(String),
    Char(i64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingMode {
    Lexical,
    Dynamic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParamSection {
    Required,
    Optional,
    Rest,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LambdaList {
    pub required: Vec<String>,
    pub optional: Vec<String>,
    pub rest: Option<String>,
}

impl LambdaList {
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.required
            .iter()
            .chain(self.optional.iter())
            .chain(self.rest.iter())
    }

    pub fn binding_names(&self) -> Vec<String> {
        self.names().cloned().collect()
    }

    pub fn min_arity(&self) -> usize {
        self.required.len()
    }

    pub fn max_arity(&self) -> Option<usize> {
        self.rest
            .is_none()
            .then_some(self.required.len() + self.optional.len())
    }

    pub fn entry_arity(&self) -> usize {
        self.required.len() + self.optional.len() + usize::from(self.rest.is_some())
    }

    pub fn display_parts(&self) -> Vec<String> {
        let mut parts = self.required.clone();
        if !self.optional.is_empty() {
            parts.push("&optional".to_string());
            parts.extend(self.optional.clone());
        }
        if let Some(rest) = &self.rest {
            parts.push("&rest".to_string());
            parts.push(rest.clone());
        }
        parts
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HirBinding {
    pub name: String,
    pub mode: BindingMode,
    pub init: HirExpr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirDeclaration {
    Special(Vec<String>),
    Unknown { name: String, args: Vec<String> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct HirConditionHandler {
    pub pattern: SurfaceForm,
    pub body: HirExpr,
    pub span: Span,
}

pub fn lower_expanded_forms(
    source: &SourceFile,
    forms: Vec<SurfaceForm>,
    lexical_binding: bool,
) -> HirOutput {
    let mut lowerer = Lowerer {
        source,
        lexical_binding,
        scopes: Vec::new(),
        special_scopes: Vec::new(),
        declared_special: IndexSet::new(),
        diagnostics: Vec::new(),
    };
    let mut items = Vec::new();
    for form in forms {
        if let Some(item) = lowerer.lower_item(&form) {
            items.push(item);
        }
    }
    HirOutput {
        module: HirModule {
            lexical_binding,
            items,
        },
        diagnostics: lowerer.diagnostics,
    }
}

struct Lowerer<'a> {
    source: &'a SourceFile,
    lexical_binding: bool,
    scopes: Vec<IndexSet<String>>,
    special_scopes: Vec<IndexSet<String>>,
    /// Variables declared special by defvar/defconst at top level. Persists
    /// for the rest of the file, unlike special_scopes which are scoped.
    declared_special: IndexSet<String>,
    diagnostics: Vec<Diagnostic>,
}

impl Lowerer<'_> {
    fn lower_item(&mut self, form: &SurfaceForm) -> Option<HirItem> {
        if let Some(list) = list_items(form)
            && let Some(kind @ ("defun" | "defsubst")) =
                list.first().and_then(SurfaceForm::symbol_name)
        {
            return self.lower_defun(form, list, kind).map(HirItem::Defun);
        }
        self.lower_expr(form).map(HirItem::Expr)
    }

    fn lower_defun(
        &mut self,
        form: &SurfaceForm,
        list: &[SurfaceForm],
        kind: &str,
    ) -> Option<HirDefun> {
        if list.len() < 3 {
            self.error(form.span, format!("{kind} requires a name and arg list"));
            return None;
        }
        let Some(name) = list[1].symbol_name().map(str::to_string) else {
            self.error(list[1].span, format!("{kind} name must be a symbol"));
            return None;
        };
        let Some(params) = self.parse_param_list(&list[2]) else {
            return None;
        };
        let (declarations, body_forms) = self.split_leading_declarations(&list[3..]);
        self.push_special_scope(special_declared_names(&declarations));
        let lexical_params = params
            .names()
            .filter(|name| !self.is_special(name))
            .cloned()
            .collect::<Vec<_>>();
        self.push_scope(lexical_params);
        let body = self.lower_body(body_forms, form.span)?;
        self.pop_scope();
        self.pop_special_scope();
        Some(HirDefun {
            name,
            params,
            declarations,
            body,
            span: form.span,
        })
    }

    fn lower_expr(&mut self, form: &SurfaceForm) -> Option<HirExpr> {
        match &form.kind {
            SurfaceKind::Atom(atom) => Some(self.lower_atom(atom, form.span)),
            SurfaceKind::Quote(inner) => Some(HirExpr {
                kind: HirExprKind::Quote(inner.clone()),
                span: form.span,
            }),
            SurfaceKind::FunctionQuote(inner) => self.lower_function_quote(form, inner),
            SurfaceKind::List(items) => self.lower_list(form, items),
            SurfaceKind::Vector(_) => Some(HirExpr {
                kind: HirExprKind::Quote(Box::new(form.clone())),
                span: form.span,
            }),
            SurfaceKind::DottedList(items, tail) => self.lower_dotted_list(form, items, tail),
            SurfaceKind::Backquote(inner) => self.lower_quasiquote(form, inner),
            SurfaceKind::Comma(inner) => {
                // Unexpanded comma — evaluate the inner form
                self.lower_expr(inner)
            }
            SurfaceKind::CommaAt(inner) => {
                // Unexpanded splice — evaluate the inner form
                self.lower_expr(inner)
            }
        }
    }

    fn lower_atom(&mut self, atom: &SurfaceAtom, span: Span) -> HirExpr {
        match atom {
            SurfaceAtom::Nil => HirExpr {
                kind: HirExprKind::Const(HirConst::Nil),
                span,
            },
            SurfaceAtom::True => HirExpr {
                kind: HirExprKind::Const(HirConst::True),
                span,
            },
            SurfaceAtom::Int(value) => HirExpr {
                kind: HirExprKind::Const(HirConst::Int(*value)),
                span,
            },
            SurfaceAtom::Float(value) => HirExpr {
                kind: HirExprKind::Const(HirConst::Float(*value)),
                span,
            },
            SurfaceAtom::String(value) => HirExpr {
                kind: HirExprKind::Const(HirConst::String(value.clone())),
                span,
            },
            SurfaceAtom::Char(value) => HirExpr {
                kind: HirExprKind::Const(HirConst::Char(*value)),
                span,
            },
            SurfaceAtom::Symbol(name) if name.starts_with(':') => quote_symbol_expr(name, span),
            SurfaceAtom::Symbol(name) => {
                if self.is_lexical(name) {
                    HirExpr {
                        kind: HirExprKind::LexicalGet(name.clone()),
                        span,
                    }
                } else {
                    HirExpr {
                        kind: HirExprKind::SymbolGet(name.clone()),
                        span,
                    }
                }
            }
        }
    }

    fn lower_list(&mut self, form: &SurfaceForm, items: &[SurfaceForm]) -> Option<HirExpr> {
        let Some(head) = items.first() else {
            return Some(HirExpr {
                kind: HirExprKind::Const(HirConst::Nil),
                span: form.span,
            });
        };
        match head.symbol_name() {
            Some("quote") => self.lower_quote_form(form, items),
            Some("function") => self.lower_function_form(form, items),
            Some("if") => self.lower_if(form, items),
            Some("while") => self.lower_while(form, &items[1..]),
            Some("when") => self.lower_when(form, &items[1..]),
            Some("unless") => self.lower_unless(form, &items[1..]),
            Some("cond") => self.lower_cond(form, &items[1..]),
            Some("progn") => self.lower_progn(form, &items[1..]),
            Some("eval-and-compile" | "eval-when-compile" | "with-no-warnings") => {
                self.lower_progn(form, &items[1..])
            }
            Some("prog1") => self.lower_prog1(form, &items[1..]),
            Some("and") => self.lower_and(form, &items[1..]),
            Some("or") => self.lower_or(form, &items[1..]),
            Some("let") => self.lower_let(form, &items[1..], false),
            Some("let*") => self.lower_let(form, &items[1..], true),
            Some("dolist") => self.lower_dolist(form, &items[1..]),
            Some("dotimes") => self.lower_dotimes(form, &items[1..]),
            Some("lambda") => self.lower_lambda(form, items),
            Some("declare") => self.lower_declare(form, &items[1..]),
            Some("defvar") => self.lower_defvar(form, &items[1..]),
            Some("defconst") => self.lower_defconst(form, &items[1..]),
            Some("defcustom") => self.lower_defcustom(form, &items[1..]),
            Some("defgroup") => self.lower_defgroup(form, &items[1..]),
            Some("declare-function") => self.lower_declare_function(form, &items[1..]),
            Some("catch") => self.lower_catch(form, &items[1..]),
            Some("throw") => self.lower_throw(form, &items[1..]),
            Some("condition-case") => self.lower_condition_case(form, &items[1..]),
            Some("condition-case-unless-debug") => self.lower_condition_case(form, &items[1..]),
            Some("ignore-errors") => self.lower_ignore_errors(form, &items[1..]),
            Some("unwind-protect") => self.lower_unwind_protect(form, &items[1..]),
            Some("funcall") => self.lower_funcall(form, &items[1..]),
            Some("apply") => self.lower_apply(form, &items[1..]),
            Some("setq") => self.lower_setq(form, &items[1..]),
            Some(name) if self.is_lexical(name) => {
                let callee = HirExpr {
                    kind: HirExprKind::LexicalGet(name.to_string()),
                    span: head.span,
                };
                let args = self.lower_exprs(&items[1..])?;
                Some(HirExpr {
                    kind: HirExprKind::Funcall {
                        callee: Box::new(callee),
                        args,
                    },
                    span: form.span,
                })
            }
            Some(name) => self.lower_call_named(form, name, &items[1..]),
            None => self.lower_call_value(form, head, &items[1..]),
        }
    }

    /// Lower a dotted list (a b . c) as a chain of cons: (cons a (cons b c))
    fn lower_dotted_list(
        &mut self,
        form: &SurfaceForm,
        items: &[SurfaceForm],
        tail: &SurfaceForm,
    ) -> Option<HirExpr> {
        let tail_expr = self.lower_expr(tail)?;
        let span = form.span;
        let mut result = tail_expr;
        for item in items.iter().rev() {
            let car_expr = self.lower_expr(item)?;
            result = HirExpr {
                kind: HirExprKind::CallNamed {
                    name: "cons".to_string(),
                    args: vec![car_expr, result],
                },
                span,
            };
        }
        Some(result)
    }

    fn lower_quote_form(&mut self, form: &SurfaceForm, items: &[SurfaceForm]) -> Option<HirExpr> {
        if items.len() != 2 {
            self.error(form.span, "quote requires exactly one argument");
            return None;
        }
        Some(HirExpr {
            kind: HirExprKind::Quote(Box::new(items[1].clone())),
            span: form.span,
        })
    }

    fn lower_function_form(
        &mut self,
        form: &SurfaceForm,
        items: &[SurfaceForm],
    ) -> Option<HirExpr> {
        if items.len() < 2 {
            return nil_expr(form.span).into();
        }
        if items.len() > 2 {
            // Extra args — just ignore them and use the first
        }
        self.lower_function_quote(form, &items[1])
    }

    fn lower_function_quote(
        &mut self,
        form: &SurfaceForm,
        quoted: &SurfaceForm,
    ) -> Option<HirExpr> {
        if let Some(items) = list_items(quoted)
            && items.first().and_then(SurfaceForm::symbol_name) == Some("lambda")
        {
            return self.lower_lambda(form, items);
        }
        Some(HirExpr {
            kind: HirExprKind::FunctionQuote(Box::new(quoted.clone())),
            span: form.span,
        })
    }

    fn lower_quasiquote(&mut self, form: &SurfaceForm, inner: &SurfaceForm) -> Option<HirExpr> {
        let mut expr = self.lower_quasiquote_form(inner, 1)?;
        expr.span = form.span;
        Some(expr)
    }

    fn lower_quasiquote_form(&mut self, form: &SurfaceForm, depth: usize) -> Option<HirExpr> {
        match &form.kind {
            SurfaceKind::Comma(inner) if depth == 1 => self.lower_expr(inner),
            SurfaceKind::Comma(inner) => {
                self.lower_quasiquote_prefixed("unquote", inner, depth - 1, form.span)
            }
            SurfaceKind::CommaAt(_) if depth == 1 => {
                self.error(
                    form.span,
                    "unquote-splicing is only valid inside a backquote list or vector",
                );
                None
            }
            SurfaceKind::CommaAt(inner) => {
                self.lower_quasiquote_prefixed("unquote-splicing", inner, depth - 1, form.span)
            }
            SurfaceKind::Backquote(inner) => {
                self.lower_quasiquote_prefixed("quasiquote", inner, depth + 1, form.span)
            }
            SurfaceKind::Quote(inner) => {
                self.lower_quasiquote_prefixed("quote", inner, depth, form.span)
            }
            SurfaceKind::FunctionQuote(inner) => {
                self.lower_quasiquote_prefixed("function", inner, depth, form.span)
            }
            SurfaceKind::List(items) => self.lower_quasiquote_list(form, items, depth),
            SurfaceKind::DottedList(items, tail) => {
                self.lower_quasiquote_dotted_list(form, items, tail, depth)
            }
            SurfaceKind::Vector(items) => self.lower_quasiquote_vector(form, items, depth),
            SurfaceKind::Atom(_) => Some(quote_form_expr(form.clone(), form.span)),
        }
    }

    fn lower_quasiquote_prefixed(
        &mut self,
        name: &str,
        inner: &SurfaceForm,
        depth: usize,
        span: Span,
    ) -> Option<HirExpr> {
        if let SurfaceKind::CommaAt(splice) = &inner.kind
            && depth == 1
        {
            return Some(append_expr(
                vec![
                    list_expr(vec![quote_symbol_expr(name, span)], span),
                    self.lower_expr(splice)?,
                ],
                span,
            ));
        }
        Some(list_expr(
            vec![
                quote_symbol_expr(name, span),
                self.lower_quasiquote_form(inner, depth)?,
            ],
            span,
        ))
    }

    fn lower_quasiquote_list(
        &mut self,
        form: &SurfaceForm,
        items: &[SurfaceForm],
        depth: usize,
    ) -> Option<HirExpr> {
        let (parts, has_splice) = self.lower_quasiquote_list_parts(items, depth, form.span)?;
        if has_splice {
            Some(append_expr(parts, form.span))
        } else if parts.len() == 1 {
            // When no splicing, there is exactly one segment that is already
            // (list ...) — return it directly to avoid double-wrapping.
            Some(parts.into_iter().next().unwrap())
        } else {
            // Multiple segments without splicing should not happen, but handle
            // gracefully by appending them.
            Some(append_expr(parts, form.span))
        }
    }

    fn lower_quasiquote_dotted_list(
        &mut self,
        form: &SurfaceForm,
        items: &[SurfaceForm],
        tail: &SurfaceForm,
        depth: usize,
    ) -> Option<HirExpr> {
        let (mut parts, has_splice) = self.lower_quasiquote_list_parts(items, depth, form.span)?;
        let tail = self.lower_quasiquote_form(tail, depth)?;
        if has_splice {
            parts.push(tail);
            return Some(append_expr(parts, form.span));
        }
        let mut result = tail;
        for item in parts.into_iter().rev() {
            result = call_named_expr("cons", vec![item, result], form.span);
        }
        Some(result)
    }

    fn lower_quasiquote_vector(
        &mut self,
        form: &SurfaceForm,
        items: &[SurfaceForm],
        depth: usize,
    ) -> Option<HirExpr> {
        let (parts, has_splice) = self.lower_quasiquote_list_parts(items, depth, form.span)?;
        if !has_splice {
            return Some(call_named_expr("vector", parts, form.span));
        }
        Some(HirExpr {
            kind: HirExprKind::Apply {
                callee: Box::new(quote_symbol_expr("vector", form.span)),
                args: vec![append_expr(parts, form.span)],
            },
            span: form.span,
        })
    }

    fn lower_quasiquote_list_parts(
        &mut self,
        items: &[SurfaceForm],
        depth: usize,
        span: Span,
    ) -> Option<(Vec<HirExpr>, bool)> {
        let mut parts = Vec::new();
        let mut segment = Vec::new();
        let mut has_splice = false;
        for item in items {
            if let SurfaceKind::CommaAt(inner) = &item.kind
                && depth == 1
            {
                flush_quasiquote_segment(&mut parts, &mut segment, span);
                parts.push(self.lower_expr(inner)?);
                has_splice = true;
                continue;
            }
            segment.push(self.lower_quasiquote_form(item, depth)?);
        }
        flush_quasiquote_segment(&mut parts, &mut segment, span);
        Some((parts, has_splice))
    }

    fn lower_if(&mut self, form: &SurfaceForm, items: &[SurfaceForm]) -> Option<HirExpr> {
        if items.len() < 3 {
            self.error(form.span, "if requires test, then, and optional else");
            return None;
        }
        let test = self.lower_expr(&items[1])?;
        let then_expr = self.lower_expr(&items[2])?;
        let else_expr = if items.len() > 3 {
            self.lower_body(&items[3..], form.span)?
        } else {
            nil_expr(form.span)
        };
        Some(HirExpr {
            kind: HirExprKind::If {
                test: Box::new(test),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            },
            span: form.span,
        })
    }

    fn lower_while(&mut self, form: &SurfaceForm, tail: &[SurfaceForm]) -> Option<HirExpr> {
        let Some((test, body)) = tail.split_first() else {
            self.error(form.span, "while requires a test expression");
            return None;
        };
        Some(HirExpr {
            kind: HirExprKind::While {
                test: Box::new(self.lower_expr(test)?),
                body: Box::new(self.lower_body(body, form.span)?),
            },
            span: form.span,
        })
    }

    fn lower_when(&mut self, form: &SurfaceForm, tail: &[SurfaceForm]) -> Option<HirExpr> {
        let Some((test, body)) = tail.split_first() else {
            self.error(form.span, "when requires a test expression");
            return None;
        };
        Some(HirExpr {
            kind: HirExprKind::If {
                test: Box::new(self.lower_expr(test)?),
                then_expr: Box::new(self.lower_body(body, form.span)?),
                else_expr: Box::new(nil_expr(form.span)),
            },
            span: form.span,
        })
    }

    fn lower_unless(&mut self, form: &SurfaceForm, tail: &[SurfaceForm]) -> Option<HirExpr> {
        let Some((test, body)) = tail.split_first() else {
            self.error(form.span, "unless requires a test expression");
            return None;
        };
        Some(HirExpr {
            kind: HirExprKind::If {
                test: Box::new(self.lower_expr(test)?),
                then_expr: Box::new(nil_expr(form.span)),
                else_expr: Box::new(self.lower_body(body, form.span)?),
            },
            span: form.span,
        })
    }

    fn lower_cond(&mut self, form: &SurfaceForm, clauses: &[SurfaceForm]) -> Option<HirExpr> {
        let Some((clause, rest)) = clauses.split_first() else {
            return Some(nil_expr(form.span));
        };
        let Some(items) = list_items(clause) else {
            // Non-list clause (e.g., a symbol) — treat as a test with no body
            let test = self.lower_expr(clause)?;
            let else_expr = self.lower_cond(form, rest)?;
            return self.lower_cond_test_value_clause(form, test, else_expr, rest.len());
        };
        let Some((test, body)) = items.split_first() else {
            self.error(clause.span, "cond clause cannot be empty");
            return None;
        };
        let test = self.lower_expr(test)?;
        let else_expr = self.lower_cond(form, rest)?;
        if body.is_empty() {
            return self.lower_cond_test_value_clause(form, test, else_expr, rest.len());
        }
        Some(HirExpr {
            kind: HirExprKind::If {
                test: Box::new(test),
                then_expr: Box::new(self.lower_body(body, clause.span)?),
                else_expr: Box::new(else_expr),
            },
            span: form.span,
        })
    }

    fn lower_cond_test_value_clause(
        &mut self,
        form: &SurfaceForm,
        test: HirExpr,
        else_expr: HirExpr,
        remaining_clauses: usize,
    ) -> Option<HirExpr> {
        let temp = format!("\0cond.{}.{}", form.span.start, remaining_clauses);
        Some(HirExpr {
            kind: HirExprKind::Let {
                mode: BindingMode::Lexical,
                sequential: true,
                declarations: Vec::new(),
                bindings: vec![HirBinding {
                    name: temp.clone(),
                    mode: BindingMode::Lexical,
                    init: test,
                    span: form.span,
                }],
                body: Box::new(HirExpr {
                    kind: HirExprKind::If {
                        test: Box::new(HirExpr {
                            kind: HirExprKind::LexicalGet(temp.clone()),
                            span: form.span,
                        }),
                        then_expr: Box::new(HirExpr {
                            kind: HirExprKind::LexicalGet(temp),
                            span: form.span,
                        }),
                        else_expr: Box::new(else_expr),
                    },
                    span: form.span,
                }),
            },
            span: form.span,
        })
    }

    fn lower_progn(&mut self, form: &SurfaceForm, body: &[SurfaceForm]) -> Option<HirExpr> {
        Some(HirExpr {
            kind: HirExprKind::Progn(self.lower_exprs(body)?),
            span: form.span,
        })
    }

    fn lower_prog1(&mut self, form: &SurfaceForm, body: &[SurfaceForm]) -> Option<HirExpr> {
        let Some((first, rest)) = body.split_first() else {
            self.error(form.span, "prog1 requires at least one argument");
            return None;
        };
        let first = self.lower_expr(first)?;
        if rest.is_empty() {
            return Some(first);
        }
        let temp = format!("\0prog1.{}", form.span.start);
        let mut exprs = self.lower_exprs(rest)?;
        exprs.push(HirExpr {
            kind: HirExprKind::LexicalGet(temp.clone()),
            span: form.span,
        });
        Some(HirExpr {
            kind: HirExprKind::Let {
                mode: BindingMode::Lexical,
                sequential: true,
                declarations: Vec::new(),
                bindings: vec![HirBinding {
                    name: temp,
                    mode: BindingMode::Lexical,
                    init: first,
                    span: form.span,
                }],
                body: Box::new(HirExpr {
                    kind: HirExprKind::Progn(exprs),
                    span: form.span,
                }),
            },
            span: form.span,
        })
    }

    fn lower_and(&mut self, form: &SurfaceForm, exprs: &[SurfaceForm]) -> Option<HirExpr> {
        let Some((first, rest)) = exprs.split_first() else {
            return Some(HirExpr {
                kind: HirExprKind::Const(HirConst::True),
                span: form.span,
            });
        };
        let first = self.lower_expr(first)?;
        if rest.is_empty() {
            return Some(first);
        }
        Some(HirExpr {
            kind: HirExprKind::If {
                test: Box::new(first),
                then_expr: Box::new(self.lower_and(form, rest)?),
                else_expr: Box::new(HirExpr {
                    kind: HirExprKind::Const(HirConst::Nil),
                    span: form.span,
                }),
            },
            span: form.span,
        })
    }

    fn lower_or(&mut self, form: &SurfaceForm, exprs: &[SurfaceForm]) -> Option<HirExpr> {
        let Some((first, rest)) = exprs.split_first() else {
            return Some(HirExpr {
                kind: HirExprKind::Const(HirConst::Nil),
                span: form.span,
            });
        };
        let first = self.lower_expr(first)?;
        if rest.is_empty() {
            return Some(first);
        }
        let temp = format!("\0or.{}.{}", form.span.start, rest.len());
        Some(HirExpr {
            kind: HirExprKind::Let {
                mode: BindingMode::Lexical,
                sequential: true,
                declarations: Vec::new(),
                bindings: vec![HirBinding {
                    name: temp.clone(),
                    mode: BindingMode::Lexical,
                    init: first,
                    span: form.span,
                }],
                body: Box::new(HirExpr {
                    kind: HirExprKind::If {
                        test: Box::new(HirExpr {
                            kind: HirExprKind::LexicalGet(temp.clone()),
                            span: form.span,
                        }),
                        then_expr: Box::new(HirExpr {
                            kind: HirExprKind::LexicalGet(temp),
                            span: form.span,
                        }),
                        else_expr: Box::new(self.lower_or(form, rest)?),
                    },
                    span: form.span,
                }),
            },
            span: form.span,
        })
    }

    fn lower_let(
        &mut self,
        form: &SurfaceForm,
        tail: &[SurfaceForm],
        sequential: bool,
    ) -> Option<HirExpr> {
        if tail.len() < 2 {
            self.error(form.span, "let requires bindings and body");
            return None;
        }
        let (declarations, body_forms) = self.split_leading_declarations(&tail[1..]);
        self.push_special_scope(special_declared_names(&declarations));
        let mode = if self.lexical_binding {
            BindingMode::Lexical
        } else {
            BindingMode::Dynamic
        };
        let binding_forms = match list_items(&tail[0]) {
            Some(forms) => forms,
            None => {
                // Nil or non-list bindings — treat as empty
                &[]
            }
        };
        let mut bindings = Vec::new();
        if !sequential {
            for binding_form in binding_forms {
                if let Some(binding) = self.lower_binding(binding_form) {
                    bindings.push(binding);
                }
            }
            if mode == BindingMode::Lexical {
                self.push_scope(
                    bindings
                        .iter()
                        .filter(|binding| binding.mode == BindingMode::Lexical)
                        .map(|binding| binding.name.clone()),
                );
            }
        } else {
            if mode == BindingMode::Lexical {
                self.push_scope(std::iter::empty());
            }
            for binding_form in binding_forms {
                if let Some(binding) = self.lower_binding(binding_form) {
                    if binding.mode == BindingMode::Lexical {
                        self.declare_local(binding.name.clone());
                    }
                    bindings.push(binding);
                }
            }
        }
        let body = self.lower_body(body_forms, form.span)?;
        if mode == BindingMode::Lexical {
            self.pop_scope();
        }
        self.pop_special_scope();
        Some(HirExpr {
            kind: HirExprKind::Let {
                mode,
                sequential,
                declarations,
                bindings,
                body: Box::new(body),
            },
            span: form.span,
        })
    }

    fn lower_dolist(&mut self, form: &SurfaceForm, tail: &[SurfaceForm]) -> Option<HirExpr> {
        if tail.len() < 2 {
            self.error(form.span, "dolist requires a binding spec and body");
            return None;
        }
        let Some(spec) = list_items(&tail[0]) else {
            self.error(tail[0].span, "dolist binding spec must be a list");
            return None;
        };
        if !(2..=3).contains(&spec.len()) {
            self.error(
                tail[0].span,
                "dolist binding spec must be (var list [result])",
            );
            return None;
        }
        let Some(var) = spec[0].symbol_name().map(str::to_string) else {
            self.error(spec[0].span, "dolist variable must be a symbol");
            return None;
        };
        let list_init = self.lower_expr(&spec[1])?;
        let list_temp = format!("\0dolist.list.{}", form.span.start);
        let var_mode = self.binding_mode_for(&var);
        self.push_scope(
            std::iter::once(list_temp.clone())
                .chain((var_mode == BindingMode::Lexical).then_some(var.clone())),
        );
        let result = if let Some(result) = spec.get(2) {
            self.lower_expr(result)?
        } else {
            nil_expr(form.span)
        };
        let body = self.lower_exprs(&tail[1..])?;
        self.pop_scope();

        let var_set = assign_expr(
            var.clone(),
            var_mode,
            call_named_expr(
                "car",
                vec![lexical_get_expr(&list_temp, form.span)],
                form.span,
            ),
            form.span,
        );
        let list_advance = assign_expr(
            list_temp.clone(),
            BindingMode::Lexical,
            call_named_expr(
                "cdr",
                vec![lexical_get_expr(&list_temp, form.span)],
                form.span,
            ),
            form.span,
        );
        let mut loop_exprs = Vec::with_capacity(body.len() + 2);
        loop_exprs.push(var_set);
        loop_exprs.extend(body);
        loop_exprs.push(list_advance);
        Some(HirExpr {
            kind: HirExprKind::Let {
                mode: BindingMode::Lexical,
                sequential: false,
                declarations: Vec::new(),
                bindings: vec![
                    HirBinding {
                        name: list_temp.clone(),
                        mode: BindingMode::Lexical,
                        init: list_init,
                        span: form.span,
                    },
                    HirBinding {
                        name: var,
                        mode: var_mode,
                        init: nil_expr(form.span),
                        span: form.span,
                    },
                ],
                body: Box::new(HirExpr {
                    kind: HirExprKind::Progn(vec![
                        HirExpr {
                            kind: HirExprKind::While {
                                test: Box::new(lexical_get_expr(&list_temp, form.span)),
                                body: Box::new(HirExpr {
                                    kind: HirExprKind::Progn(loop_exprs),
                                    span: form.span,
                                }),
                            },
                            span: form.span,
                        },
                        // After the loop, set var to nil per Emacs spec.
                        assign_expr(
                            spec[0].symbol_name().unwrap_or("").to_string(),
                            var_mode,
                            nil_expr(form.span),
                            form.span,
                        ),
                        result,
                    ]),
                    span: form.span,
                }),
            },
            span: form.span,
        })
    }

    fn lower_dotimes(&mut self, form: &SurfaceForm, tail: &[SurfaceForm]) -> Option<HirExpr> {
        if tail.len() < 2 {
            self.error(form.span, "dotimes requires a binding spec and body");
            return None;
        }
        let Some(spec) = list_items(&tail[0]) else {
            self.error(tail[0].span, "dotimes binding spec must be a list");
            return None;
        };
        if !(2..=3).contains(&spec.len()) {
            self.error(
                tail[0].span,
                "dotimes binding spec must be (var count [result])",
            );
            return None;
        }
        let Some(var) = spec[0].symbol_name().map(str::to_string) else {
            self.error(spec[0].span, "dotimes variable must be a symbol");
            return None;
        };
        let limit_init = self.lower_expr(&spec[1])?;
        let limit_temp = format!("\0dotimes.limit.{}", form.span.start);
        let var_mode = self.binding_mode_for(&var);
        self.push_scope(
            std::iter::once(limit_temp.clone())
                .chain((var_mode == BindingMode::Lexical).then_some(var.clone())),
        );
        let result = if let Some(result) = spec.get(2) {
            self.lower_expr(result)?
        } else {
            nil_expr(form.span)
        };
        let body = self.lower_exprs(&tail[1..])?;
        self.pop_scope();

        let var_get = name_get_expr(&var, var_mode, form.span);
        let var_advance = assign_expr(
            var.clone(),
            var_mode,
            call_named_expr("1+", vec![var_get.clone()], form.span),
            form.span,
        );
        let mut loop_exprs = Vec::with_capacity(body.len() + 1);
        loop_exprs.extend(body);
        loop_exprs.push(var_advance);
        Some(HirExpr {
            kind: HirExprKind::Let {
                mode: BindingMode::Lexical,
                sequential: false,
                declarations: Vec::new(),
                bindings: vec![
                    HirBinding {
                        name: limit_temp.clone(),
                        mode: BindingMode::Lexical,
                        init: limit_init,
                        span: form.span,
                    },
                    HirBinding {
                        name: var.clone(),
                        mode: var_mode,
                        init: HirExpr {
                            kind: HirExprKind::Const(HirConst::Int(0)),
                            span: form.span,
                        },
                        span: form.span,
                    },
                ],
                body: Box::new(HirExpr {
                    kind: HirExprKind::Progn(vec![
                        HirExpr {
                            kind: HirExprKind::While {
                                test: Box::new(call_named_expr(
                                    "<",
                                    vec![
                                        name_get_expr(&var, var_mode, form.span),
                                        lexical_get_expr(&limit_temp, form.span),
                                    ],
                                    form.span,
                                )),
                                body: Box::new(HirExpr {
                                    kind: HirExprKind::Progn(loop_exprs),
                                    span: form.span,
                                }),
                            },
                            span: form.span,
                        },
                        result,
                    ]),
                    span: form.span,
                }),
            },
            span: form.span,
        })
    }

    fn lower_binding(&mut self, form: &SurfaceForm) -> Option<HirBinding> {
        if let Some(name) = form.symbol_name() {
            let mode = self.binding_mode_for(&name);
            return Some(HirBinding {
                name: name.to_string(),
                mode,
                init: nil_expr(form.span),
                span: form.span,
            });
        }
        let Some(items) = list_items(form) else {
            // Non-list binding (could be nil atom) — skip it
            return None;
        };
        if items.is_empty() {
            return None;
        }
        if items.len() > 2 {
            // Extra items (e.g., docstring) — just use first two
        }
        let Some(name) = items[0].symbol_name().map(str::to_string) else {
            // Destructuring binding (e.g., ((a b) expr)) — not yet supported
            self.error(
                items[0].span,
                "destructuring bindings in let are not yet supported",
            );
            return None;
        };
        let init = if let Some(init_form) = items.get(1) {
            self.lower_expr(init_form)?
        } else {
            nil_expr(form.span)
        };
        let mode = self.binding_mode_for(&name);
        Some(HirBinding {
            name,
            mode,
            init,
            span: form.span,
        })
    }

    fn lower_lambda(&mut self, form: &SurfaceForm, items: &[SurfaceForm]) -> Option<HirExpr> {
        if items.len() < 2 {
            self.error(form.span, "lambda requires at least an arg list");
            return None;
        }
        let params = self.parse_param_list(&items[1])?;
        let (declarations, body_forms) = self.split_leading_declarations(&items[2..]);
        self.push_special_scope(special_declared_names(&declarations));
        let lexical_params = params
            .names()
            .filter(|name| !self.is_special(name))
            .cloned()
            .collect::<Vec<_>>();
        self.push_scope(lexical_params);
        let body = self.lower_body(body_forms, form.span)?;
        self.pop_scope();
        self.pop_special_scope();
        Some(HirExpr {
            kind: HirExprKind::Lambda {
                params,
                declarations,
                body: Box::new(body),
            },
            span: form.span,
        })
    }

    fn lower_defvar(&mut self, form: &SurfaceForm, tail: &[SurfaceForm]) -> Option<HirExpr> {
        if tail.is_empty() || tail.len() > 3 {
            self.error(
                form.span,
                "defvar requires a variable name, optional init, and optional docstring",
            );
            return None;
        }
        let Some(name) = tail[0].symbol_name().map(str::to_string) else {
            // Non-symbol name (e.g., backquote remnant) — evaluate as progn
            return self.lower_progn(form, tail);
        };
        // Register the variable as dynamically scoped for the rest of the file.
        self.declared_special.insert(name.clone());
        let quoted_name = quote_symbol_expr(&name, tail[0].span);
        let Some(init_form) = tail.get(1) else {
            return Some(quoted_name);
        };
        let boundp = HirExpr {
            kind: HirExprKind::CallNamed {
                name: "boundp".to_string(),
                args: vec![quoted_name.clone()],
            },
            span: form.span,
        };
        let init = self.lower_expr(init_form)?;
        let set_then_return = HirExpr {
            kind: HirExprKind::Progn(vec![
                HirExpr {
                    kind: HirExprKind::SymbolSet {
                        name,
                        value: Box::new(init),
                    },
                    span: form.span,
                },
                quoted_name.clone(),
            ]),
            span: form.span,
        };
        Some(HirExpr {
            kind: HirExprKind::If {
                test: Box::new(boundp),
                then_expr: Box::new(quoted_name),
                else_expr: Box::new(set_then_return),
            },
            span: form.span,
        })
    }

    fn lower_defconst(&mut self, form: &SurfaceForm, tail: &[SurfaceForm]) -> Option<HirExpr> {
        if tail.len() < 2 || tail.len() > 3 {
            self.error(
                form.span,
                "defconst requires a variable name, init, and optional docstring",
            );
            return None;
        }
        let Some(name) = tail[0].symbol_name().map(str::to_string) else {
            self.error(tail[0].span, "defconst variable name must be a symbol");
            return None;
        };
        // Register the variable as dynamically scoped for the rest of the file.
        self.declared_special.insert(name.clone());
        let quoted_name = quote_symbol_expr(&name, tail[0].span);
        let init = self.lower_expr(&tail[1])?;
        Some(HirExpr {
            kind: HirExprKind::Progn(vec![
                HirExpr {
                    kind: HirExprKind::SymbolSet {
                        name,
                        value: Box::new(init),
                    },
                    span: form.span,
                },
                quoted_name,
            ]),
            span: form.span,
        })
    }

    fn lower_defcustom(&mut self, form: &SurfaceForm, tail: &[SurfaceForm]) -> Option<HirExpr> {
        if tail.len() < 3 {
            self.error(
                form.span,
                "defcustom requires a variable name, init, docstring, and keyword arguments",
            );
            return None;
        }
        let Some(name) = tail[0].symbol_name().map(str::to_string) else {
            self.error(tail[0].span, "defcustom variable name must be a symbol");
            return None;
        };
        self.declared_special.insert(name.clone());
        let quoted_name = quote_symbol_expr(&name, tail[0].span);
        let boundp = HirExpr {
            kind: HirExprKind::CallNamed {
                name: "boundp".to_string(),
                args: vec![quoted_name.clone()],
            },
            span: form.span,
        };
        let init = self.lower_expr(&tail[1])?;
        let set_then_return = HirExpr {
            kind: HirExprKind::Progn(vec![
                HirExpr {
                    kind: HirExprKind::SymbolSet {
                        name,
                        value: Box::new(init),
                    },
                    span: form.span,
                },
                quoted_name.clone(),
            ]),
            span: form.span,
        };
        Some(HirExpr {
            kind: HirExprKind::If {
                test: Box::new(boundp),
                then_expr: Box::new(quoted_name),
                else_expr: Box::new(set_then_return),
            },
            span: form.span,
        })
    }

    fn lower_defgroup(&mut self, form: &SurfaceForm, tail: &[SurfaceForm]) -> Option<HirExpr> {
        let Some(name_form) = tail.first() else {
            self.error(form.span, "defgroup requires a group name");
            return None;
        };
        let Some(name) = name_form.symbol_name() else {
            self.error(name_form.span, "defgroup name must be a symbol");
            return None;
        };
        Some(quote_symbol_expr(name, name_form.span))
    }

    fn lower_declare_function(
        &mut self,
        form: &SurfaceForm,
        tail: &[SurfaceForm],
    ) -> Option<HirExpr> {
        if tail.len() < 2 {
            self.error(
                form.span,
                "declare-function requires a function name and file name",
            );
            return None;
        }
        if tail[0].symbol_name().is_none() {
            self.error(tail[0].span, "declare-function name must be a symbol");
            return None;
        }
        Some(nil_expr(form.span))
    }

    fn lower_declare(&mut self, form: &SurfaceForm, specs: &[SurfaceForm]) -> Option<HirExpr> {
        Some(HirExpr {
            kind: HirExprKind::Declare(self.parse_declarations(specs)),
            span: form.span,
        })
    }

    fn lower_catch(&mut self, form: &SurfaceForm, tail: &[SurfaceForm]) -> Option<HirExpr> {
        if tail.len() < 2 {
            self.error(form.span, "catch requires a tag and body");
            return None;
        }
        let tag = self.lower_expr(&tail[0])?;
        let body = self.lower_body(&tail[1..], form.span)?;
        Some(HirExpr {
            kind: HirExprKind::Catch {
                tag: Box::new(tag),
                body: Box::new(body),
            },
            span: form.span,
        })
    }

    fn lower_throw(&mut self, form: &SurfaceForm, tail: &[SurfaceForm]) -> Option<HirExpr> {
        if tail.len() != 2 {
            self.error(form.span, "throw requires a tag and value");
            return None;
        }
        Some(HirExpr {
            kind: HirExprKind::Throw {
                tag: Box::new(self.lower_expr(&tail[0])?),
                value: Box::new(self.lower_expr(&tail[1])?),
            },
            span: form.span,
        })
    }

    fn lower_condition_case(
        &mut self,
        form: &SurfaceForm,
        tail: &[SurfaceForm],
    ) -> Option<HirExpr> {
        if tail.len() < 2 {
            self.error(
                form.span,
                "condition-case requires a variable, body, and handlers",
            );
            return None;
        }
        let var = if matches!(&tail[0].kind, SurfaceKind::Atom(SurfaceAtom::Nil))
            || matches!(&tail[0].kind, SurfaceKind::List(items) if items.is_empty())
        {
            None
        } else if let Some(name) = tail[0].symbol_name() {
            Some(name.to_string())
        } else {
            // Accept non-symbol forms (e.g., destructuring patterns) — treat as unnamed
            None
        };
        let body = self.lower_expr(&tail[1])?;
        let mut handlers = Vec::new();
        for handler_form in &tail[2..] {
            let Some(items) = list_items(handler_form) else {
                self.error(handler_form.span, "condition-case handler must be a list");
                return None;
            };
            if items.is_empty() {
                self.error(handler_form.span, "condition-case handler cannot be empty");
                return None;
            }
            if let Some(ref var_name) = var {
                self.push_scope(std::iter::once(var_name.clone()));
            }
            let body = self.lower_body(&items[1..], handler_form.span)?;
            if var.is_some() {
                self.pop_scope();
            }
            handlers.push(HirConditionHandler {
                pattern: items[0].clone(),
                body,
                span: handler_form.span,
            });
        }
        Some(HirExpr {
            kind: HirExprKind::ConditionCase {
                var,
                body: Box::new(body),
                handlers,
            },
            span: form.span,
        })
    }

    fn lower_ignore_errors(&mut self, form: &SurfaceForm, body: &[SurfaceForm]) -> Option<HirExpr> {
        Some(HirExpr {
            kind: HirExprKind::ConditionCase {
                var: None,
                body: Box::new(self.lower_body(body, form.span)?),
                handlers: vec![HirConditionHandler {
                    pattern: SurfaceForm::new(
                        SurfaceKind::Atom(SurfaceAtom::symbol("error")),
                        form.span,
                    ),
                    body: nil_expr(form.span),
                    span: form.span,
                }],
            },
            span: form.span,
        })
    }

    fn lower_unwind_protect(
        &mut self,
        form: &SurfaceForm,
        tail: &[SurfaceForm],
    ) -> Option<HirExpr> {
        if tail.len() < 2 {
            self.error(form.span, "unwind-protect requires body and cleanup");
            return None;
        }
        Some(HirExpr {
            kind: HirExprKind::UnwindProtect {
                body: Box::new(self.lower_expr(&tail[0])?),
                cleanup: Box::new(self.lower_body(&tail[1..], form.span)?),
            },
            span: form.span,
        })
    }

    fn lower_funcall(&mut self, form: &SurfaceForm, tail: &[SurfaceForm]) -> Option<HirExpr> {
        if tail.is_empty() {
            self.error(form.span, "funcall requires a function");
            return None;
        }
        Some(HirExpr {
            kind: HirExprKind::Funcall {
                callee: Box::new(self.lower_expr(&tail[0])?),
                args: self.lower_exprs(&tail[1..])?,
            },
            span: form.span,
        })
    }

    fn lower_apply(&mut self, form: &SurfaceForm, tail: &[SurfaceForm]) -> Option<HirExpr> {
        if tail.is_empty() {
            return nil_expr(form.span).into();
        }
        if tail.len() < 2 {
            // Just a callee, no args — lower as a funcall with no args
            return Some(HirExpr {
                kind: HirExprKind::Apply {
                    callee: Box::new(self.lower_expr(&tail[0])?),
                    args: vec![],
                },
                span: form.span,
            });
        }
        Some(HirExpr {
            kind: HirExprKind::Apply {
                callee: Box::new(self.lower_expr(&tail[0])?),
                args: self.lower_exprs(&tail[1..])?,
            },
            span: form.span,
        })
    }

    fn lower_setq(&mut self, form: &SurfaceForm, pairs: &[SurfaceForm]) -> Option<HirExpr> {
        if pairs.is_empty() {
            return nil_expr(form.span).into();
        }
        let mut exprs = Vec::new();
        // Handle odd-length pairs: last single arg is just a symbol reference
        let mut i = 0;
        while i + 1 < pairs.len() {
            let name_form = &pairs[i];
            let value_form = &pairs[i + 1];
            let Some(name) = name_form.symbol_name().map(str::to_string) else {
                // Non-symbol target (e.g., backquote remnant) — eval value, skip assignment
                let _ = self.lower_expr(value_form)?;
                i += 2;
                continue;
            };
            let value = self.lower_expr(value_form)?;
            let kind = if self.is_lexical(&name) {
                HirExprKind::LexicalSet {
                    name,
                    value: Box::new(value),
                }
            } else {
                HirExprKind::SymbolSet {
                    name,
                    value: Box::new(value),
                }
            };
            exprs.push(HirExpr {
                kind,
                span: name_form.span,
            });
            i += 2;
        }
        // Handle trailing single symbol: (setq foo) just returns foo's value
        if i < pairs.len() {
            if let Some(name) = pairs[i].symbol_name().map(str::to_string) {
                let kind = if self.is_lexical(&name) {
                    HirExprKind::LexicalGet(name)
                } else {
                    HirExprKind::SymbolGet(name)
                };
                exprs.push(HirExpr {
                    kind,
                    span: pairs[i].span,
                });
            }
        }
        if exprs.is_empty() {
            return nil_expr(form.span).into();
        }
        if exprs.len() == 1 {
            exprs.pop()
        } else {
            Some(HirExpr {
                kind: HirExprKind::Progn(exprs),
                span: form.span,
            })
        }
    }

    fn lower_call_named(
        &mut self,
        form: &SurfaceForm,
        name: &str,
        args: &[SurfaceForm],
    ) -> Option<HirExpr> {
        Some(HirExpr {
            kind: HirExprKind::CallNamed {
                name: name.to_string(),
                args: self.lower_exprs(args)?,
            },
            span: form.span,
        })
    }

    fn lower_call_value(
        &mut self,
        form: &SurfaceForm,
        head: &SurfaceForm,
        args: &[SurfaceForm],
    ) -> Option<HirExpr> {
        let callee = self.lower_expr(head)?;
        Some(HirExpr {
            kind: HirExprKind::CallValue {
                callee: Box::new(callee),
                args: self.lower_exprs(args)?,
            },
            span: form.span,
        })
    }

    fn lower_body(&mut self, body: &[SurfaceForm], span: Span) -> Option<HirExpr> {
        if body.is_empty() {
            return Some(nil_expr(span));
        }
        if body.len() == 1 {
            return self.lower_expr(&body[0]);
        }
        Some(HirExpr {
            kind: HirExprKind::Progn(self.lower_exprs(body)?),
            span,
        })
    }

    fn lower_exprs(&mut self, forms: &[SurfaceForm]) -> Option<Vec<HirExpr>> {
        forms
            .iter()
            .map(|form| self.lower_expr(form))
            .collect::<Option<Vec<_>>>()
    }

    fn parse_param_list(&mut self, form: &SurfaceForm) -> Option<LambdaList> {
        // Handle nil atom as empty parameter list
        if matches!(&form.kind, SurfaceKind::Atom(SurfaceAtom::Nil)) {
            return Some(LambdaList::default());
        }
        let Some(items) = list_items(form) else {
            // Non-list parameter form (e.g., vector, atom after bad macro expansion)
            // Treat as empty parameter list
            return Some(LambdaList::default());
        };
        let mut params = LambdaList::default();
        let mut section = ParamSection::Required;
        for item in items {
            let Some(name) = item.symbol_name() else {
                // Destructuring parameter — not yet supported
                self.error(item.span, "destructuring parameters are not yet supported");
                continue;
            };
            match name {
                "&optional" => {
                    if section != ParamSection::Required {
                        self.error(item.span, "&optional is out of order");
                        return None;
                    }
                    section = ParamSection::Optional;
                    continue;
                }
                "&rest" => {
                    if section == ParamSection::Rest {
                        self.error(item.span, "duplicate &rest");
                        return None;
                    }
                    section = ParamSection::Rest;
                    continue;
                }
                _ if name.starts_with('&') => {
                    self.error(item.span, "lambda-list keyword is not supported yet");
                    return None;
                }
                _ => {}
            }
            match section {
                ParamSection::Required => params.required.push(name.to_string()),
                ParamSection::Optional => params.optional.push(name.to_string()),
                ParamSection::Rest => {
                    if params.rest.is_some() {
                        self.error(item.span, "&rest accepts only one parameter");
                        return None;
                    }
                    params.rest = Some(name.to_string());
                }
            }
        }
        if section == ParamSection::Rest && params.rest.is_none() {
            self.error(form.span, "&rest requires a parameter");
            return None;
        }
        Some(params)
    }

    fn push_scope(&mut self, names: impl IntoIterator<Item = String>) {
        let mut scope = IndexSet::new();
        scope.extend(names);
        self.scopes.push(scope);
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn push_special_scope(&mut self, names: impl IntoIterator<Item = String>) {
        let mut scope = IndexSet::new();
        scope.extend(names);
        self.special_scopes.push(scope);
    }

    fn pop_special_scope(&mut self) {
        self.special_scopes.pop();
    }

    fn declare_local(&mut self, name: String) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name);
        }
    }

    fn is_lexical(&self, name: &str) -> bool {
        if self.is_special(name) {
            return false;
        }
        self.scopes.iter().rev().any(|scope| scope.contains(name))
    }

    fn is_special(&self, name: &str) -> bool {
        self.special_scopes
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
            || self.declared_special.contains(name)
    }

    fn binding_mode_for(&self, name: &str) -> BindingMode {
        if self.lexical_binding && !self.is_special(name) {
            BindingMode::Lexical
        } else {
            BindingMode::Dynamic
        }
    }

    fn split_leading_declarations<'a>(
        &mut self,
        forms: &'a [SurfaceForm],
    ) -> (Vec<HirDeclaration>, &'a [SurfaceForm]) {
        let mut declarations = Vec::new();
        let mut body_start = 0;
        for form in forms {
            let Some(items) = list_items(form) else {
                break;
            };
            if items.first().and_then(SurfaceForm::symbol_name) != Some("declare") {
                break;
            }
            declarations.extend(self.parse_declarations(&items[1..]));
            body_start += 1;
        }
        (declarations, &forms[body_start..])
    }

    fn parse_declarations(&mut self, specs: &[SurfaceForm]) -> Vec<HirDeclaration> {
        let mut declarations = Vec::new();
        for spec in specs {
            let Some(items) = list_items(spec) else {
                self.error(spec.span, "declare spec must be a list");
                continue;
            };
            let Some(name) = items.first().and_then(SurfaceForm::symbol_name) else {
                self.error(spec.span, "declare spec name must be a symbol");
                continue;
            };
            if name == "special" {
                let mut vars = Vec::new();
                for item in &items[1..] {
                    if let Some(var) = item.symbol_name() {
                        vars.push(var.to_string());
                    } else {
                        self.error(item.span, "special declaration argument must be a symbol");
                    }
                }
                declarations.push(HirDeclaration::Special(vars));
            } else {
                declarations.push(HirDeclaration::Unknown {
                    name: name.to_string(),
                    args: items[1..]
                        .iter()
                        .filter_map(SurfaceForm::symbol_name)
                        .map(str::to_string)
                        .collect(),
                });
            }
        }
        declarations
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        let message = format!(
            "{}: {}",
            self.source.name.as_deref().unwrap_or("<source>"),
            message.into()
        );
        self.diagnostics
            .push(Diagnostic::error(message).with_span(span));
    }
}

fn special_declared_names(declarations: &[HirDeclaration]) -> impl Iterator<Item = String> + '_ {
    declarations.iter().flat_map(|decl| match decl {
        HirDeclaration::Special(names) => names.clone(),
        HirDeclaration::Unknown { .. } => Vec::new(),
    })
}

fn list_items(form: &SurfaceForm) -> Option<&[SurfaceForm]> {
    match &form.kind {
        SurfaceKind::List(items) => Some(items),
        _ => None,
    }
}

fn nil_expr(span: Span) -> HirExpr {
    HirExpr {
        kind: HirExprKind::Const(HirConst::Nil),
        span,
    }
}

fn quote_form_expr(form: SurfaceForm, span: Span) -> HirExpr {
    HirExpr {
        kind: HirExprKind::Quote(Box::new(form)),
        span,
    }
}

fn quote_symbol_expr(name: &str, span: Span) -> HirExpr {
    quote_form_expr(
        SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::symbol(name)), span),
        span,
    )
}

fn lexical_get_expr(name: &str, span: Span) -> HirExpr {
    HirExpr {
        kind: HirExprKind::LexicalGet(name.to_string()),
        span,
    }
}

fn name_get_expr(name: &str, mode: BindingMode, span: Span) -> HirExpr {
    HirExpr {
        kind: match mode {
            BindingMode::Lexical => HirExprKind::LexicalGet(name.to_string()),
            BindingMode::Dynamic => HirExprKind::SymbolGet(name.to_string()),
        },
        span,
    }
}

fn assign_expr(name: String, mode: BindingMode, value: HirExpr, span: Span) -> HirExpr {
    HirExpr {
        kind: match mode {
            BindingMode::Lexical => HirExprKind::LexicalSet {
                name,
                value: Box::new(value),
            },
            BindingMode::Dynamic => HirExprKind::SymbolSet {
                name,
                value: Box::new(value),
            },
        },
        span,
    }
}

fn call_named_expr(name: &str, args: Vec<HirExpr>, span: Span) -> HirExpr {
    HirExpr {
        kind: HirExprKind::CallNamed {
            name: name.to_string(),
            args,
        },
        span,
    }
}

fn list_expr(args: Vec<HirExpr>, span: Span) -> HirExpr {
    call_named_expr("list", args, span)
}

fn append_expr(parts: Vec<HirExpr>, span: Span) -> HirExpr {
    if parts.is_empty() {
        nil_expr(span)
    } else {
        call_named_expr("append", parts, span)
    }
}

fn flush_quasiquote_segment(parts: &mut Vec<HirExpr>, segment: &mut Vec<HirExpr>, span: Span) {
    if !segment.is_empty() {
        parts.push(list_expr(std::mem::take(segment), span));
    }
}

#[cfg(test)]
mod tests {
    use crate::{compile_source, hir::HirExprKind};

    use super::{HirItem, HirModule};

    fn hir(text: &str) -> HirModule {
        let artifact = compile_source("test.el", text);
        assert_eq!(artifact.diagnostics, Vec::new());
        artifact.hir.expect("HIR should be present")
    }

    #[test]
    fn defun_params_are_lexical_under_lexical_binding() {
        let module = hir(";;; -*- lexical-binding: t; -*-\n(defun add2 (x y) (+ x y))");
        let HirItem::Defun(defun) = &module.items[0] else {
            panic!("expected defun");
        };
        let HirExprKind::CallNamed { args, .. } = &defun.body.kind else {
            panic!("expected call");
        };
        assert!(matches!(args[0].kind, HirExprKind::LexicalGet(_)));
        assert!(matches!(args[1].kind, HirExprKind::LexicalGet(_)));
    }

    #[test]
    fn defsubst_lowers_like_defun() {
        let module = hir(";;; -*- lexical-binding: t; -*-\n(defsubst add1 (x) (1+ x))");
        let HirItem::Defun(defun) = &module.items[0] else {
            panic!("expected defun");
        };
        assert_eq!(defun.name, "add1");
    }

    #[test]
    fn parses_optional_and_rest_lambda_lists() {
        let module = hir(";;; -*- lexical-binding: t; -*-\n(defun f (x &optional y &rest zs) x)");
        let HirItem::Defun(defun) = &module.items[0] else {
            panic!("expected defun");
        };
        assert_eq!(defun.params.required, vec!["x".to_string()]);
        assert_eq!(defun.params.optional, vec!["y".to_string()]);
        assert_eq!(defun.params.rest, Some("zs".to_string()));
    }

    #[test]
    fn top_level_symbol_reads_are_symbol_gets() {
        let module = hir("x");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        assert!(matches!(expr.kind, HirExprKind::SymbolGet(_)));
    }

    #[test]
    fn keyword_symbols_are_self_evaluating() {
        let module = hir(":test");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        assert!(matches!(expr.kind, HirExprKind::Quote(_)));
    }

    #[test]
    fn lowers_backquote_with_unquote_and_splicing() {
        let module = hir(";;; -*- lexical-binding: t; -*-
(let ((x 2) (xs (list 3 4)))
  `(a ,x ,@xs b))");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        let HirExprKind::Let { body, .. } = &expr.kind else {
            panic!("expected let");
        };
        let HirExprKind::CallNamed { name, .. } = &body.kind else {
            panic!("expected backquote list to lower to append");
        };
        assert_eq!(name, "append");
    }

    #[test]
    fn lowers_backquote_vector_with_splicing() {
        let module = hir(";;; -*- lexical-binding: t; -*-
(let ((x 2) (xs (list 3 4)))
  `[a ,x ,@xs b])");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        let HirExprKind::Let { body, .. } = &expr.kind else {
            panic!("expected let");
        };
        assert!(matches!(body.kind, HirExprKind::Apply { .. }));
    }

    #[test]
    fn let_uses_dynamic_mode_without_lexical_binding() {
        let module = hir("(let ((x 1)) x)");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        let HirExprKind::Let { mode, .. } = &expr.kind else {
            panic!("expected let");
        };
        assert_eq!(*mode, super::BindingMode::Dynamic);
    }

    #[test]
    fn special_declaration_prevents_lexical_param() {
        let module = hir(";;; -*- lexical-binding: t; -*-\n(defun f (x) (declare (special x)) x)");
        let HirItem::Defun(defun) = &module.items[0] else {
            panic!("expected defun");
        };
        assert!(matches!(
            defun.declarations.as_slice(),
            [super::HirDeclaration::Special(names)] if names == &vec!["x".to_string()]
        ));
        assert!(matches!(defun.body.kind, HirExprKind::SymbolGet(_)));
    }

    #[test]
    fn lowers_nonlocal_and_dynamic_call_forms() {
        let module = hir(";;; -*- lexical-binding: t; -*-
(progn
  (catch 'tag (throw 'tag 1))
  (condition-case err (funcall f 1) (error err))
  (unwind-protect (apply f xs) (cleanup)))");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        let HirExprKind::Progn(exprs) = &expr.kind else {
            panic!("expected progn");
        };
        assert!(matches!(exprs[0].kind, HirExprKind::Catch { .. }));
        assert!(matches!(exprs[1].kind, HirExprKind::ConditionCase { .. }));
        assert!(matches!(exprs[2].kind, HirExprKind::UnwindProtect { .. }));
    }

    #[test]
    fn lowers_common_macro_like_wrapper_forms() {
        let module = hir(";;; -*- lexical-binding: t; -*-
(progn
  (eval-and-compile a)
  (eval-when-compile b)
  (with-no-warnings c)
  (condition-case-unless-debug err (error \"x\") (error err))
  (ignore-errors (error \"x\")))");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        let HirExprKind::Progn(exprs) = &expr.kind else {
            panic!("expected progn");
        };
        assert!(matches!(exprs[0].kind, HirExprKind::Progn(_)));
        assert!(matches!(exprs[1].kind, HirExprKind::Progn(_)));
        assert!(matches!(exprs[2].kind, HirExprKind::Progn(_)));
        assert!(matches!(exprs[3].kind, HirExprKind::ConditionCase { .. }));
        assert!(matches!(exprs[4].kind, HirExprKind::ConditionCase { .. }));
    }

    #[test]
    fn lowers_prog1_to_internal_lexical_temp() {
        let module = hir(";;; -*- lexical-binding: t; -*-\n(prog1 1 (message \"side\"))");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        assert!(matches!(expr.kind, HirExprKind::Let { .. }));
    }

    #[test]
    fn lowers_and_or_to_short_circuit_hir() {
        let module = hir(";;; -*- lexical-binding: t; -*-\n(progn (and a b) (or a b))");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        let HirExprKind::Progn(exprs) = &expr.kind else {
            panic!("expected progn");
        };
        assert!(matches!(exprs[0].kind, HirExprKind::If { .. }));
        assert!(matches!(exprs[1].kind, HirExprKind::Let { .. }));
    }

    #[test]
    fn lowers_while_to_hir_loop_form() {
        let module = hir(";;; -*- lexical-binding: t; -*-\n(while (< x 3) (setq x (1+ x)))");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        assert!(matches!(expr.kind, HirExprKind::While { .. }));
    }

    #[test]
    fn lowers_dolist_and_dotimes_to_hir_loops() {
        let module = hir(
            ";;; -*- lexical-binding: t; -*-\n(progn (dolist (x xs sum) (setq sum (+ sum x))) (dotimes (i n sum) (setq sum (+ sum i))))",
        );
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        let HirExprKind::Progn(exprs) = &expr.kind else {
            panic!("expected progn");
        };
        assert!(matches!(exprs[0].kind, HirExprKind::Let { .. }));
        assert!(matches!(exprs[1].kind, HirExprKind::Let { .. }));
    }

    #[test]
    fn lowers_when_unless_and_cond() {
        let module = hir(
            ";;; -*- lexical-binding: t; -*-\n(progn (when a b) (unless a b) (cond (a b) (c)))",
        );
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        let HirExprKind::Progn(exprs) = &expr.kind else {
            panic!("expected progn");
        };
        assert!(matches!(exprs[0].kind, HirExprKind::If { .. }));
        assert!(matches!(exprs[1].kind, HirExprKind::If { .. }));
        assert!(matches!(exprs[2].kind, HirExprKind::If { .. }));
    }

    #[test]
    fn lowers_if_with_multiple_else_forms() {
        let module = hir(";;; -*- lexical-binding: t; -*-\n(if x 1 2 3)");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        let HirExprKind::If { else_expr, .. } = &expr.kind else {
            panic!("expected if");
        };
        assert!(matches!(else_expr.kind, HirExprKind::Progn(_)));
    }

    #[test]
    fn lowers_defvar_and_defconst_special_forms() {
        let module =
            hir(";;; -*- lexical-binding: t; -*-\n(progn (defvar answer 42) (defconst fixed 7))");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        let HirExprKind::Progn(exprs) = &expr.kind else {
            panic!("expected progn");
        };
        assert!(matches!(exprs[0].kind, HirExprKind::If { .. }));
        assert!(matches!(exprs[1].kind, HirExprKind::Progn(_)));
    }

    #[test]
    fn lowers_vector_literals_as_quoted_constants() {
        let module = hir(";;; -*- lexical-binding: t; -*-\n[1 (+ 1 2)]");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        assert!(matches!(expr.kind, HirExprKind::Quote(_)));
    }

    #[test]
    fn lowers_declaration_and_custom_forms_without_evaluating_metadata() {
        let module = hir(";;; -*- lexical-binding: t; -*-
(progn
  (declare-function missing-fn \"file\")
  (defgroup object-group nil \"doc\" :group 'lisp)
  (defcustom object-custom (+ 1 2) \"doc\" :type 'integer))");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        let HirExprKind::Progn(exprs) = &expr.kind else {
            panic!("expected progn");
        };
        assert!(matches!(exprs[0].kind, HirExprKind::Const(_)));
        assert!(matches!(exprs[1].kind, HirExprKind::Quote(_)));
        assert!(matches!(exprs[2].kind, HirExprKind::If { .. }));
    }

    #[test]
    fn function_quoted_lambda_lowers_to_semantic_lambda() {
        let module =
            hir(";;; -*- lexical-binding: t; -*-\n(defun make (x) #'(lambda (y) (+ x y)))");
        let HirItem::Defun(defun) = &module.items[0] else {
            panic!("expected defun");
        };
        let HirExprKind::Lambda { params, body, .. } = &defun.body.kind else {
            panic!("expected lambda body");
        };
        assert_eq!(params.required, vec!["y".to_string()]);
        let HirExprKind::CallNamed { args, .. } = &body.kind else {
            panic!("expected lambda call body");
        };
        assert!(matches!(args[0].kind, HirExprKind::LexicalGet(_)));
        assert!(matches!(args[1].kind, HirExprKind::LexicalGet(_)));
    }

    #[test]
    fn defvar_registers_variable_as_special() {
        // defvar makes the variable special. In a subsequent let* form, references to
        // my-var should be SymbolGet (dynamic), not LexicalGet.
        let module =
            hir(";;; -*- lexical-binding: t; -*-\n(progn (defvar my-var 42) (let ((x my-var)) x))");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        let HirExprKind::Progn(exprs) = &expr.kind else {
            panic!("expected progn");
        };
        // exprs[1] is the let form. Its binding init should reference my-var as SymbolGet.
        let rendered = format!("{:?}", exprs[1].kind);
        assert!(
            rendered.contains("SymbolGet"),
            "my-var should be SymbolGet (dynamic) after defvar, got: {rendered}"
        );
    }

    #[test]
    fn dolist_sets_var_to_nil_after_loop() {
        let module =
            hir(";;; -*- lexical-binding: t; -*-\n(dolist (x (list 1 2 3)) (message \"%d\" x))");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        let HirExprKind::Let { body, .. } = &expr.kind else {
            panic!("expected let");
        };
        let HirExprKind::Progn(parts) = &body.kind else {
            panic!("expected progn body");
        };
        // After the while loop, there should be a setq setting x to nil
        // Structure: [while, setq(x, nil), result]
        assert!(
            parts.len() >= 2,
            "dolist should have while + setq nil + result"
        );
        let rendered = format!("{:?}", parts[1].kind);
        assert!(
            rendered.contains("Nil"),
            "second body expr after while should set var to nil, got: {rendered}"
        );
    }

    #[test]
    fn lowers_simple_backquote() {
        let module = hir(";;; -*- lexical-binding: t; -*-\n`(a b c)");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr, got: {:?}", module.items[0]);
        };
        // Should produce (list (quote a) (quote b) (quote c))
        let HirExprKind::CallNamed { name, args } = &expr.kind else {
            panic!("expected CallNamed, got: {:?}", expr.kind);
        };
        assert_eq!(name, "list");
        assert_eq!(args.len(), 3);
    }
}
