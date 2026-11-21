use anyhow::{Context, Result};
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};

mod ctx;
mod diagnostic;
mod interner;
mod lexer;
mod literals;
mod source;

pub use ctx::Ctx;
pub use diagnostic::{Diagnostic, Severity};
pub use interner::{Interner, Sym};
pub use lexer::{Suffix, Token, TokenKind, lex};
pub use literals::{Literals, StrId};
pub use source::*;

use crate::frontend::diagnostic::CompileFailed;

pub type SourceLoaderFn = Box<dyn Fn(&Path) -> io::Result<String>>;

pub struct Frontend {
    sources: SourceMap,
    ctx: Ctx,
    root: Option<PathBuf>,
    loader: SourceLoaderFn,
}

impl Frontend {
    pub fn new() -> Self {
        Self::with_loader(|p| fs::read_to_string(p))
    }

    pub fn add_file(&mut self, name: impl Into<PathBuf>, text: impl Into<String>) -> FileId {
        self.sources.add(name.into(), text.into())
    }

    pub fn run(&mut self) -> Result<()> {
        // TODO: when parsing, collect all the includes and queue those files for lexing and parsing
        // avoiding cyclic imports

        let root = self.root.clone().context("no root loaded")?;
        let text = (self.loader)(root.as_path())
            .with_context(|| format!("falied to read {}", root.display()))?;

        let id = self.sources.add(root, text);
        let tokens = lex(&self.sources, id, &mut self.ctx);

        println!("\x1b[0;32mOutput tokens\x1b[0m");
        dbg!(tokens);
        if self.ctx.has_error() {
            return Err(CompileFailed {
                errors: self.ctx.error_count(),
            }
            .into());
        }
        Ok(())
    }

    pub fn set_root(&mut self, path: String) {
        self.root = Some(path.into());
    }

    fn with_loader(loader: impl Fn(&Path) -> io::Result<String> + 'static) -> Frontend {
        Frontend {
            loader: Box::new(loader),
            sources: Default::default(),
            ctx: Default::default(),
            root: Default::default(),
        }
    }

    pub fn render_diagnostics(&self) -> String {
        todo!("render diagnostics")
    }
}

impl Default for Frontend {
    fn default() -> Self {
        Self::new()
    }
}
