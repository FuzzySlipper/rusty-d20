use std::env;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut check = false;
    let mut output = None;
    for argument in env::args().skip(1) {
        if argument == "--check" {
            check = true;
        } else if output.replace(PathBuf::from(argument)).is_some() {
            return Err("only one output path may be supplied".into());
        }
    }
    let output = output.ok_or("usage: rusty-d20-protocol [--check] <output-path>")?;
    let generated = rusty_d20::generated_typescript();

    if check {
        let committed = fs::read_to_string(&output)?;
        if committed != generated {
            return Err(format!(
                "generated protocol is stale; run `pnpm run protocol:generate` ({})",
                output.display()
            )
            .into());
        }
    } else {
        fs::write(&output, generated)?;
    }
    Ok(())
}
