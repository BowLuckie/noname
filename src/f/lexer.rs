use crate::frontend::{
    self, Ctx, Diagnostic, Frontend, Severity, Span, diagnostic::ErrorKind, span,
};
use anyhow::Result;

#[derive(Debug)]
pub struct Token {
    span: Span,
    kind: TokenKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Error,
    Eof,
    Fn,
    Include,
    Intrinsic,
    Let,
}

struct Lexer<'a> {
    src: &'a [u8],
    ctx: &'a mut Ctx,
    pos: usize,
}

pub fn lex(source: &str, context: &mut Ctx) -> Vec<Token> {
    let mut lx = Lexer {
        src: source.as_bytes(),
        pos: 0,
        ctx: context,
    };

    let mut tokens = Vec::new();
    loop {
        let tok = lx.next_token();
        let done = tok.kind == TokenKind::Eof;
        tokens.push(tok);
        if done {
            break;
        }
    }

    tokens
}

impl Lexer<'_> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek_at(&self, n: usize) -> Option<u8> {
        self.src.get(self.pos + n).copied()
    }

    fn skip_trivial(&mut self) {
        while matches!(self.peek(), Some(b) if b.is_ascii_whitespace()) {
            self.pos += 1;
        }
        // TODO: comments, probably just // syntax
    }

    fn next_token(&mut self) -> Token {
        self.skip_trivial();
        let start = self.pos;

        let Some(b) = self.peek() else {
            return self.token(TokenKind::Eof, start);
        };

        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.consume_word(),
            _ => self.illegal_char(start),
        }
    }

    fn consume_word(&mut self) -> Token {
        todo!()
    }

    fn add_error(&mut self, diagnostic: Diagnostic) -> Token {
        Token {
            span: span(self.pos as u32, self.pos as u32 + 1),
            kind: TokenKind::Error,
        }
    }

    fn illegal_char(&mut self, start: usize) -> Token {
        self.pos += 1;
        while matches!(self.peek(), Some(b) if is_continuation(b)) {
            self.pos += 1; // keep consuming non-ascii bytes
        }

        let text: String = String::from_utf8_lossy(&self.src[start..self.pos]).into();
        self.ctx.diags.push(Diagnostic::error(
            span(start as u32, self.pos as u32),
            ErrorKind::IllegalChar,
            format!("IllegalChar: unexpected char `{text}`"),
        ));

        self.token(TokenKind::Error, start)
    }

    fn token(&self, kind: TokenKind, start: usize) -> Token {
        Token {
            span: span(start as u32, self.pos as u32),
            kind,
        }
    }
}

fn is_continuation(b: u8) -> bool {
    (b & 0b1100_0000) == 0b1000_0000
}
