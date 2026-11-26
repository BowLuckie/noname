use TokenKind::*;
use uname::frontend::{Ctx, Frontend, Severity, Suffix, Token, TokenKind};

fn lex(src: &str) -> (Vec<Token>, Ctx) {
    let mut frontend = Frontend::new();
    let file = frontend.add_file("test.un", src);
    let toks = frontend.lex(file);
    let ctx = frontend.ctx().clone();
    (toks, ctx)
}

fn run(src: &str) -> (Vec<TokenKind>, Ctx) {
    let (toks, ctx) = lex(src);
    let mut ks: Vec<_> = toks.into_iter().map(|t| t.kind).collect();
    assert_eq!(ks.pop(), Some(Eof), "stream must end with Eof");
    (ks, ctx)
}

fn errors(ctx: &Ctx) -> usize {
    ctx.diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count()
}

fn ok(src: &str, expected: &[TokenKind]) {
    let (ks, ctx) = run(src);
    assert_eq!(ks, expected, "{src:?}");
    assert_eq!(errors(&ctx), 0, "{src:?}: {:?}", ctx.diags);
}

fn bad(src: &str, n: usize) {
    let (ks, ctx) = run(src);
    assert_eq!(errors(&ctx), n, "{src:?}: {:?}", ctx.diags);
    assert!(ks.contains(&Error), "{src:?} should contain an Error token");
}

fn f(x: f64) -> TokenKind {
    Float(x.to_bits(), None)
}

fn i(x: u64) -> TokenKind {
    Int(x, None)
}

#[test]
fn tokens_and_keywords() {
    ok("", &[]);
    ok(" \t\r\n ", &[]);
    ok("fn include intrinsic let", &[Fn, Include, Intrinsic, Let]);
    ok(
        "( ) { } [ ] , ; : . + - * % _",
        &[
            LParen, RParen, LBrace, RBrace, LBracket, RBracket, Comma, Semi, Colon, Dot, Plus,
            Minus, Star, Percent, Underscore,
        ],
    );
    ok(
        "== != <= >= -> ::",
        &[EqEq, BangEq, LtEq, GtEq, Arrow, ColonColon],
    );
    ok("= ! < > - :", &[Eq, Bang, Lt, Gt, Minus, Colon]);
    ok("===", &[EqEq, Eq]);
    assert!(!run("a - > b").0.contains(&Arrow));
}

#[test]
fn identifiers() {
    let (ks, _) = run("foo bar foo fns _x __ a1");
    assert_eq!(ks[0], ks[2], "same name, same Sym");
    assert_ne!(ks[0], ks[1]);
    assert!(ks.iter().all(|k| matches!(k, Ident(_))));
}

#[test]
fn spans() {
    let src = "fn main() { let x = 1.5e3 + 0xFFu8; }";
    let (toks, _) = lex(src);
    let mut prev = 0;
    for t in &toks {
        assert!(prev <= t.span.start() && t.span.start() <= t.span.end());
        assert!(t.span.end() as usize <= src.len());
        prev = t.span.end();
    }
    let eof = toks.last().unwrap();
    assert_eq!(
        (eof.span.start(), eof.span.end()),
        (src.len() as u32, src.len() as u32)
    );
}

#[test]
fn integers() {
    ok("0 42 1_000 1__0", &[i(0), i(42), i(1000), i(10)]);
    ok("0xFF 0xff 0b1010 0o17", &[i(255), i(255), i(10), i(15)]);
    ok("18446744073709551615", &[i(u64::MAX)]);
    ok("10u8", &[Int(10, Some(Suffix::U8))]);
    ok("0xFFu8", &[Int(255, Some(Suffix::U8))]);
    ok("1_0i32", &[Int(10, Some(Suffix::I32))]);
    ok("0x1f32", &[i(0x1f32)]);
    ok("-5", &[Minus, i(5)]);
}

#[test]
fn integer_errors() {
    bad("18446744073709551616", 1);
    bad("10q", 1);
    bad("12abc", 1);
    for src in ["0x", "0b", "0o", "0x_"] {
        bad(src, 1);
    }
    bad("0b102", 1);
    bad("0o789", 1);
}

