#[path = "support/compatibility.rs"]
mod compatibility;

use std::fs;

use compatibility::{FixtureResult, corpus_path, generate_corpus_json};

fn main() -> FixtureResult<()> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let generated = generate_corpus_json()?;
    let path = corpus_path();

    match arguments.as_slice() {
        [] => print!("{generated}"),
        [argument] if argument == "--write" => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, generated)?;
            println!("updated {}", path.display());
        }
        [argument] if argument == "--check" => {
            let checked_in = fs::read_to_string(&path)?;
            if checked_in != generated {
                return Err(format!(
                    "{} is stale; run `cargo run -p sealtask-client-crypto --example generate_compat_fixtures -- --write`",
                    path.display(),
                )
                .into());
            }
            println!("compatibility fixture corpus is current");
        }
        _ => {
            return Err("usage: generate_compat_fixtures [--write|--check]"
                .to_string()
                .into());
        }
    }

    Ok(())
}
