use anyhow::Result;
use std::path::Path;

mod ctx;
mod diagnostic;
mod interner;
mod lexer;
mod literals;

pub use ctx::Ctx;
pub use diagnostic::{Diagnostic, Severity};
pub use interner::{Interner, Sym};
pub use lexer::{Token, TokenKind, lex};
pub use literals::{Literals, StrId};

#[derive(Default, Debug)]
pub struct Frontend {
    source: Box<str>,
    context: Ctx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    start: u32,
    end: u32,
}

pub(crate) fn span(start: impl Into<u32>, end: impl Into<u32>) -> Span {
    Span {
        start: start.into(),
        end: end.into(),
    }
}

impl Frontend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn assign_source(&mut self, source: impl Into<String>) {
        self.source = source.into().into_boxed_str()
    }

    pub fn run(&mut self) -> Result<()> {
        let tokens = lex(&self.source, &mut self.context);
        self.report_errors()?;
        println!("\x1b[0;32mOutput tokens\x1b[0m");
        dbg!(tokens);
        // do the stuff
        Ok(())
    }

    fn report_errors(&self) -> Result<()> {
        if self.context.has_error() {
            eprintln!("\x1b[31mSome errors have occured!\x1b[0m");
            dbg!(&self.context.diags);
        }

        Ok(())
    }
}
