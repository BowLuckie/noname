use anyhow::{Context, Result};
use farm::frontend::Frontend;
use std::path::Path;

fn read_file(p: &str) -> Result<Box<str>> {
    let path = Path::new(p);
    let root_path = &Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let s = std::fs::read_to_string(root_path)
        .with_context(|| format!("Failed to read file {}", root_path.display()))?;

    Ok(s.into_boxed_str())
}

fn main() -> Result<()> {
    let mut frontend = Frontend::new();
    let source = read_file("examples/main.farm")?;
    frontend.assign_source(source);

    frontend.run()
}
