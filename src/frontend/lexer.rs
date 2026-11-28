use crate::{
    frontend::{
        Ctx, Diagnostic, ErrorKind, FileId, SourceMap, Span, Spanned, StrId, Sym,
        common::{FPart, FStrId, FString},
    },
    utils::Color,
};
use std::{
    char,
    fmt::{self, Debug, Display, Formatter},
    mem,
    num::IntErrorKind,
    str,
};
use strum::IntoStaticStr;

struct Lexer<'src> {
    src: &'src [u8],
    ctx: &'src mut Ctx,
    base: u32,
    pos: usize,
}

pub type Token = Spanned<TokenKind>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::IntoStaticStr, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, IntoStaticStr)]
pub enum TokenKind {
    Ident(Sym),
    Int(u64, Option<Suffix>),
    Float(u64, Option<Suffix>),
    Str(StrId),
    FStr(FStrId),
    Char(u8),
    Rune(u32),

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
    Carrot, // ^
    Hash,   // #

    Error,
    Eof,
}

#[derive(Clone, Copy)]
enum Quote {
    Str,
    Char,
}

impl Quote {
    fn delim(self) -> u8 {
        match self {
            Quote::Str => b'"',
            Quote::Char => b'\'',
        }
    }
    fn what(self) -> &'static str {
        match self {
            Quote::Str => "string literal",
            Quote::Char => "character literal",
        }
    }
    fn control_permitted(self) -> bool {
        matches!(self, Quote::Str)
    }
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
            b'\"' => return self.consume_string(false),
            b'\'' => return self.consume_char(None),

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
            b'#' => self.single(TokenKind::Hash),
            b'^' => self.single(TokenKind::Carrot),

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

        if ["u", "b"].contains(&word)
            && let Some(b'\'') = self.peek()
        {
            return self.consume_char(Some(*word.as_bytes().first().unwrap()));
        } else if word == "f"
            && let Some(b'"') = self.peek()
        {
            return self.consume_string(true);
        }

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

    fn consume_string(&mut self, _is_format: bool) -> Token {
        debug_assert_eq!(self.peek(), Some(b'"'));
        let start = self.pos;
        if _is_format {
            match self.process_format() {
                Ok(fs) => {
                    let str_id = self.ctx.literals.add_fstr(fs);
                    self.token(TokenKind::FStr(str_id), start)
                }
                Err(e) => e,
            }
        } else {
            match self.process_string(Quote::Str) {
                Ok(s) => {
                    let str_id = self.ctx.literals.intern_str(&s);
                    self.token(TokenKind::Str(str_id), start)
                }
                Err(e) => e,
            }
        }
    }

    fn process_string(&mut self, quote: Quote) -> Result<String, Token> {
        let what = quote.what();
        let start = self.pos;
        self.pos += 1;

        let mut buf: Vec<u8> = Vec::new();
        let mut terminated = false;
        let mut bad = false;

        while let Some(b) = self.peek() {
            match b {
                c if c == quote.delim() => {
                    self.pos += 1;
                    terminated = true;
                    break;
                }
                b'\n' if !quote.control_permitted() => {
                    break;
                }
                c if !quote.control_permitted() && c.is_ascii_control() => {
                    self.text_error_with_note(
                        self.pos,
                        "char literals cannot have control characters in them",
                        format!("try writing the escape code '{}' instead", c.escape_ascii()),
                    );
                    self.pos += 1;
                    bad = true;
                }
                // b'\r' => {
                //
                // }
                b'\\' => {
                    self.process_escape(&mut buf, &mut bad);
                }
                _ => {
                    let ch = std::str::from_utf8(&self.src[self.pos..])
                        .expect("source must be valid UTF-8")
                        .chars()
                        .next()
                        .unwrap();

                    let len = ch.len_utf8();

                    buf.extend_from_slice(&self.src[self.pos..self.pos + len]);
                    self.pos += len;
                }
            }
        }

        if !terminated {
            self.ctx.diags.push(
                Diagnostic::error(
                    self.span(start, self.pos),
                    ErrorKind::BadTextLiteral,
                    format!("unterminated {}", what),
                )
                .with_label(self.span(start, start + 1), format!("{} starts here", what)),
            );
            return Err(self.token(TokenKind::Error, start));
        }
        if bad {
            return Err(self.token(TokenKind::Error, start));
        }

        Ok(String::from_utf8(buf).expect("source is utf8"))
    }

    fn process_escape(&mut self, buf: &mut Vec<u8>, bad: &mut bool) {
        let esc_start = self.pos;
        self.pos += 1;
        if self.peek().is_none() {
            return;
        }

        match self.escape(esc_start) {
            Some(v) => buf.extend_from_slice(
                char::from_u32(v)
                    .unwrap()
                    .encode_utf8(&mut [0; 4])
                    .as_bytes(),
            ),
            None => *bad = true,
        }
    }

    fn consume_char(&mut self, prefix: Option<u8>) -> Token {
        #[derive(PartialEq, Eq)]
        enum Prefix {
            Char,
            Rune,
            Either,
        }

        debug_assert_eq!(self.peek(), Some(b'\''));
        let prefix = if prefix == Some(b'u') {
            Prefix::Rune
        } else if prefix == Some(b'b') {
            Prefix::Char
        } else if prefix.is_none() {
            Prefix::Either
        } else {
            panic!("compiler panic: illegal prefix");
        };

        let opening_quote = self.pos;
        let data_start = self.pos + 1;

        if let Some(b'\n') = self.peek() {
            return self.unterminated_byte_literal(opening_quote);
        }

        let data = match self.process_string(Quote::Char) {
            Ok(s) => s,
            Err(e) => return e,
        };

        let (chars, bytes) = (data.chars().count(), data.len());

        if bytes == 0 {
            return self.text_error(data_start, "character literal must not be empty");
        }

        if chars != 1 {
            return self.text_error_with_note(
                data_start,
                "character literal can only hold one character",
                "consider using a string literal using \"\"",
            );
        }

        let ch = data.chars().next().unwrap();
        match (prefix, ch.is_ascii()) {
            (Prefix::Char, false) => {
                self.text_error(opening_quote, "character literal must be ascii")
            }
            (Prefix::Char, true) | (Prefix::Either, true) => {
                self.token(TokenKind::Char(ch as u8), opening_quote)
            }
            (Prefix::Rune, _) | (Prefix::Either, false) => {
                self.token(TokenKind::Rune(ch as u32), opening_quote)
            }
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

    fn escape(&mut self, esc_start: usize) -> Option<u32> {
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
        val.map(|inner_b| inner_b as u32)
    }

    fn text_error(&mut self, start: usize, msg: impl Into<String>) -> Token {
        let d = Diagnostic::error(self.span(start, self.pos), ErrorKind::BadTextLiteral, msg);
        self.ctx.diags.push(d);
        self.error_token(start)
    }

    fn text_error_with_note(
        &mut self,
        start: usize,
        msg: impl Into<String>,
        note: impl Into<String>,
    ) -> Token {
        let d = Diagnostic::error(self.span(start, self.pos), ErrorKind::BadTextLiteral, msg)
            .with_note(note);
        self.ctx.diags.push(d);
        self.error_token(start)
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

    fn error_token(&self, start: usize) -> Token {
        self.token(TokenKind::Error, start)
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

    fn process_format(&mut self) -> Result<FString, Token> {
        debug_assert_eq!(self.peek(), Some(b'"'));
        let start = self.pos;
        self.pos += 1;

        let mut parts: Vec<FPart> = Vec::new();
        let mut text_buf: Vec<u8> = Vec::new();
        let (mut terminated, mut bad) = (false, false);

        while let Some(b) = self.peek() {
            match b {
                b'"' => {
                    self.pos += 1;
                    terminated = true;
                    break;
                }
                b'\\' => {
                    self.process_escape(&mut text_buf, &mut bad);
                }
                b'{' if self.peek_at(1) == Some(b'{') => {
                    text_buf.push(b'{');
                    self.pos += 2;
                }
                b'}' if self.peek_at(1) == Some(b'}') => {
                    text_buf.push(b'}');
                    self.pos += 2;
                }
                b'{' => {
                    self.flush_format_parts(&mut text_buf, &mut parts);
                    let open = self.pos;
                    self.pos += 1;
                    let li = self.lex_interpolation(open);
                    match li {
                        Some(tokens) => parts.push(FPart::Expr(tokens)),
                        None => bad = true,
                    }
                }
                _ => {
                    text_buf.push(b);
                    self.pos += 1;
                }
            }
        }

        if !terminated {
            self.ctx.diags.push(
                Diagnostic::error(
                    self.span(start, self.pos),
                    ErrorKind::BadTextLiteral,
                    "unterminated format string",
                )
                .with_label(self.span(start, start + 1), "format string starts here"),
            );
            return Err(self.token(TokenKind::Error, start));
        }
        if bad {
            return Err(self.token(TokenKind::Error, start));
        }

        self.flush_format_parts(&mut text_buf, &mut parts);
        Ok(parts.into())
    }

    fn flush_format_parts(&mut self, text: &mut Vec<u8>, parts: &mut Vec<FPart>) {
        if text.is_empty() {
            return;
        }
        let s = String::from_utf8(mem::take(text)).expect("source is utf8");
        let id = self.ctx.literals.intern_str(&s);
        parts.push(FPart::Text(id));
    }

    fn lex_interpolation(&mut self, open: usize) -> Option<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut depth = 0;
        loop {
            let t = self.next_token();
            match t.kind {
                TokenKind::Eof => {
                    self.text_error(open, "unterminated `{` in format string");
                    return None;
                }
                TokenKind::RBrace if depth == 0 => break,
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => depth -= 1,
                _ => {}
            }
            tokens.push(t);
        }
        if tokens.is_empty() {
            self.text_error_with_note(
                open,
                "empty `{}` in format string, an expression is needed",
                "if you want an actual brace pair, use `{{}}` instead",
            );
            return None;
        }

        Some(tokens)
    }
}

pub struct TokenRenderer<'ctx> {
    tokens: &'ctx Vec<Token>,
    sources: &'ctx SourceMap,
    ctx: &'ctx Ctx,
    color: bool,
}

impl TokenRenderer<'_> {
    fn paint(&self, c: Color, s: String) -> String {
        if self.color {
            format!("{c}{s}{}", Color::Reset)
        } else {
            s
        }
    }

    fn value(&self, k: &TokenKind) -> String {
        match *k {
            TokenKind::Char(b) => format!("'{}'", (b as char).escape_debug()),
            TokenKind::Rune(u) => format!("'{}'", (char::from_u32(u).unwrap()).escape_debug()),
            TokenKind::Int(v, s) => format!("{v}{}", s.map_or("", Suffix::into)),
            TokenKind::Float(bits, s) => {
                format!("{:?}{}", f64::from_bits(bits), s.map_or("", Suffix::into))
            }
            TokenKind::Str(id) => {
                format!("\"{}\"", self.ctx.literals.resolve_str(id).escape_debug())
            }
            TokenKind::FStr(_id) => {
                "format string".to_string() // TODO: Format String
                // Rendering
            }
            TokenKind::Ident(sym) => self.ctx.interner.resolve(sym).to_string(),
            _ => String::new(),
        }
    }
}

