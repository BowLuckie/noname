use anyhow::{Context, Ok, Result};
use uname::frontend::Frontend;

fn main() -> Result<()> {
    let path = std::env::args().nth(1).context("usage: uname <file>")?;

    let mut frontend = Frontend::new();
    frontend.set_root(path);

    if let Err(e) = frontend.run() {
        eprint!("{}", frontend.render_diagnostics(true));
        return Err(e);
    }
    Ok(())
}
