use axum::{
    routing::post,
    Router,
};

use crate::{
    app_state::AppState,
    handlers::auth_handler::{
        login,
        register,
    },
};

pub fn auth_routes(
) -> Router<AppState> {

    Router::new()

        .route(
            "/register",
            post(register),
        )

        .route(
            "/login",
            post(login),
        )
}