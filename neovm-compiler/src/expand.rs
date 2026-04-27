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
            SurfaceKind::Vector(items) => SurfaceForm::new(
                SurfaceKind::Vector(
                    items
                        .into_iter()
                        .map(|item| self.expand_form(item))
                        .collect(),
                ),
                form.span,
            ),
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

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(message.into()).with_span(span));
    }
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
}
