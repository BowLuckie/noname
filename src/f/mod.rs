use std::path::Path;

use anyhow::Result;

use crate::frontend::lexer::lex;

pub mod lexer;

#[derive(Default, Debug)]
pub struct Frontend {
    source: Box<str>,
    // internals to come
}

impl Frontend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn assign_source(&mut self, source: Box<str>) {
        self.source = source
    }

    pub fn run(&mut self) -> Result<()> {
        lex(self);
        // do the stuff
        Ok(())
    }
}