#[test]
fn floats() {
    ok("1.5 0.25 1_0.5_0", &[f(1.5), f(0.25), f(10.5)]);
    ok("1.1", &[Float(4607632778762754458, None)]);
    ok("1.5f32", &[Float(1.5f64.to_bits(), Some(Suffix::F32))]);
}

#[test]
fn dot_only_makes_a_float_when_a_digit_follows() {
    ok("1.foo", &[i(1), Dot, run("foo").0[0]]);
    ok("1..5", &[i(1), Dot, Dot, i(5)]);
    ok("1.", &[i(1), Dot]);
    ok(".5", &[Dot, i(5)]);
    ok("1.2.3", &[f(1.2), Dot, i(3)]);
    ok("0x1.5", &[i(1), Dot, i(5)]);
}

#[test]
fn exponents() {
    ok(
        "1e5 1E5 1.5e3 2e-4 6.02e+23 1_0e1_0",
        &[f(1e5), f(1e5), f(1500.0), f(2e-4), f(6.02e23), f(10e10)],
    );
    ok("1e5f32", &[Float(1e5f64.to_bits(), Some(Suffix::F32))]);
    ok("0x1e5", &[i(0x1e5)]);
    for src in ["1e", "1e+", "1e-", "1else"] {
        bad(src, 1);
    }
    ok("1.e5", &[i(1), Dot, run("e5").0[0]]);
    bad("1e999", 1);
    ok("1e-999", &[f(0.0)]);
}

fn lit(src: &str) -> String {
    let (ks, ctx) = run(src);
    assert_eq!(errors(&ctx), 0, "{src:?}: {:?}", ctx.diags);
    match ks.as_slice() {
        [Str(id)] => ctx.literals.resolve_str(*id).to_string(),
        other => panic!("{src:?}: expected one Str, got {other:?}"),
    }
}

#[test]
fn line_comments() {
    ok("// x\nfn", &[Fn]);
    ok("fn // x", &[Fn]);
    ok("//", &[]);
    ok("fn//x\nlet", &[Fn, Let]);
    ok("// a\n// b\n\n// c\nfn", &[Fn]);
    ok("// /* not a block\nfn", &[Fn]);
}

#[test]
fn block_comments() {
    ok("/* x */ fn", &[Fn]);
    ok("fn/*x*/let", &[Fn, Let]);
    ok("/**/", &[]);
    ok("/***/ fn", &[Fn]);
    ok("/* a\nb\nc */ fn", &[Fn]);
    ok("/* // not a line comment */ fn", &[Fn]);
    ok("/* * / */ fn", &[Fn]);
    ok("/* a */ /* b */ // c\n /* d */ fn", &[Fn]);
}

#[test]
fn nested_block_comments() {
    ok("/* /* */ */ fn", &[Fn]);
    ok("/* /* /* */ */ */ fn", &[Fn]);
}

#[test]
fn unterminated_block_comments() {
    for src in ["/* never closed", "/*", "/*/", "fn /* x", "/* a\nb\nc"] {
        let (_, ctx) = run(src);
        assert_eq!(errors(&ctx), 1, "{src:?}: {:?}", ctx.diags);
    }
    let (ks, _) = run("fn /* x");
    assert_eq!(ks, vec![Fn]);
    let (_, ctx) = run("/* /* */ fn");
    assert_eq!(errors(&ctx), 1);
}

#[test]
fn slash_is_division_when_not_a_comment() {
    ok("/", &[Slash]);
    ok("1 / 2", &[Int(1, None), Slash, Int(2, None)]);
    ok("1/2", &[Int(1, None), Slash, Int(2, None)]);
    ok("1//2", &[Int(1, None)]);
    ok("1/ /2", &[Int(1, None), Slash, Slash, Int(2, None)]);
}

