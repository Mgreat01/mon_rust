use axum::{routing::{get, post}, Router};

use crate::{
    app_state::AppState,
    handlers::auth_handler::{
        login, me,
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
        .route("/me", get(me))
}
