use crate::{
    frontend::{SourceMap, Span},
    utils::Color,
};
use core::fmt::{self, Display, Formatter};
use std::error;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub kind: ErrorKind,
    pub message: String,
    pub span: Span,
    pub labels: Vec<(Span, String)>,
    pub notes: Vec<String>,
}

pub struct DiagnosticRenderer<'a> {
    diag: &'a Diagnostic,
    sources: &'a SourceMap,
    color: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

impl From<Severity> for Color {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Warning => Color::Yellow,
            Severity::Error => Color::BrightRed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    IllegalChar,
    BadNumber,
    UnterminatedComment,
    BadTextLiteral,
    BadEscape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompileFailed {
    pub errors: usize,
}

impl Display for CompileFailed {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.errors {
            1 => write!(f, "aborting due to 1 error"),
            n => write!(f, "aborting due to {n} errors"),
        }
    }
}

impl error::Error for CompileFailed {}

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

    pub fn render<'a>(&'a self, sources: &'a SourceMap, color: bool) -> DiagnosticRenderer<'a> {
        DiagnosticRenderer {
            diag: self,
            sources,
            color,
        }
    }
}
impl DiagnosticRenderer<'_> {
    fn format_span(&self, f: &mut impl fmt::Write, span: Span, hl: &str) -> fmt::Result {
        let (file, start_line, start_col) = self.sources.locate(span);
        let text = file.line_text(start_line);

        let end_local = span.end() - file.base;
        let (end_line, end_col) = file.line_col(end_local);

        let width = if end_line == start_line {
            end_col - start_col
        } else {
            text.chars().count().saturating_sub(start_col)
        }
        .max(1);

        let gutter = (start_line + 1).to_string();
        let pad = " ".repeat(gutter.len());

        writeln!(f, "{pad}--> {}", span.format(self.sources)?)?;
        writeln!(f, "{pad} |")?;
        writeln!(f, "{gutter} | {text}")?;
        writeln!(
            f,
            "{pad} | {}{hl}{}{reset}",
            " ".repeat(start_col),
            "^".repeat(width),
            reset = if self.color {
                Color::Reset.to_string()
            } else {
                String::new()
            },
        )?;

        Ok(())
    }

    fn format_label(&self, f: &mut impl fmt::Write, label: &(Span, String)) -> fmt::Result {
        let (hl, reset) = if self.color {
            (Color::BrightCyan.to_string(), Color::Reset.to_string())
        } else {
            Default::default()
        };

        writeln!(f, "{hl}Help[{:?}]{reset}: {}", self.diag.kind, label.1,)?;

        self.format_span(f, label.0, &hl)?;

        Ok(())
    }
}

impl Display for DiagnosticRenderer<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let d = self.diag;

        let (hl, bold, reset, note_hl) = if self.color {
            (
                Color::from(d.severity).to_string(),
                Color::Bold.to_string(),
                Color::Reset.to_string(),
                Color::BrightCyan.to_string(),
            )
        } else {
            Default::default()
        };

        writeln!(
            f,
            "{hl}{:?}[{:?}]{reset}{bold}: {}{reset}",
            d.severity, d.kind, d.message
        )?;

        self.format_span(f, d.span, &hl)?;

        for label in &d.labels {
            self.format_label(f, label)?;
        }

        for note in &d.notes {
            writeln!(f, "{note_hl}- note{reset}: {note}")?;
        }

        Ok(())
    }
}
