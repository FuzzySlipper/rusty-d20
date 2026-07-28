//! HTTP and static-file host for the durable browser shell.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::{
    ApiErrorDto, ApiErrorKindDto, ApplyActionRequestDto, ApplyReactionRequestDto,
    ExpectedRevisionDto, GameRuntime, GameRuntimeError, GameSnapshotDto, HealthDto,
    PreviewActionRequestDto, RuntimeReadoutDto,
};

#[derive(Clone)]
struct HostState {
    runtime: Arc<Mutex<GameRuntime>>,
    save_path: Arc<PathBuf>,
}

type ApiResult = Result<Json<GameSnapshotDto>, (StatusCode, Json<ApiErrorDto>)>;

pub fn router(
    runtime: Arc<Mutex<GameRuntime>>,
    save_path: PathBuf,
    web_root: Option<&Path>,
) -> Router {
    let api = Router::new()
        .route("/healthz", get(health))
        .route("/api/v1/readout", get(readout))
        .route("/api/v1/session", get(session))
        .route("/api/v1/session/start", post(start))
        .route("/api/v1/session/preview", post(preview))
        .route("/api/v1/session/reaction", post(reaction))
        .route("/api/v1/session/action", post(action))
        .route("/api/v1/session/turn", post(advance_turn))
        .route("/api/v1/session/save", post(save))
        .with_state(HostState {
            runtime,
            save_path: Arc::new(save_path),
        });

    let app = if let Some(root) = web_root {
        let index = root.join("index.html");
        api.fallback_service(ServeDir::new(root).fallback(ServeFile::new(index)))
    } else {
        api
    };

    app.layer(TraceLayer::new_for_http())
}

