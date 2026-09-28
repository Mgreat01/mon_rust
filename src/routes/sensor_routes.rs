use axum::{routing::{get, post}, Router};

use crate::{
    app_state::AppState,
    handlers::sensor_handler::{
        analytics, create_sensor_data, latest_sensor_data, list_sensor_data,
    },
};

pub fn sensor_routes()
    -> Router<AppState>
{
    Router::new()
        .route("/", get(list_sensor_data))
        .route("/ingest", post(create_sensor_data))
        .route("/latest", get(latest_sensor_data))
        .route("/analytics", get(analytics))
}
