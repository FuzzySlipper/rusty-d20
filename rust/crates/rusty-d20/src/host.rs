//! HTTP and static-file host for the durable browser shell.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::{GameRuntime, HealthDto, RuntimeReadoutDto};

#[derive(Clone)]
struct HostState {
    runtime: Arc<GameRuntime>,
}

pub fn router(runtime: Arc<GameRuntime>, web_root: Option<&Path>) -> Router {
    let api = Router::new()
        .route("/healthz", get(health))
        .route("/api/v1/readout", get(readout))
        .with_state(HostState { runtime });

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
        router(Arc::new(runtime), Some(web_root.as_path())),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

async fn health() -> Json<HealthDto> {
    Json(HealthDto {
        status: "ok".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    })
}

async fn readout(State(state): State<HostState>) -> Json<RuntimeReadoutDto> {
    Json(state.runtime.readout())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn api_exposes_health_and_rust_owned_readout() {
        let app = router(
            Arc::new(GameRuntime::bootstrap().expect("bootstrap state")),
            None,
        );

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
        assert_eq!(readout.entity_count, 1);
    }
}
