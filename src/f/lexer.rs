use anyhow::Result;

use crate::frontend::Frontend;

pub struct Token {
    span: Span,
    kind: TokenKind,
}

pub struct Span {
    start: u32,
    end: u32,
}

pub enum TokenKind {
    Error,
}

pub fn lex(state: &mut Frontend) -> Result<Vec<Token>> {
    let tokens = Vec::new();
    Ok(tokens)
}
