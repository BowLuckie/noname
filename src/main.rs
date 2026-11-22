use std::process::ExitCode;

use crate::args::Cli;
use clap::Parser;
use uname::{CompilerOptions, frontend::Frontend, utils::Color};

mod args;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mut frontend = Frontend::new();
    frontend.set_root(cli.file.display().to_string());
    let options = CompilerOptions::from(&cli);

    match frontend.run(&options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let color = cli.color.should_color(&std::io::stderr());
            eprint!("{}", frontend.render_diagnostics(color));

            let (hl, reset) = if color {
                (Color::BrightRed.to_string(), Color::Reset.to_string())
            } else {
                Default::default()
            };
            eprintln!("{hl}error{reset}: {e}");
            ExitCode::FAILURE
        }
    }
}
