use std::env;
use std::path::PathBuf;

use rusty_d20::{host, RollSourceConfig};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let mut address = "127.0.0.1:4317".to_owned();
    let mut web_root = PathBuf::from("dist/apps/app/browser");
    let mut save_path = PathBuf::from("target/rusty-d20/save.json");
    let mut roll_source_path = None;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--address" => {
                address = arguments.next().ok_or("--address requires a value")?;
            }
            "--web-root" => {
                web_root = PathBuf::from(arguments.next().ok_or("--web-root requires a value")?);
            }
            "--save-file" => {
                save_path = PathBuf::from(arguments.next().ok_or("--save-file requires a value")?);
            }
            "--roll-source" => {
                roll_source_path = Some(PathBuf::from(
                    arguments.next().ok_or("--roll-source requires a value")?,
                ));
            }
            _ => return Err(format!("unknown argument: {argument}").into()),
        }
    }

    let roll_source = if let Some(path) = roll_source_path {
        serde_json::from_slice::<RollSourceConfig>(&std::fs::read(&path)?)
            .map_err(|error| format!("invalid roll source {}: {error}", path.display()))?
    } else {
        RollSourceConfig::default()
    };
    let runtime = host::load_host_runtime_with_roll_source(&save_path, roll_source)?;
    host::serve_host(&address, web_root, save_path, runtime).await
}
