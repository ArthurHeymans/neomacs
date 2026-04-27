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
    pub params: Vec<String>,
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
    Progn(Vec<HirExpr>),
    Let {
        mode: BindingMode,
        sequential: bool,
        declarations: Vec<HirDeclaration>,
        bindings: Vec<HirBinding>,
        body: Box<HirExpr>,
    },
    Lambda {
        params: Vec<String>,
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
    diagnostics: Vec<Diagnostic>,
}

impl Lowerer<'_> {
    fn lower_item(&mut self, form: &SurfaceForm) -> Option<HirItem> {
        if let Some(list) = list_items(form)
            && list.first().and_then(SurfaceForm::symbol_name) == Some("defun")
        {
            return self.lower_defun(form, list).map(HirItem::Defun);
        }
        self.lower_expr(form).map(HirItem::Expr)
    }

    fn lower_defun(&mut self, form: &SurfaceForm, list: &[SurfaceForm]) -> Option<HirDefun> {
        if list.len() < 4 {
            self.error(form.span, "defun requires a name, arg list, and body");
            return None;
        }
        let Some(name) = list[1].symbol_name().map(str::to_string) else {
            self.error(list[1].span, "defun name must be a symbol");
            return None;
        };
        let Some(params) = self.parse_param_list(&list[2]) else {
            return None;
        };
        let (declarations, body_forms) = self.split_leading_declarations(&list[3..]);
        self.push_special_scope(special_declared_names(&declarations));
        let lexical_params = params
            .iter()
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
            SurfaceKind::Vector(_) => {
                self.error(form.span, "vector expressions are not supported yet");
                None
            }
            SurfaceKind::DottedList(_, _) => {
                self.error(form.span, "dotted-list expressions are not supported here");
                None
            }
            SurfaceKind::Backquote(_) | SurfaceKind::Comma(_) | SurfaceKind::CommaAt(_) => {
                self.error(
                    form.span,
                    "backquote syntax requires macroexpansion support",
                );
                None
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
            Some("progn") => self.lower_progn(form, &items[1..]),
            Some("prog1") => self.lower_prog1(form, &items[1..]),
            Some("let") => self.lower_let(form, &items[1..], false),
            Some("let*") => self.lower_let(form, &items[1..], true),
            Some("lambda") => self.lower_lambda(form, items),
            Some("declare") => self.lower_declare(form, &items[1..]),
            Some("catch") => self.lower_catch(form, &items[1..]),
            Some("throw") => self.lower_throw(form, &items[1..]),
            Some("condition-case") => self.lower_condition_case(form, &items[1..]),
            Some("unwind-protect") => self.lower_unwind_protect(form, &items[1..]),
            Some("funcall") => self.lower_funcall(form, &items[1..]),
            Some("apply") => self.lower_apply(form, &items[1..]),
            Some("setq") => self.lower_setq(form, &items[1..]),
            Some(name) => self.lower_call_named(form, name, &items[1..]),
            None => self.lower_call_value(form, head, &items[1..]),
        }
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
        if items.len() != 2 {
            self.error(form.span, "function requires exactly one argument");
            return None;
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

    fn lower_if(&mut self, form: &SurfaceForm, items: &[SurfaceForm]) -> Option<HirExpr> {
        if !(3..=4).contains(&items.len()) {
            self.error(form.span, "if requires test, then, and optional else");
            return None;
        }
        let test = self.lower_expr(&items[1])?;
        let then_expr = self.lower_expr(&items[2])?;
        let else_expr = if let Some(else_form) = items.get(3) {
            self.lower_expr(else_form)?
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
        let binding_forms = list_items(&tail[0])?;
        let mut bindings = Vec::new();
        if !sequential {
            for binding_form in binding_forms {
                let binding = self.lower_binding(binding_form)?;
                bindings.push(binding);
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
                let binding = self.lower_binding(binding_form)?;
                if binding.mode == BindingMode::Lexical {
                    self.declare_local(binding.name.clone());
                }
                bindings.push(binding);
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
            self.error(form.span, "let binding must be a symbol or (symbol init)");
            return None;
        };
        if items.is_empty() || items.len() > 2 {
            self.error(form.span, "let binding must be a symbol or (symbol init)");
            return None;
        }
        let Some(name) = items[0].symbol_name().map(str::to_string) else {
            self.error(items[0].span, "let binding name must be a symbol");
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
        if items.len() < 3 {
            self.error(form.span, "lambda requires arg list and body");
            return None;
        }
        let params = self.parse_param_list(&items[1])?;
        let (declarations, body_forms) = self.split_leading_declarations(&items[2..]);
        self.push_special_scope(special_declared_names(&declarations));
        let lexical_params = params
            .iter()
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
        let var = if tail[0].symbol_name() == Some("nil") {
            None
        } else if let Some(name) = tail[0].symbol_name() {
            Some(name.to_string())
        } else {
            self.error(
                tail[0].span,
                "condition-case variable must be a symbol or nil",
            );
            return None;
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
            let body = self.lower_body(&items[1..], handler_form.span)?;
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
        if tail.len() < 2 {
            self.error(form.span, "apply requires a function and arguments");
            return None;
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
        if pairs.is_empty() || !pairs.len().is_multiple_of(2) {
            self.error(form.span, "setq requires symbol/value pairs");
            return None;
        }
        let mut exprs = Vec::new();
        for pair in pairs.chunks_exact(2) {
            let Some(name) = pair[0].symbol_name().map(str::to_string) else {
                self.error(pair[0].span, "setq target must be a symbol");
                return None;
            };
            let value = self.lower_expr(&pair[1])?;
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
                span: form.span,
            });
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

    fn parse_param_list(&mut self, form: &SurfaceForm) -> Option<Vec<String>> {
        let Some(items) = list_items(form) else {
            self.error(form.span, "parameter list must be a proper list");
            return None;
        };
        let mut params = Vec::new();
        for item in items {
            let Some(name) = item.symbol_name() else {
                self.error(item.span, "parameter name must be a symbol");
                return None;
            };
            if name.starts_with('&') {
                self.error(item.span, "lambda-list keywords are not supported yet");
                return None;
            }
            params.push(name.to_string());
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
    fn top_level_symbol_reads_are_symbol_gets() {
        let module = hir("x");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        assert!(matches!(expr.kind, HirExprKind::SymbolGet(_)));
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
    fn lowers_prog1_to_internal_lexical_temp() {
        let module = hir(";;; -*- lexical-binding: t; -*-\n(prog1 1 (message \"side\"))");
        let HirItem::Expr(expr) = &module.items[0] else {
            panic!("expected expr");
        };
        assert!(matches!(expr.kind, HirExprKind::Let { .. }));
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
        assert_eq!(params, &vec!["y".to_string()]);
        let HirExprKind::CallNamed { args, .. } = &body.kind else {
            panic!("expected lambda call body");
        };
        assert!(matches!(args[0].kind, HirExprKind::LexicalGet(_)));
        assert!(matches!(args[1].kind, HirExprKind::LexicalGet(_)));
    }
}
