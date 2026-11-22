#![allow(dead_code, unused)]

use anyhow::{Context, Result};
use std::path::Path;

pub(crate) type Btye = u8;

pub mod frontend;
pub mod utils;

pub struct CompilerOptions {
    pub emit_tokens: bool,
    pub emit_ast: bool,
    pub color: bool,
}
