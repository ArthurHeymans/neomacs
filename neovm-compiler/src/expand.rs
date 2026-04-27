use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::surface::{SurfaceAtom, SurfaceForm, SurfaceKind};

#[derive(Clone, Debug, PartialEq)]
pub struct ExpandOutput {
    pub forms: Vec<SurfaceForm>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn expand_forms(forms: Vec<SurfaceForm>) -> ExpandOutput {
    let mut expander = Expander {
        diagnostics: Vec::new(),
    };
    let forms = forms
        .into_iter()
        .map(|form| expander.expand_form(form))
        .collect();
    ExpandOutput {
        forms,
        diagnostics: expander.diagnostics,
    }
}

struct Expander {
    diagnostics: Vec<Diagnostic>,
}

impl Expander {
    fn expand_form(&mut self, form: SurfaceForm) -> SurfaceForm {
        match form.kind {
            SurfaceKind::List(items) => self.expand_list(form.span, items),
            SurfaceKind::DottedList(items, tail) => SurfaceForm::new(
                SurfaceKind::DottedList(
                    items
                        .into_iter()
                        .map(|item| self.expand_form(item))
                        .collect(),
                    Box::new(self.expand_form(*tail)),
                ),
                form.span,
            ),
            SurfaceKind::Vector(_) => form,
            SurfaceKind::Quote(_)
            | SurfaceKind::FunctionQuote(_)
            | SurfaceKind::Backquote(_)
            | SurfaceKind::Comma(_)
            | SurfaceKind::CommaAt(_)
            | SurfaceKind::Atom(_) => form,
        }
    }

    fn expand_list(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        let Some(head) = items.first().and_then(SurfaceForm::symbol_name) else {
            return SurfaceForm::new(
                SurfaceKind::List(
                    items
                        .into_iter()
                        .map(|item| self.expand_form(item))
                        .collect(),
                ),
                span,
            );
        };
        match head {
            "quote" | "function" => SurfaceForm::new(SurfaceKind::List(items), span),
            "push" => self.expand_push(span, items),
            "pop" => self.expand_pop(span, items),
            "if-let*" => self.expand_if_let(span, items),
            "when-let*" => self.expand_when_let(span, items),
            _ => SurfaceForm::new(
                SurfaceKind::List(
                    items
                        .into_iter()
                        .map(|item| self.expand_form(item))
                        .collect(),
                ),
                span,
            ),
        }
    }

    fn expand_push(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() != 3 {
            self.error(span, "push requires a value and a symbol place");
            return SurfaceForm::new(SurfaceKind::List(items), span);
        }
        let Some(place) = items[2].symbol_name().map(str::to_string) else {
            self.error(
                items[2].span,
                "push supports only simple symbol places for now",
            );
            return SurfaceForm::new(SurfaceKind::List(items), span);
        };
        let value = items[1].clone();
        let expanded = list_form(
            vec![
                symbol_form("setq", span),
                symbol_form(&place, items[2].span),
                list_form(
                    vec![
                        symbol_form("cons", span),
                        value,
                        symbol_form(&place, items[2].span),
                    ],
                    span,
                ),
            ],
            span,
        );
        self.expand_form(expanded)
    }

    fn expand_pop(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() != 2 {
            self.error(span, "pop requires a symbol place");
            return SurfaceForm::new(SurfaceKind::List(items), span);
        }
        let Some(place) = items[1].symbol_name().map(str::to_string) else {
            self.error(
                items[1].span,
                "pop supports only simple symbol places for now",
            );
            return SurfaceForm::new(SurfaceKind::List(items), span);
        };
        let place_span = items[1].span;
        let expanded = list_form(
            vec![
                symbol_form("car-safe", span),
                list_form(
                    vec![
                        symbol_form("prog1", span),
                        symbol_form(&place, place_span),
                        list_form(
                            vec![
                                symbol_form("setq", span),
                                symbol_form(&place, place_span),
                                list_form(
                                    vec![symbol_form("cdr", span), symbol_form(&place, place_span)],
                                    span,
                                ),
                            ],
                            span,
                        ),
                    ],
                    span,
                ),
            ],
            span,
        );
        self.expand_form(expanded)
    }

    fn expand_if_let(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() < 3 {
            self.error(span, "if-let* requires bindings and a then form");
            return SurfaceForm::new(SurfaceKind::List(items), span);
        }
        let Some(bindings) = self.parse_if_let_bindings(&items[1]) else {
            return SurfaceForm::new(SurfaceKind::List(items), span);
        };
        let then_form = items[2].clone();
        let else_forms = items[3..].to_vec();
        let expanded = build_if_let_form(bindings, then_form, else_forms, span);
        self.expand_form(expanded)
    }

