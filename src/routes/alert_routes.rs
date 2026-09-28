use axum::{routing::{get,patch}, Router};
use crate::{app_state::AppState, handlers::alert_handler::*};

pub fn alert_routes() -> Router<AppState> {
    Router::new()
        .route("/alert-rules", get(list_rules).post(create_rule))
        .route("/alert-rules/{id}", axum::routing::delete(delete_rule))
        .route("/alerts", get(list_alerts))
        .route("/alerts/{id}/status", patch(change_alert_status))
        .route("/notifications", get(list_notifications))
        .route("/notifications/{id}/read", patch(read_notification))
}
