mod app_state;
mod config;
mod dto;
mod error;
mod handlers;
mod middleware;
mod routes;
mod services;
mod websocket;

use std::net::SocketAddr;

use app_state::AppState;
use axum::{
    http::{header::{AUTHORIZATION, CONTENT_TYPE}, HeaderValue, Method, StatusCode},
    routing::get,
    Json, Router,
};
use config::database::connect_db;
use error::ApiResult;
use routes::{
    alert_routes::alert_routes, auth_routes::auth_routes, device_routes::device_routes,
    sensor_routes::sensor_routes,
};
use serde_json::{json, Value};
use tokio::signal;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=info".into()))
        .json()
        .init();

    let db = connect_db().await?;
    let (events, _) = tokio::sync::broadcast::channel(1_024);
    let state = AppState { db, events };
    let cors_origin = std::env::var("CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:4200".to_owned());
    let origin = HeaderValue::from_str(&cors_origin)?;

    let api = Router::new()
        .nest("/auth", auth_routes())
        .nest("/devices", device_routes())
        .nest("/sensor-data", sensor_routes())
        .merge(alert_routes());
    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(readiness))
        .route("/ws/monitoring", get(websocket::connect))
        .nest("/api/v1", api)
        .layer(
            CorsLayer::new()
                .allow_origin(origin)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE])
                .allow_headers([AUTHORIZATION, CONTENT_TYPE]),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let address: SocketAddr = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:8000".to_owned())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "serveur IoT démarré");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn health() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({"status":"ok"})))
}

async fn readiness(axum::extract::State(state): axum::extract::State<AppState>) -> ApiResult<Json<Value>> {
    sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.db).await?;
    Ok(Json(json!({"status":"ready"})))
}

async fn shutdown_signal() {
    let ctrl_c = async { signal::ctrl_c().await.expect("gestionnaire Ctrl+C") };
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("gestionnaire SIGTERM")
            .recv().await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("arrêt gracieux demandé");
}
