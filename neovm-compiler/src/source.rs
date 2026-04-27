#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId(u32);

impl SourceId {
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    pub source: SourceId,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(source: SourceId, start: usize, end: usize) -> Self {
        Self { source, start, end }
    }

    pub const fn point(source: SourceId, offset: usize) -> Self {
        Self {
            source,
            start: offset,
            end: offset,
        }
    }

    pub const fn len(self) -> usize {
        self.end - self.start
    }

    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub const fn join(self, other: Self) -> Self {
        debug_assert!(self.source.0 == other.source.0);
        Self {
            source: self.source,
            start: self.start,
            end: other.end,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    pub id: SourceId,
    pub name: Option<String>,
    pub text: String,
    pub lexical_binding: bool,
}

impl SourceFile {
    pub fn new(id: SourceId, name: Option<String>, text: String) -> Self {
        let lexical_binding = detect_lexical_binding(&text).unwrap_or(false);
        Self {
            id,
            name,
            text,
            lexical_binding,
        }
    }

    pub fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.id, start, end)
    }

    pub fn eof_span(&self) -> Span {
        Span::point(self.id, self.text.len())
    }
}

/// Detect the common file-local lexical-binding marker.
///
/// This is intentionally conservative. Full GNU file-local variable handling
/// belongs in a later reader/source metadata pass.
pub fn detect_lexical_binding(text: &str) -> Option<bool> {
    for line in text.lines().take(2) {
        let marker = "lexical-binding:";
        let Some(pos) = line.find(marker) else {
            continue;
        };
        let value_start = pos + marker.len();
        let value = line[value_start..]
            .trim_start()
            .split(|ch: char| ch == ';' || ch == '-' || ch.is_whitespace())
            .next()
            .unwrap_or_default();
        return match value {
            "t" | "true" => Some(true),
            "nil" | "false" => Some(false),
            _ => None,
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_lexical_binding_true() {
        assert_eq!(
            detect_lexical_binding(";;; -*- lexical-binding: t; -*-\n(defun f ())"),
            Some(true)
        );
    }

    #[test]
    fn detects_lexical_binding_false() {
        assert_eq!(
            detect_lexical_binding(";;; -*- lexical-binding: nil; -*-\n"),
            Some(false)
        );
    }

    #[test]
    fn missing_lexical_binding_is_unknown() {
        assert_eq!(detect_lexical_binding("(defun f ())"), None);
    }
}
