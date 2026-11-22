pub mod frontend;
pub mod utils;

pub struct CompilerOptions {
    pub emit_tokens: bool,
    pub emit_ast: bool,
    pub color: bool,
}