pub async fn serve(
    address: &str,
    web_root: PathBuf,
    save_path: PathBuf,
    runtime: GameRuntime,
) -> Result<(), Box<dyn std::error::Error>> {
    if !web_root.join("index.html").is_file() {
        return Err(format!(
            "web root does not contain index.html: {}",
            web_root.display()
        )
        .into());
    }

    let listener = TcpListener::bind(address).await?;
    let local_address = listener.local_addr()?;
    println!("BASE_URL=http://{local_address}");
    axum::serve(
        listener,
        router(
            Arc::new(Mutex::new(runtime)),
            save_path,
            Some(web_root.as_path()),
        ),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

pub fn load_runtime(save_path: &Path) -> Result<GameRuntime, Box<dyn std::error::Error>> {
    match fs::read_to_string(save_path) {
        Ok(encoded) => Ok(GameRuntime::decode_save(&encoded)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(GameRuntime::empty()?),
        Err(error) => Err(error.into()),
    }
}

async fn health() -> Json<HealthDto> {
    Json(HealthDto {
        status: "ok".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    })
}

async fn readout(State(state): State<HostState>) -> Json<RuntimeReadoutDto> {
    Json(
        lock_runtime(&state)
            .expect("poisoned runtime lock is an unrecoverable host defect")
            .readout(),
    )
}

async fn session(State(state): State<HostState>) -> ApiResult {
    snapshot(&state)
}

async fn start(
    State(state): State<HostState>,
    Json(request): Json<ExpectedRevisionDto>,
) -> ApiResult {
    mutate(&state, |runtime| {
        runtime.start_encounter(request.expected_revision)
    })
}

async fn preview(
    State(state): State<HostState>,
    Json(request): Json<PreviewActionRequestDto>,
) -> ApiResult {
    mutate(&state, |runtime| runtime.preview_action(request))
}

async fn reaction(
    State(state): State<HostState>,
    Json(request): Json<ApplyReactionRequestDto>,
) -> ApiResult {
    mutate(&state, |runtime| runtime.apply_reaction(request))
}

async fn action(
    State(state): State<HostState>,
    Json(request): Json<ApplyActionRequestDto>,
) -> ApiResult {
    mutate(&state, |runtime| runtime.apply_action(request))
}

async fn advance_turn(
    State(state): State<HostState>,
    Json(request): Json<ExpectedRevisionDto>,
) -> ApiResult {
    mutate(&state, |runtime| {
        runtime.advance_turn(request.expected_revision)
    })
}

async fn save(
    State(state): State<HostState>,
    Json(request): Json<ExpectedRevisionDto>,
) -> ApiResult {
    let mut runtime = lock_runtime(&state).map_err(api_error)?;
    let encoded = runtime
        .encode_save_at(request.expected_revision)
        .map_err(api_error)?;
    persist_atomic(&state.save_path, encoded.as_bytes()).map_err(|error| {
        api_error(GameRuntimeError::InvalidSave(format!(
            "could not persist {}: {error}",
            state.save_path.display()
        )))
    })?;
    runtime.mark_saved(request.expected_revision);
    runtime.snapshot().map(Json).map_err(api_error)
}

fn snapshot(state: &HostState) -> ApiResult {
    lock_runtime(state)
        .map_err(api_error)?
        .snapshot()
        .map(Json)
        .map_err(api_error)
}

fn mutate(
    state: &HostState,
    operation: impl FnOnce(&mut GameRuntime) -> Result<GameSnapshotDto, GameRuntimeError>,
) -> ApiResult {
    let mut runtime = lock_runtime(state).map_err(api_error)?;
    operation(&mut runtime).map(Json).map_err(api_error)
}

fn lock_runtime(state: &HostState) -> Result<MutexGuard<'_, GameRuntime>, GameRuntimeError> {
    state
        .runtime
        .lock()
        .map_err(|_| GameRuntimeError::InvalidState("runtime lock was poisoned".to_owned()))
}

fn api_error(error: GameRuntimeError) -> (StatusCode, Json<ApiErrorDto>) {
    let body = error.api_error();
    let status = match body.kind {
        ApiErrorKindDto::Stale => StatusCode::CONFLICT,
        ApiErrorKindDto::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
        ApiErrorKindDto::NotFound => StatusCode::NOT_FOUND,
        ApiErrorKindDto::Persistence | ApiErrorKindDto::Internal => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    (status, Json(body))
}

fn persist_atomic(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("save.json");
    let temporary = path.with_file_name(format!(".{file_name}.tmp"));
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, path)
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;

    static NEXT_TEST_SAVE: AtomicU64 = AtomicU64::new(1);

    fn test_state() -> (Router, PathBuf) {
        let save = std::env::temp_dir().join(format!(
            "rusty-d20-host-test-{}-{}.json",
            std::process::id(),
            NEXT_TEST_SAVE.fetch_add(1, Ordering::Relaxed)
        ));
        (
            router(
                Arc::new(Mutex::new(GameRuntime::empty().expect("empty state"))),
                save.clone(),
                None,
            ),
            save,
        )
    }

    #[tokio::test]
    async fn api_exposes_health_and_rust_owned_readout() {
        let (app, _) = test_state();

        let health_response = app
            .clone()
            .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(health_response.status(), StatusCode::OK);

        let readout_response = app
            .oneshot(Request::get("/api/v1/readout").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(readout_response.status(), StatusCode::OK);
        let bytes = readout_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let readout: RuntimeReadoutDto = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(readout.product, "Rusty D20");
        assert_eq!(readout.engine_revision, crate::ENGINE_REVISION);
        assert_eq!(readout.entity_count, 0);
    }

    #[tokio::test]
    async fn typed_commands_reject_stale_mutation_and_persist_reopenable_state() {
        let (app, save_path) = test_state();
        let start_response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/session/start")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"expectedRevision":0}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(start_response.status(), StatusCode::OK);
        let start: GameSnapshotDto = serde_json::from_slice(
            &start_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();

        let stale = app
            .clone()
            .oneshot(
                Request::post("/api/v1/session/turn")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"expectedRevision":0}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stale.status(), StatusCode::CONFLICT);

        let save_response = app
            .oneshot(
                Request::post("/api/v1/session/save")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"expectedRevision":{}}}"#,
                        start.revision
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(save_response.status(), StatusCode::OK);
        let reopened = load_runtime(&save_path).unwrap();
        assert!(reopened.snapshot().unwrap().encounter.is_some());
        fs::remove_file(save_path).unwrap();
    }
}
