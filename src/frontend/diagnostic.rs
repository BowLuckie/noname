use crate::frontend::Span;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub kind: ErrorKind,
    pub span: Span,
    pub labels: Vec<(Span, String)>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    IllegalChar,
    BadNumber,
    UnterminatedComment,
    BadTextLiteral,
    BadEscape,
}

impl Diagnostic {
    pub fn error(span: Span, error_kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            kind: error_kind,
            message: message.into(),
            span,
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_label(mut self, span: Span, text: impl Into<String>) -> Self {
        self.labels.push((span, text.into()));
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}
