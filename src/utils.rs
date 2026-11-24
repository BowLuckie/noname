use std::fmt::{self, Formatter};

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Reset = 0,
    Bold = 1,
    Dim = 2,

    Black = 30,
    Red = 31,
    Green = 32,
    Yellow = 33,
    Blue = 34,
    Magenta = 35,
    Cyan = 36,
    White = 37,

    BrightBlack = 90,
    BrightRed = 91,
    BrightGreen = 92,
    BrightYellow = 93,
    BrightBlue = 94,
    BrightMagenta = 95,
    BrightCyan = 96,
    BrightWhite = 97,
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "\x1b[{}m", *self as u8)
    }
}

impl fmt::Debug for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "\x1b[{}m", *self as u8)
    }
}