impl TokenKind {
    fn color(&self) -> Color {
        match self {
            TokenKind::Int(..)
            | TokenKind::Float(..)
            | TokenKind::Char(..)
            | TokenKind::Str(..)
            | TokenKind::FStr(..)
            | TokenKind::Rune(..) => Color::Yellow,
            TokenKind::Ident(..) => Color::Cyan,
            TokenKind::Fn | TokenKind::Include | TokenKind::Intrinsic | TokenKind::Let => {
                Color::Magenta
            }
            TokenKind::Error => Color::Red,
            TokenKind::Eof => Color::Dim,
            _ => Color::White,
        }
    }
}

impl Display for TokenRenderer<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut last_base = None;

        for t in self.tokens {
            let (file, local) = self.sources.lookup(t.span.start());
            if last_base != Some(file.base) {
                let header = format!("-- {}", file.name.display());
                writeln!(f, "{}", self.paint(Color::Dim, header))?;
                last_base = Some(file.base);
            }

            let (line, col) = file.line_col(local);
            let lexeme = &file.text[local as usize..(t.span.end() - file.base) as usize];
            let name: &'static str = (&t.kind).into();
            let c = t.kind.color();

            writeln!(
                f,
                "  {}{}{}{}",
                self.paint(
                    Color::Dim,
                    format!("{:<7}", format!("{}:{}", line + 1, col + 1))
                ),
                self.paint(c, format!("{name:<7}")),
                self.paint(c, format!("{:<16}", self.value(&t.kind))),
                self.paint(Color::Dim, lexeme.escape_debug().to_string()),
            )?;
        }
        Ok(())
    }
}

impl<'a> TokenRenderer<'a> {
    pub fn new(tokens: &'a Vec<Token>, sources: &'a SourceMap, ctx: &'a Ctx, color: bool) -> Self {
        TokenRenderer {
            tokens,
            sources,
            color,
            ctx,
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
