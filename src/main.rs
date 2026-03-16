mod config;
mod app_state;
mod routes;
mod handlers;
mod services;
mod dto;
mod models;
mod middleware;

use axum::{
    routing::get,
    Router,
};

use app_state::AppState;
use config::database::connect_db;
use routes::auth_routes::auth_routes;

#[tokio::main]
async fn main() {

    dotenvy::dotenv().ok();

    let db = connect_db().await;

    println!("Connexion PostgreSQL réussie");

    let state = AppState {
        db: db.clone(),
    };

    let app = Router::new()

        .route(
            "/",
            get(|| async {
                "IoT Monitoring API"
            }),
        )

        .nest(
            "/api/auth",
            auth_routes(),
        )
        .nest(
        "/api/devices",
        device_routes(),
        )

        .with_state(state);

    let listener =
        tokio::net::TcpListener::bind(
            "0.0.0.0:8000",
        )
        .await
        .unwrap();

    println!(
        "Serveur démarré sur http://localhost:8000"
    );

    use routes::{
    auth_routes::auth_routes,
    device_routes::device_routes,
};
.nest(
    "/api/devices",
    device_routes(),
)

    axum::serve(listener, app)
        .await
        .unwrap();
}