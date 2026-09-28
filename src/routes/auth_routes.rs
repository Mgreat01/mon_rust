use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    app_state::AppState,
    handlers::auth_handler::{
        forgot_password, login, logout, me, refresh, register, reset_password,
    },
};

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password))
        .route("/me", get(me))
}
