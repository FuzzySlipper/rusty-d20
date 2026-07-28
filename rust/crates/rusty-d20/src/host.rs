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
    EnterEncounterRequestDto, EquipItemRequestDto, ExpectedRevisionDto, GameRuntime,
    GameRuntimeError, GameSnapshotDto, HealthDto, PreviewActionRequestDto, RuntimeReadoutDto,
    TransferItemRequestDto, UnequipItemRequestDto,
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
        .route("/api/v1/session/new", post(new_adventure))
        .route("/api/v1/session/encounter", post(enter_encounter))
        .route("/api/v1/session/loadout/equip", post(equip_item))
        .route("/api/v1/session/loadout/unequip", post(unequip_item))
        .route("/api/v1/session/loadout/transfer", post(transfer_item))
        .route("/api/v1/session/preview", post(preview))
        .route("/api/v1/session/reaction", post(reaction))
        .route("/api/v1/session/action", post(action))
        .route("/api/v1/session/opposition", post(begin_opposition_turn))
        .route("/api/v1/session/camp", post(return_to_camp))
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

async fn new_adventure(
    State(state): State<HostState>,
    Json(request): Json<ExpectedRevisionDto>,
) -> ApiResult {
    mutate(&state, |runtime| {
        runtime.new_adventure(request.expected_revision)
    })
}

async fn enter_encounter(
    State(state): State<HostState>,
    Json(request): Json<EnterEncounterRequestDto>,
) -> ApiResult {
    mutate(&state, |runtime| runtime.enter_encounter(request))
}

async fn equip_item(
    State(state): State<HostState>,
    Json(request): Json<EquipItemRequestDto>,
) -> ApiResult {
    mutate(&state, |runtime| runtime.equip_item(request))
}

async fn unequip_item(
    State(state): State<HostState>,
    Json(request): Json<UnequipItemRequestDto>,
) -> ApiResult {
    mutate(&state, |runtime| runtime.unequip_item(request))
}

async fn transfer_item(
    State(state): State<HostState>,
    Json(request): Json<TransferItemRequestDto>,
) -> ApiResult {
    mutate(&state, |runtime| runtime.transfer_item(request))
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

async fn begin_opposition_turn(
    State(state): State<HostState>,
    Json(request): Json<ExpectedRevisionDto>,
) -> ApiResult {
    mutate(&state, |runtime| {
        runtime.begin_opposition_turn(request.expected_revision)
    })
}

async fn return_to_camp(
    State(state): State<HostState>,
    Json(request): Json<ExpectedRevisionDto>,
) -> ApiResult {
    mutate(&state, |runtime| {
        runtime.return_to_camp(request.expected_revision)
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
        ApiErrorKindDto::Invalid
        | ApiErrorKindDto::InvalidSlot
        | ApiErrorKindDto::Capacity
        | ApiErrorKindDto::Containment
        | ApiErrorKindDto::TrackBound
        | ApiErrorKindDto::Phase => StatusCode::UNPROCESSABLE_ENTITY,
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
                Request::post("/api/v1/session/new")
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
                Request::post("/api/v1/session/opposition")
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
        let reopened = reopened.snapshot().unwrap();
        assert!(reopened.campaign.is_some());
        assert!(reopened.encounter.is_none());
        fs::remove_file(save_path).unwrap();
    }

    #[tokio::test]
    async fn pending_preview_and_reaction_saves_reject_before_runtime_or_file_mutation() {
        assert_pending_save_rejection(false).await;
        assert_pending_save_rejection(true).await;
    }

    async fn assert_pending_save_rejection(apply_reaction: bool) {
        let (app, save_path) = test_state();
        let (_, camp_body) =
            post_json(&app, "/api/v1/session/new", r#"{"expectedRevision":0}"#).await;
        let camp: GameSnapshotDto = serde_json::from_slice(&camp_body).unwrap();
        let (_, start_body) = post_json(
            &app,
            "/api/v1/session/encounter",
            &format!(
                r#"{{"expectedRevision":{},"encounterId":"iron-warden"}}"#,
                camp.revision
            ),
        )
        .await;
        let start: GameSnapshotDto = serde_json::from_slice(&start_body).unwrap();

        let (save_status, _) = post_json(
            &app,
            "/api/v1/session/save",
            &format!(r#"{{"expectedRevision":{}}}"#, start.revision),
        )
        .await;
        assert_eq!(save_status, StatusCode::OK);
        let saved_bytes = fs::read(&save_path).unwrap();

        let encounter = start.encounter.as_ref().unwrap();
        let target = encounter
            .characters
            .iter()
            .find(|character| character.id != encounter.player_id)
            .unwrap()
            .id;
        let (_, preview_body) = post_json(
            &app,
            "/api/v1/session/preview",
            &format!(
                r#"{{"expectedRevision":{},"actorId":{},"targetId":{},"actionId":"longsword-strike"}}"#,
                start.revision, encounter.player_id, target
            ),
        )
        .await;
        let mut before: GameSnapshotDto = serde_json::from_slice(&preview_body).unwrap();

        if apply_reaction {
            let pending = before
                .encounter
                .as_ref()
                .unwrap()
                .pending_action
                .as_ref()
                .unwrap();
            let (_, reaction_body) = post_json(
                &app,
                "/api/v1/session/reaction",
                &format!(
                    r#"{{"expectedRevision":{},"previewToken":"{}","reactionId":"parry"}}"#,
                    before.revision, pending.token
                ),
            )
            .await;
            before = serde_json::from_slice(&reaction_body).unwrap();
        }

        let (rejected_status, rejected_body) = post_json(
            &app,
            "/api/v1/session/save",
            &format!(r#"{{"expectedRevision":{}}}"#, before.revision),
        )
        .await;
        assert_eq!(rejected_status, StatusCode::UNPROCESSABLE_ENTITY);
        let rejection: ApiErrorDto = serde_json::from_slice(&rejected_body).unwrap();
        assert_eq!(rejection.kind, ApiErrorKindDto::Invalid);
        assert_eq!(
            rejection.message,
            "resolve the pending action before saving"
        );

        let (_, after_body) = get_json(&app, "/api/v1/session").await;
        let after: GameSnapshotDto = serde_json::from_slice(&after_body).unwrap();
        assert_eq!(after, before);
        assert_eq!(fs::read(&save_path).unwrap(), saved_bytes);

        let reopened = load_runtime(&save_path).unwrap().snapshot().unwrap();
        assert_eq!(reopened.revision, start.revision);
        assert!(reopened
            .encounter
            .as_ref()
            .unwrap()
            .pending_action
            .is_none());
        fs::remove_file(save_path).unwrap();
    }

    async fn post_json(app: &Router, path: &str, body: &str) -> (StatusCode, Vec<u8>) {
        let response = app
            .clone()
            .oneshot(
                Request::post(path)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, bytes.to_vec())
    }

    async fn get_json(app: &Router, path: &str) -> (StatusCode, Vec<u8>) {
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, bytes.to_vec())
    }
}
