use uname::frontend::{Token, TokenKind};

#[test]
fn token_size() {
    assert_eq!(std::mem::size_of::<TokenKind>(), 16);
    assert_eq!(std::mem::size_of::<Token>(), 24);
}
