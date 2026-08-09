use std::{env, fs, path::PathBuf};

use serde::Deserialize;

const ENGINE_REPOSITORY: &str = "https://github.com/FuzzySlipper/rusty-engine";

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct EngineSource {
    schema_version: u32,
    repository: String,
    branch: String,
    commit: String,
}

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
    let repository_root = manifest_dir
        .ancestors()
        .nth(3)
        .expect("Rusty D20 crate remains under rust/crates/rusty-d20");
    let source_path = repository_root.join("engine-source.json");
    println!("cargo:rerun-if-changed={}", source_path.display());

    let source: EngineSource = serde_json::from_str(
        &fs::read_to_string(&source_path).expect("read canonical engine-source.json"),
    )
    .expect("decode canonical engine-source.json");
    assert_eq!(source.schema_version, 1, "unsupported Engine source schema");
    assert_eq!(
        source.repository, ENGINE_REPOSITORY,
        "non-canonical Engine repository"
    );
    assert_eq!(source.branch, "main", "Engine dependency must track main");
    assert!(
        source.commit.len() == 40
            && source
                .commit
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "Engine commit must be one lowercase 40-character hexadecimal value"
    );
    println!(
        "cargo:rustc-env=RUSTY_D20_ENGINE_REVISION={}",
        source.commit
    );
}
