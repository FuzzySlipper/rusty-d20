use std::env;
use std::path::PathBuf;

use rusty_d20::host;
use rusty_d20::GameRuntime;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let mut address = "127.0.0.1:4317".to_owned();
    let mut web_root = PathBuf::from("dist/apps/app/browser");
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--address" => {
                address = arguments.next().ok_or("--address requires a value")?;
            }
            "--web-root" => {
                web_root = PathBuf::from(arguments.next().ok_or("--web-root requires a value")?);
            }
            _ => return Err(format!("unknown argument: {argument}").into()),
        }
    }

    let runtime = GameRuntime::bootstrap()?;
    host::serve(&address, web_root, runtime).await
}
