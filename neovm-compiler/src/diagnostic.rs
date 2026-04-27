use crate::source::Span;

use ariadne::{Label, Report, ReportKind, Source};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
    pub notes: Vec<DiagnosticNote>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticNote {
    pub message: String,
    pub span: Option<Span>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span: None,
            notes: Vec::new(),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span: None,
            notes: Vec::new(),
        }
    }

    pub fn note(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Note,
            message: message.into(),
            span: None,
            notes: Vec::new(),
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_note(mut self, message: impl Into<String>, span: Option<Span>) -> Self {
        self.notes.push(DiagnosticNote {
            message: message.into(),
            span,
        });
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

pub fn render_diagnostics(
    source_name: &str,
    source_text: &str,
    diagnostics: &[Diagnostic],
) -> String {
    let mut out = Vec::new();
    for diagnostic in diagnostics {
        let kind = match diagnostic.severity {
            Severity::Error => ReportKind::Error,
            Severity::Warning => ReportKind::Warning,
            Severity::Note => ReportKind::Advice,
        };
        let range = diagnostic
            .span
            .map(|span| span.start..span.end)
            .unwrap_or(0..0);
        let mut report = Report::build(kind, (source_name, range.clone()))
            .with_message(diagnostic.message.clone());
        if diagnostic.span.is_some() {
            report = report.with_label(
                Label::new((source_name, range)).with_message(diagnostic.message.clone()),
            );
        }
        for note in &diagnostic.notes {
            report = if let Some(span) = note.span {
                report.with_label(
                    Label::new((source_name, span.start..span.end))
                        .with_message(note.message.clone()),
                )
            } else {
                report.with_note(note.message.clone())
            };
        }
        report
            .finish()
            .write((source_name, Source::from(source_text)), &mut out)
            .expect("render diagnostic");
    }
    String::from_utf8(out).expect("ariadne rendered utf8")
}

#[cfg(test)]
mod tests {
    use crate::source::{SourceId, Span};

    use super::*;

    #[test]
    fn renders_diagnostic_with_ariadne() {
        let source = "(if";
        let diagnostic =
            Diagnostic::error("unterminated list").with_span(Span::new(SourceId::new(0), 0, 3));
        let rendered = render_diagnostics("bad.el", source, &[diagnostic]);
        assert!(rendered.contains("unterminated list"));
        assert!(rendered.contains("bad.el"));
    }
}
