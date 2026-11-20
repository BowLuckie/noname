use anyhow::{Context, Ok, Result};
use std::path::Path;
use uname::frontend::Frontend;

fn read_file(p: &str) -> Result<String> {
    let path = Path::new(p);
    let root_path = &Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let s = std::fs::read_to_string(root_path)
        .with_context(|| format!("Failed to read file {}", root_path.display()))?;

    Ok(s)
}

fn main() -> Result<()> {
    let mut frontend = Frontend::new();
    let source = read_file("examples/main.un")?;
    frontend.assign_source(source);

    frontend.run()
}
