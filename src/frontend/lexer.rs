use crate::frontend::{Ctx, Diagnostic, ErrorKind, FileId, SourceMap, Span, StrId, Sym};
use std::{
    fmt::{self, Display, Formatter},
    num::IntErrorKind,
    str,
};

struct Lexer<'src> {
    src: &'src [u8],
    ctx: &'src mut Ctx,
    base: u32,
    pos: usize,
}

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
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
    Str(StrId),
    Char(u8),

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

/// Lexer entry point
pub(super) fn lex(sources: &SourceMap, file: FileId, ctx: &mut Ctx) -> Vec<Token> {
    let f = sources.file(file);
    let mut lx = Lexer {
        src: f.text.as_bytes(),
        pos: 0,
        ctx,
        base: f.base,
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

    fn skip_chars(&mut self) {
        loop {
            self.consume_while(|b| b.is_ascii_whitespace());
            match (self.peek(), self.peek_at(1)) {
                (Some(b'/'), Some(b'/')) => self.consume_while(|b| b != b'\n'),
                (Some(b'/'), Some(b'*')) => self.block_comment(),
                _ => break,
            }
        }
    }

    fn block_comment(&mut self) {
        let start = self.pos;
        self.pos += 2;
        let mut depth = 1;
        while depth > 0 {
            match (self.peek(), self.peek_at(1)) {
                (Some(b'*'), Some(b'/')) => {
                    self.pos += 2;
                    depth -= 1;
                }
                (Some(b'/'), Some(b'*')) => {
                    self.pos += 2;
                    depth += 1;
                }
                (Some(_), _) => self.pos += 1,
                (None, _) => {
                    self.ctx.diags.push(
                        Diagnostic::error(
                            self.span(start, self.pos),
                            ErrorKind::UnterminatedComment,
                            "unterminated block comment",
                        )
                        .with_label(self.span(start, start + 2), "comment starts here"),
                    );
                    return;
                }
            }
        }
    }

    fn next_token(&mut self) -> Token {
        self.skip_chars();
        let start = self.pos;

        let Some(b) = self.peek() else {
            return self.token(TokenKind::Eof, start);
        };

        let kind = match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => return self.consume_word(),
            b'0'..=b'9' => return self.consume_number(),
            b'\"' => return self.consume_string(),
            b'\'' => return self.consume_byte_char(),

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
            b'/' => self.single(TokenKind::Slash),

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

    fn consume_string(&mut self) -> Token {
        debug_assert_eq!(self.peek(), Some(b'"'));
        let start = self.pos;
        self.pos += 1;

        let mut buf: Vec<u8> = Vec::new();
        let mut terminated = false;
        let mut bad_escape = false;

        while let Some(b) = self.peek() {
            match b {
                b'"' => {
                    self.pos += 1;
                    terminated = true;
                    break;
                }
                b'\\' => {
                    let esc_start = self.pos;
                    self.pos += 1;
                    if self.peek().is_none() {
                        break;
                    }
                    match self.escape(esc_start) {
                        Some(v) => buf.push(v),
                        None => bad_escape = true,
                    }
                }
                _ => {
                    buf.push(b);
                    self.pos += 1;
                }
            }
        }

        if !terminated {
            self.ctx.diags.push(
                Diagnostic::error(
                    self.span(start, self.pos),
                    ErrorKind::BadTextLiteral,
                    "unterminated string literal",
                )
                .with_label(self.span(start, start + 1), "string starts here"),
            );
            return self.token(TokenKind::Error, start);
        }
        if bad_escape {
            return self.token(TokenKind::Error, start);
        }

        let text = String::from_utf8(buf).expect("source is UTF-8 and escapes are ASCII");
        let id = self.ctx.literals.intern_str(&text);
        self.token(TokenKind::Str(id), start)
    }

    fn consume_byte_char(&mut self) -> Token {
        debug_assert_eq!(self.peek(), Some(b'\''));
        let start = self.pos;
        self.pos += 1;

        let value: Option<u8> = match self.peek() {
            None | Some(b'\n') => return self.unterminated_byte_literal(start),
            Some(b'\'') => {
                self.pos += 1;
                self.text_error(start, "empty byte literal");
                return self.token(TokenKind::Error, start);
            }
            Some(b'\\') => {
                let esc_start = self.pos;
                self.pos += 1;
                if self.peek().is_none() {
                    return self.unterminated_byte_literal(start);
                }
                self.escape(esc_start)
            }
            Some(b) if !b.is_ascii() => {
                let ch_start = self.pos;
                self.pos += 1;
                while matches!(self.peek(), Some(b) if is_continuation(b)) {
                    self.pos += 1;
                }
                self.text_error(ch_start, "byte literals must be ASCII");
                None
            }
            Some(b) => {
                self.pos += 1;
                Some(b)
            }
        };

        let mut errored = value.is_none();

        if self.peek() == Some(b'\'') {
            self.pos += 1;
        } else {
            let extra = self.pos;
            self.consume_while(|b| b != b'\'' && b != b'\n');
            if self.peek() == Some(b'\'') {
                self.pos += 1;
                if !errored {
                    self.text_error(extra, "byte literal holds more than one byte");
                }
            } else if !errored {
                return self.unterminated_byte_literal(start);
            }
            errored = true;
        }

        match value {
            Some(v) if !errored => self.token(TokenKind::Char(v), start),
            _ => self.token(TokenKind::Error, start),
        }
    }

    fn unterminated_byte_literal(&mut self, start: usize) -> Token {
        let span = self.span(start, self.pos);
        let label = self.span(start, start + 1);
        self.ctx.diags.push(
            Diagnostic::error(span, ErrorKind::BadTextLiteral, "unterminated byte literal")
                .with_label(label, "literal starts here"),
        );
        self.token(TokenKind::Error, start)
    }

    fn escape(&mut self, esc_start: usize) -> Option<u8> {
        let b = self.peek()?;
        let val = match b {
            b'n' => Some(b'\n'),
            b't' => Some(b'\t'),
            b'r' => Some(b'\r'),
            b'0' => Some(0),
            b'\\' => Some(b'\\'),
            b'\'' => Some(b'\''),
            b'"' => Some(b'"'),
            _ => None,
        };
        self.pos += 1;
        if val.is_none() {
            while matches!(self.peek(), Some(b) if is_continuation(b)) {
                self.pos += 1; // whole character, stay on a boundary
            }
            let text = String::from_utf8_lossy(&self.src[esc_start..self.pos]);
            self.text_error(esc_start, format!("unknown escape sequence `{text}`"));
        }
        val
    }

    fn text_error(&mut self, start: usize, msg: impl Into<String>) {
        self.ctx.diags.push(Diagnostic::error(
            self.span(start, self.pos),
            ErrorKind::BadTextLiteral,
            msg,
        ));
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

        let mut float = radix == 10
            && self.peek() == Some(b'.')
            && matches!(self.peek_at(1), Some(b) if b.is_ascii_digit());
        if float {
            self.pos += 1;
            self.consume_while(number_legal10);
        }

        if radix == 10 && self.at_exponent() {
            float = true;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            self.consume_while(number_legal10);
        }

        let digits_end = self.pos;

        if let Some(b) = self.peek()
            && b.is_ascii_digit()
        {
            let msg = format!("invalid digit `{}` in base-{} literal", b as char, radix);
            self.consume_while(ident_legal);
            return self.bad_number(start, msg);
        }

        let suffix_start = self.pos;
        self.consume_while(ident_legal);
        let suffix_end = self.pos;

        let digits = str::from_utf8(&self.src[digits_start..digits_end])
            .expect("illegal characters will not be consumed");
        let suffix_s = str::from_utf8(&self.src[suffix_start..suffix_end])
            .expect("illegal characters will not be consumed");

        let cleaned = digits.chars().filter(|&c| c != '_').collect::<String>();

        if cleaned.is_empty() {
            return self.bad_number(start, format!("Missing digits after base-{} prefix", radix));
        }

        let suffix = Lexer::parse_suffix(suffix_s);
        if suffix.is_none() && !suffix_s.is_empty() {
            self.bad_number(
                start,
                format!("unknown or invalid literal suffix {}", suffix_s),
            );

            return self.token(TokenKind::Error, start);
        }

        if float {
            let n = match cleaned.parse::<f64>() {
                Ok(n) if n.is_infinite() => {
                    return self.bad_number(start, "float literal out of range");
                }
                Ok(n) => n,
                Err(_) => return self.bad_number(start, "invalid float literal"),
            };

            self.token(TokenKind::Float(n.to_bits(), suffix), start)
        } else {
            let n = match u64::from_str_radix(&cleaned, radix) {
                Ok(n) => n,
                Err(e) => {
                    let msg = if let IntErrorKind::PosOverflow = e.kind() {
                        "int literal too large".to_string()
                    } else {
                        format!("invalid int literal {}", cleaned)
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
            self.span(start, self.pos),
            ErrorKind::IllegalChar,
            format!("unexpected char `{text}`"),
        ));

        self.token(TokenKind::Error, start)
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.base + start as u32, self.base + end as u32)
    }

    fn token(&self, kind: TokenKind, start: usize) -> Token {
        Token {
            span: self.span(start, self.pos),
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
            "u1" => Suffix::U1,
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

    fn bad_number(&mut self, start: usize, msg: impl Into<String>) -> Token {
        self.ctx.diags.push(Diagnostic::error(
            self.span(start, self.pos),
            ErrorKind::BadNumber,
            msg.into().to_string(),
        ));

        self.token(TokenKind::Error, start)
    }

    fn at_exponent(&self) -> bool {
        if !matches!(self.peek(), Some(b'e' | b'E')) {
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

fn number_legal10(b: u8) -> bool {
    number_legal(b, 10)
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.kind)
    }
}