    fn expand_when_let(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() < 2 {
            self.error(span, "when-let* requires bindings");
            return SurfaceForm::new(SurfaceKind::List(items), span);
        }
        let Some(bindings) = self.parse_if_let_bindings(&items[1]) else {
            return SurfaceForm::new(SurfaceKind::List(items), span);
        };
        let then_form = list_form(
            std::iter::once(symbol_form("progn", span))
                .chain(items[2..].iter().cloned())
                .collect(),
            span,
        );
        let expanded = build_if_let_form(bindings, then_form, Vec::new(), span);
        self.expand_form(expanded)
    }

    fn parse_if_let_bindings(&mut self, form: &SurfaceForm) -> Option<Vec<IfLetBinding>> {
        let SurfaceKind::List(items) = &form.kind else {
            if matches!(form.kind, SurfaceKind::Atom(SurfaceAtom::Nil)) {
                return Some(Vec::new());
            }
            self.error(form.span, "if-let* bindings must be a proper list");
            return None;
        };
        items
            .iter()
            .enumerate()
            .map(|(index, item)| self.parse_if_let_binding(item, index))
            .collect()
    }

    fn parse_if_let_binding(&mut self, form: &SurfaceForm, index: usize) -> Option<IfLetBinding> {
        if let Some(name) = form.symbol_name() {
            return Some(IfLetBinding {
                name: name.to_string(),
                value: form.clone(),
                span: form.span,
            });
        }
        let SurfaceKind::List(items) = &form.kind else {
            self.error(
                form.span,
                "if-let* binding must be SYMBOL, (SYMBOL VALUE), or (VALUE)",
            );
            return None;
        };
        match items.as_slice() {
            [value] => Some(IfLetBinding {
                name: generated_if_let_name(form.span, index),
                value: value.clone(),
                span: form.span,
            }),
            [name, value] => {
                let Some(name) = name.symbol_name() else {
                    self.error(name.span, "if-let* binding name must be a symbol");
                    return None;
                };
                let name = if name == "_" {
                    generated_if_let_name(form.span, index)
                } else {
                    name.to_string()
                };
                Some(IfLetBinding {
                    name,
                    value: value.clone(),
                    span: form.span,
                })
            }
            _ => {
                self.error(
                    form.span,
                    "if-let* binding must be SYMBOL, (SYMBOL VALUE), or (VALUE)",
                );
                None
            }
        }
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(message.into()).with_span(span));
    }
}

#[derive(Clone, Debug, PartialEq)]
struct IfLetBinding {
    name: String,
    value: SurfaceForm,
    span: Span,
}

fn build_if_let_form(
    bindings: Vec<IfLetBinding>,
    then_form: SurfaceForm,
    else_forms: Vec<SurfaceForm>,
    span: Span,
) -> SurfaceForm {
    if bindings.is_empty() {
        return list_form(
            vec![
                symbol_form("let*", span),
                list_form(Vec::new(), span),
                then_form,
            ],
            span,
        );
    }

    let mut previous = symbol_form("t", span);
    let mut binding_forms = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let current = symbol_form(&binding.name, binding.span);
        let value = list_form(
            vec![symbol_form("and", span), previous, binding.value],
            binding.span,
        );
        binding_forms.push(list_form(vec![current.clone(), value], binding.span));
        previous = current;
    }

    let mut if_items = vec![symbol_form("if", span), previous, then_form];
    if_items.extend(else_forms);
    list_form(
        vec![
            symbol_form("let*", span),
            list_form(binding_forms, span),
            list_form(if_items, span),
        ],
        span,
    )
}

fn generated_if_let_name(span: Span, index: usize) -> String {
    format!("\0if-let.{}.{}", span.start, index)
}

fn symbol_form(name: &str, span: Span) -> SurfaceForm {
    SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::symbol(name)), span)
}

fn list_form(items: Vec<SurfaceForm>, span: Span) -> SurfaceForm {
    SurfaceForm::new(SurfaceKind::List(items), span)
}

#[cfg(test)]
mod tests {
    use crate::compile_source;

    #[test]
    fn expands_push_and_pop_for_simple_symbol_places() {
        let artifact = compile_source(
            "push-pop.el",
            ";;; -*- lexical-binding: t; -*-\n(let ((xs nil)) (push 1 xs) (pop xs))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"setq\""));
        assert!(rendered.contains("\"car-safe\""));
    }

    #[test]
    fn expands_simple_if_let_and_when_let_star() {
        let artifact = compile_source(
            "if-let.el",
            ";;; -*- lexical-binding: t; -*-\n(progn (if-let* ((x 1) (_ x) ((+ x 1))) x 0) (when-let* ((y 2)) y))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"let*\""));
        assert!(rendered.contains("\"and\""));
        assert!(rendered.contains("\"if\""));
        assert!(rendered.contains("\"progn\""));
    }
}
