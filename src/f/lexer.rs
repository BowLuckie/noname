use std::str;

use crate::frontend::{
    self, Ctx, Diagnostic, Frontend, Severity, Span, Sym, diagnostic::ErrorKind, span,
};
use anyhow::Result;

#[derive(Debug)]
pub struct Token {
    span: Span,
    kind: TokenKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Ident(Sym),

    Fn,
    Include,
    Intrinsic,
    Let,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    Comma,
    Semi,
    Colon,
    ColonColon, // ::
    Arrow,      // ->

    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,     // =
    EqEq,   // ==
    Bang,   // !
    BangEq, // !=
    Lt,
    LtEq,
    Gt,
    GtEq,
    Underscore,

    Error,
    Eof,
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

        let kind = match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => return self.consume_word(),

            b'(' => self.single(TokenKind::LParen),
            b')' => self.single(TokenKind::RParen),
            b'{' => self.single(TokenKind::LBrace),
            b'}' => self.single(TokenKind::RBrace),
            b'[' => self.single(TokenKind::LBracket),
            b']' => self.single(TokenKind::RBracket),
            b',' => self.single(TokenKind::Comma),
            b';' => self.single(TokenKind::Semi),
            b'+' => self.single(TokenKind::Plus),
            b'*' => self.single(TokenKind::Star),
            b'%' => self.single(TokenKind::Percent),

            b'=' => self.one_or_two(b'=', TokenKind::EqEq, TokenKind::Eq),
            b'!' => self.one_or_two(b'=', TokenKind::BangEq, TokenKind::Bang),
            b'<' => self.one_or_two(b'=', TokenKind::LtEq, TokenKind::Lt),
            b'>' => self.one_or_two(b'=', TokenKind::GtEq, TokenKind::Gt),
            b'-' => self.one_or_two(b'>', TokenKind::Arrow, TokenKind::Minus),
            b':' => self.one_or_two(b':', TokenKind::ColonColon, TokenKind::Colon),

            _ => return self.illegal_char(),
        };

        self.token(kind, start)
    }

    fn consume_word(&mut self) -> Token {
        let start = self.pos;
        while matches!(self.peek(), Some(b) if b.is_ascii_alphanumeric() || b == b'_') {
            self.pos += 1; // consume legal char
        }
        let end = self.pos;

        let word = str::from_utf8(&self.src[start..end])
            .expect("illegal charects will not be consumed, and cant be in this slice");
        let kind = match word {
            "_" => TokenKind::Underscore,
            "fn" => TokenKind::Fn,
            "include" => TokenKind::Include,
            "intrinsic" => TokenKind::Intrinsic,
            "let" => TokenKind::Let,
            _ => TokenKind::Ident(self.ctx.interner.intern(word)),
        };

        self.token(kind, start)
    }

    fn add_error(&mut self, diagnostic: Diagnostic) -> Token {
        Token {
            span: span(self.pos as u32, self.pos as u32 + 1),
            kind: TokenKind::Error,
        }
    }

    fn illegal_char(&mut self) -> Token {
        let start = self.pos;
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

    fn single(&mut self, kind: TokenKind) -> TokenKind {
        self.pos += 1;
        kind
    }

    fn one_or_two(&mut self, second: u8, two: TokenKind, one: TokenKind) -> TokenKind {
        self.pos += 1;
        if self.peek() == Some(second) {
            self.pos += 1;
            two
        } else {
            one
        }
    }
}

fn is_continuation(b: u8) -> bool {
    (b & 0b1100_0000) == 0b1000_0000
}
