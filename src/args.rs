use clap::Parser;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use uname::CompilerOptions;

#[derive(Parser, Debug)]
#[command(name = "uname", version, about = "a compiler")]
pub struct Cli {
    /// file to compile
    pub file: PathBuf,

    /// debug the token stream
    #[arg(long)]
    pub emit_tokens: bool,

    /// debug the ast
    #[arg(long)]
    pub emit_ast: bool,

    /// colored diagnostics
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,
}

impl From<&Cli> for CompilerOptions {
    fn from(c: &Cli) -> Self {
        CompilerOptions {
            emit_tokens: c.emit_tokens,
            emit_ast: c.emit_ast,
            color: c.color.should_color(&io::stdout()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

impl ColorChoice {
    pub fn should_color(&self, stream: &impl IsTerminal) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Auto => {
                stream.is_terminal() && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
            }
        }
    }
}
