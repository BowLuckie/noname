use TokenKind::*;
use uname::frontend::{Ctx, Severity, Suffix, TokenKind, lex};

fn run(src: &str) -> (Vec<TokenKind>, Ctx) {
    let mut ctx = Ctx::default();
    let mut ks: Vec<_> = lex(src, &mut ctx).into_iter().map(|t| t.kind).collect();
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
    let mut ctx = Ctx::default();
    let toks = lex(src, &mut ctx);
    let mut prev = 0;
    for t in &toks {
        assert!(prev <= t.span.start && t.span.start <= t.span.end);
        assert!(t.span.end as usize <= src.len());
        prev = t.span.end;
    }
    let eof = toks.last().unwrap();
    assert_eq!(
        (eof.span.start, eof.span.end),
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

#[test]
fn illegal_chars_and_recovery() {
    bad("@", 1);
    bad("@ # $", 3);

    let (ks, ctx) = run("é\n");
    assert_eq!(ks, vec![Error]);
    assert_eq!(errors(&ctx), 1);
    let d = &ctx.diags[0];
    assert_eq!(d.span.end - d.span.start, 2);
    assert!(!d.message.contains('\n'));

    let (ks, ctx) = run("foo@bar");
    assert_eq!(errors(&ctx), 1);
    assert!(matches!(ks.as_slice(), [Ident(_), Error, Ident(_)]));
}
