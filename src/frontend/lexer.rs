use crate::frontend::{
    self, Ctx, Diagnostic, Frontend, Severity, Span, Sym, diagnostic::ErrorKind, span, span_usize,
};
use anyhow::Result;
use std::{
    num::{IntErrorKind, ParseIntError},
    result, str,
};

#[derive(Debug)]
pub struct Token {
    span: Span,
    kind: TokenKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Suffix {
    U1,
    U8,
    U16,
    U32,
    U64,
    Usize,
    I8,
    I16,
    I32,
    I64,
    Isize,
    F32,
    F64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Ident(Sym),
    Int(u64, Option<Suffix>),
    Float(u64, Option<Suffix>),

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
    Dot,
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

struct Lexer<'src> {
    src: &'src [u8],
    ctx: &'src mut Ctx,
    pos: usize,
}

/// Lexer entry point
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
            b'0'..=b'9' => return self.consume_number(),

            b'(' => self.single(TokenKind::LParen),
            b')' => self.single(TokenKind::RParen),
            b'{' => self.single(TokenKind::LBrace),
            b'}' => self.single(TokenKind::RBrace),
            b'[' => self.single(TokenKind::LBracket),
            b']' => self.single(TokenKind::RBracket),
            b',' => self.single(TokenKind::Comma),
            b'.' => self.single(TokenKind::Dot),
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
        self.consume_while(ident_legal);
        let end = self.pos;

        let word = str::from_utf8(&self.src[start..end])
            .expect("illegal characters will not be consumed, and cant be in this slice");
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

    fn consume_number(&mut self) -> Token {
        debug_assert!(self.peek().unwrap().is_ascii_digit());
        let start = self.pos;

        let radix = match (self.peek(), self.peek_at(1)) {
            (Some(b'0'), Some(b'x')) => 16,
            (Some(b'0'), Some(b'b')) => 2,
            (Some(b'0'), Some(b'o')) => 8,
            _ => 10,
        };

        if radix != 10 {
            self.pos += 2;
        }

        let digits_start = self.pos;
        self.consume_while(|b| number_legal(b, radix));

        let float = radix == 10
            && self.peek() == Some(b'.')
            && matches!(self.peek_at(1), Some(b) if b.is_ascii_digit());
        if float {
            self.pos += 1;
            self.consume_while(|b| number_legal(b, 10));
        }

        let digits_end = self.pos;

        if let Some(b) = self.peek()
            && b.is_ascii_digit()
        {
            let msg =
                &format!("invalid digit `{}` in base-{} literal", b as char, radix).to_string();
            self.consume_while(ident_legal);
            return self.bad_number(start, msg);
        }

        let suffix_start = self.pos;
        self.consume_while(ident_legal);
        let suffix_end = self.pos;
        let end = suffix_end;

        let digits = str::from_utf8(&self.src[digits_start..digits_end])
            .expect("illegal characters will not be consumed");
        let suffix_s = str::from_utf8(&self.src[suffix_start..suffix_end])
            .expect("illegal characters will not be consumed");

        let cleaned = digits.chars().filter(|&c| c != '_').collect::<String>();

        if cleaned.is_empty() {
            return self.bad_number(
                start,
                &format!("Missing digits after base-{} prefix", radix).to_string(),
            );
        }

        let suffix = Lexer::parse_suffix(suffix_s);
        if suffix.is_none() && !suffix_s.is_empty() {
            self.ctx.diags.push(
                Diagnostic::error(
                    span_usize(start, end),
                    ErrorKind::InvalidSuffix,
                    format!(
                        "InvalidSuffix: unknown or invalid literal suffix {}",
                        suffix_s
                    ),
                )
                .with_note("Try a primitive number type like u1, i16, or f32"),
            );
            return self.token(TokenKind::Error, start);
        }

        if float {
            let n = match cleaned.parse::<f64>() {
                Ok(n) => n,
                Err(_) => return self.bad_number(start, "invalid float literal"),
            };

            self.token(TokenKind::Float(n.to_bits(), suffix), start)
        } else {
            let n = match u64::from_str_radix(&cleaned, radix) {
                Ok(n) => n,
                Err(e) => {
                    let msg = if let IntErrorKind::PosOverflow = e.kind() {
                        "BadNumber: int literal too large"
                    } else {
                        &format!("BadNumber: invalid int literal {}", cleaned).to_string()
                    };
                    return self.bad_number(start, msg);
                }
            };

            self.token(TokenKind::Int(n, suffix), start)
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

    fn consume_while(&mut self, pred: impl Fn(u8) -> bool) {
        while matches!(self.peek(), Some(b) if pred(b)) {
            self.pos += 1;
        }
    }

    fn parse_suffix(s: &str) -> Option<Suffix> {
        Some(match s {
            "u8" => Suffix::U8,
            "u16" => Suffix::U16,
            "u32" => Suffix::U32,
            "u64" => Suffix::U64,
            "usize" => Suffix::Usize,
            "i8" => Suffix::I8,
            "i16" => Suffix::I16,
            "i32" => Suffix::I32,
            "i64" => Suffix::I64,
            "isize" => Suffix::Isize,
            "f32" => Suffix::F32,
            "f64" => Suffix::F64,
            _ => return None,
        })
    }

    fn bad_number(&mut self, start: usize, msg: &str) -> Token {
        self.ctx.diags.push(Diagnostic::error(
            span_usize(start, self.pos),
            ErrorKind::BadNumber,
            format!("BadNumber: {}", msg),
        ));

        self.token(TokenKind::Error, start)
    }

    fn at_exponant(&self) -> bool {
        if matches!(self.peek(), Some(b'e' | b'E')) {
            return false;
        }
        match self.peek_at(1) {
            Some(b) if b.is_ascii_digit() => true,
            Some(b'+' | b'-') => matches!(self.peek_at(2), Some(b) if b.is_ascii_digit()),
            _ => false,
        }
    }
}

fn is_continuation(b: u8) -> bool {
    (b & 0b1100_0000) == 0b1000_0000
}

fn ident_legal(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn number_legal(b: u8, radix: u32) -> bool {
    b == b'_' || (b as char).is_digit(radix)
}
