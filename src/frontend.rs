use anyhow::{Context, Result};
use std::fmt::Write;
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
pub use lexer::{Suffix, Token, TokenKind};
pub use literals::{Literals, StrId};
pub use source::*;

use crate::CompilerOptions;
use crate::{frontend::diagnostic::CompileFailed, utils::Color};

pub type SourceLoaderFn = Box<dyn Fn(&Path) -> io::Result<String>>;

pub struct Frontend {
    sources: SourceMap,
    ctx: Ctx,
    root: Option<PathBuf>,
    loader: SourceLoaderFn,
    out: Box<dyn io::Write>,
}

impl Frontend {
    pub fn new() -> Self {
        Self::with_loader(|p| fs::read_to_string(p))
    }

    pub fn with_output(mut self, out: impl io::Write + 'static) -> Self {
        self.out = Box::new(out);
        self
    }

    pub fn add_file(&mut self, name: impl Into<PathBuf>, text: impl Into<String>) -> FileId {
        self.sources.add(name.into(), text.into())
    }

    pub fn run(&mut self, opts: &CompilerOptions) -> Result<()> {
        // TODO: when parsing, collect all the includes and queue those files for lexing and parsing
        // avoiding cyclic imports

        let root = self.root.clone().context("no root loaded")?;
        let text = (self.loader)(root.as_path())
            .with_context(|| format!("falied to read {}", root.display()))?;

        let id = self.sources.add(root, text);
        let tokens = Frontend::lex(&self.sources, id, &mut self.ctx);

        if opts.emit_tokens {
            let (hi, reset) = if opts.color {
                (Color::Green.to_string(), Color::Reset.to_string())
            } else {
                Default::default()
            };
            writeln!(self.out, "{hi}Output tokens{reset}")?;
            for t in &tokens {
                writeln!(self.out, "{t}")?;
            }
        }

        if self.ctx.has_error() {
            return Err(CompileFailed {
                errors: self.ctx.error_count(),
            }
            .into());
        }

        Ok(())
    }

    pub fn set_root(&mut self, path: impl Into<PathBuf>) {
        self.root = Some(path.into());
    }

    fn with_loader(loader: impl Fn(&Path) -> io::Result<String> + 'static) -> Frontend {
        Frontend {
            loader: Box::new(loader),
            sources: Default::default(),
            ctx: Default::default(),
            root: Default::default(),
            out: Box::new(io::stdout()),
        }
    }

    pub fn render_diagnostics(&self, color: bool) -> String {
        let mut out = String::new();
        for d in &self.ctx.diags {
            let _ = writeln!(out, "{}", d.render(&self.sources, color));
        }

        out
    }
}

impl Default for Frontend {
    fn default() -> Self {
        Self::new()
    }
}
