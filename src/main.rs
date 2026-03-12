mod config;

use axum::{
    routing::get,
    Router,
};

use config::database::connect_db;

#[tokio::main]
async fn main() {

    dotenvy::dotenv().ok();

    let _db = connect_db().await;

    let app = Router::new()
        .route("/", get(|| async {
            "IoT Monitoring API"
        }));

    let listener =
        tokio::net::TcpListener::bind("0.0.0.0:8000")
            .await
            .unwrap();

    println!("Serveur démarré sur le port 8000");

    axum::serve(listener, app)
        .await
        .unwrap();
}