#[test]
fn string_contents() {
    assert_eq!(lit(r#""""#), "");
    assert_eq!(lit(r#""abc""#), "abc");
    assert_eq!(lit(r#""a b  c""#), "a b  c");
    assert_eq!(lit(r#""é""#), "é");
    assert_eq!(lit(r#""héllo wörld""#), "héllo wörld");
}

#[test]
fn string_escapes_are_processed() {
    assert_eq!(lit(r#""\n""#), "\n");
    assert_eq!(lit(r#""\t""#), "\t");
    assert_eq!(lit(r#""a\nb\tc""#), "a\nb\tc");
    assert_eq!(lit(r#""\n\n""#).len(), 2);
}

#[test]
fn string_contents_are_not_lexed() {
    assert_eq!(lit(r#""// not a comment""#), "// not a comment");
    assert_eq!(lit(r#""/* nor this */""#), "/* nor this */");
    assert_eq!(lit(r#""fn (x) -> 1 + @""#), "fn (x) -> 1 + @");
}

#[test]
fn string_at_eof_without_trailing_newline_is_fine() {
    assert_eq!(lit(r#""abc""#), "abc");
    ok(r#"fn "abc""#, &[Fn, run(r#""abc""#).0[0]]);
}

#[test]
fn string_span_covers_quotes_and_escapes_but_table_holds_processed_bytes() {
    let src = r#""\n""#;
    let (toks, ctx) = lex(src);
    assert_eq!((toks[0].span.start(), toks[0].span.end()), (0, 4));
    let Str(id) = toks[0].kind else {
        panic!("expected Str")
    };
    assert_eq!(ctx.literals.resolve_str(id).len(), 1);
}

#[test]
fn identical_literals_share_an_id() {
    let (ks, _) = run(r#""a" "a" "b""#);
    assert_eq!(ks[0], ks[1]);
    assert_ne!(ks[0], ks[2]);
}

#[test]
fn strings_between_other_tokens() {
    let (ks, ctx) = run(r#"f("x","y")"#);
    assert_eq!(errors(&ctx), 0);
    assert_eq!(ks.len(), 6);
    assert_eq!((ks[1], ks[3], ks[5]), (LParen, Comma, RParen));
    assert!(matches!(ks[2], Str(_)) && matches!(ks[4], Str(_)));

    let (ks, _) = run(r#""a""b""#);
    assert!(matches!(ks.as_slice(), [Str(_), Str(_)]));
}

#[test]
fn comment_after_string_and_string_after_comment() {
    let (ks, ctx) = run("\"a\" // \"not a string\n\"b\"");
    assert_eq!(errors(&ctx), 0);
    assert!(matches!(ks.as_slice(), [Str(_), Str(_)]));
}

#[test]
fn unterminated_strings() {
    for src in [r#""abc"#, r#"""#, r#""abc\"#] {
        bad(src, 1);
    }
    let (ks, ctx) = run(r#"fn "abc"#);
    assert_eq!(ks, vec![Fn, Error]);
    assert_eq!(
        ctx.diags[0].labels.len(),
        1,
        "should label the opening quote"
    );
}

#[test]
fn unknown_escape() {
    bad(r#""\q""#, 1);

    let (ks, ctx) = run(r#""\q" fn"#);
    assert_eq!(errors(&ctx), 1);
    assert_eq!(ks, vec![Error, Fn]);

    let (ks, ctx) = run(r#""\é" fn"#);
    assert_eq!(errors(&ctx), 1);
    assert_eq!(ks, vec![Error, Fn]);

    let (ks, ctx) = run(r#""\q\z""#);
    assert_eq!(errors(&ctx), 2);
    assert_eq!(ks, vec![Error]);
}

fn recovers(src: &str, n: usize, expected: &[TokenKind]) {
    let (ks, ctx) = run(src);
    assert_eq!(errors(&ctx), n, "{src:?}: {:?}", ctx.diags);
    assert_eq!(ks, expected, "{src:?}");
}

#[test]
fn chars() {
    ok("'a'", &[Char(b'a')]); // at EOF, no trailing newline
    ok(
        "'Z' '0' ' ' '_'",
        &[Char(b'Z'), Char(b'0'), Char(b' '), Char(b'_')],
    );
    ok("'a' 'b'", &[Char(b'a'), Char(b'b')]);
    ok("'a''b'", &[Char(b'a'), Char(b'b')]);
    ok("'\"'", &[Char(b'"')]); // raw double quote needs no escape
    ok("'/' '*'", &[Char(b'/'), Char(b'*')]); // looks like comment start, isn't
    ok("f('a')", &[run("f").0[0], LParen, Char(b'a'), RParen]);
    ok("'a'+'b'", &[Char(b'a'), Plus, Char(b'b')]);
}

#[test]
fn char_escapes() {
    ok(r"'\n'", &[Char(10)]);
    ok(r"'\t'", &[Char(9)]);
    ok(r"'\r'", &[Char(13)]);
    ok(r"'\0'", &[Char(0)]);
    ok(r"'\\'", &[Char(b'\\')]);
    ok(r"'\''", &[Char(b'\'')]);
    ok(r#"'\"'"#, &[Char(b'"')]);
}

#[test]
fn char_span_covers_quotes_and_escape() {
    let (toks, _) = lex(r"'\n'"); // 4 source bytes, one byte of value
    assert_eq!((toks[0].span.start(), toks[0].span.end()), (0, 4));
    assert_eq!(toks[0].kind, Char(10));
}

#[test]
fn chars_are_inert_inside_strings_and_comments() {
    assert_eq!(lit(r#""'a'""#), "'a'");
    assert_eq!(lit(r#""it's""#), "it's");
    ok("// 'unterminated\nfn", &[Fn]);
    ok("/* ' */ fn", &[Fn]);
}

// ---------- char errors ----------

#[test]
fn empty_char() {
    bad("''", 1);
    recovers("'' fn", 1, &[Error, Fn]);
}

#[test]
fn too_many_bytes_is_one_error_and_recovers() {
    bad("'ab'", 1);
    bad("'abcdef'", 1);
    recovers("'ab' fn", 1, &[Error, Fn]);
    recovers(r"'a\n' fn", 1, &[Error, Fn]);
}

#[test]
fn bad_escape_in_char_is_one_error() {
    bad(r"'\q'", 1);
    recovers(r"'\q' fn", 1, &[Error, Fn]);
    recovers(r"'\é' fn", 1, &[Error, Fn]); // whole character consumed
}

#[test]
fn unterminated_chars() {
    for src in ["'", "'a", "'abc", r"'\", r"'\'"] {
        bad(src, 1); // includes backslash at EOF and an escaped closing quote
    }
    // a raw newline ends the literal; the next line lexes normally
    recovers("'\nfn", 1, &[Error, Fn]);
    recovers("'a\nfn", 1, &[Error, Fn]);
}

// ---------- escapes shared with strings ----------

#[test]
fn string_escapes() {
    assert_eq!(lit(r#""\n\t\r""#), "\n\t\r");
    assert_eq!(lit(r#""\0""#), "\0");
    assert_eq!(lit(r#""\0""#).len(), 1);
    assert_eq!(lit(r#""a\"b""#), "a\"b");
    assert_eq!(lit(r#""\\""#), "\\");
    assert_eq!(lit(r#""\'""#), "'");
}

#[test]
fn escaped_backslash_does_not_escape_what_follows() {
    // \\ then n: two characters, not a newline
    assert_eq!(lit(r#""\\n""#), "\\n");
    assert_eq!(lit(r#""\\n""#).len(), 2);
    // \\ right before the closing quote: the quote still closes the string
    ok(r#""a\\" fn"#, &[run(r#""a\\""#).0[0], Fn]);
}

#[test]
fn escaped_quote_does_not_close_the_string() {
    assert_eq!(lit(r#""say \"hi\"""#), "say \"hi\"");
    bad(r#""abc\""#, 1); // \" consumed as an escape, then EOF: one unterminated error
    bad(r#""\""#, 1);
}

#[test]
fn escape_at_eof_is_only_unterminated() {
    bad(r#""abc\"#, 1); // not also a bad-escape error
    bad(r"'\", 1);
}

#[test]
fn bad_escapes_in_strings_still_recover() {
    recovers(r#""\q" fn"#, 1, &[Error, Fn]);
    recovers(r#""\q\z" fn"#, 2, &[Error, Fn]); // two diagnostics, one Error token
    recovers(r#""ok\nthen\q""#, 1, &[Error]);
}

#[test]
fn strings_and_chars_next_to_each_other() {
    let (ks, ctx) = run(r#"'a' "a" 'b'"#);
    assert_eq!(errors(&ctx), 0);
    assert!(matches!(ks.as_slice(), [Char(b'a'), Str(_), Char(b'b')]));
}
