use axum::{
    routing::{
        get,
        post,
    },
    Router,
};

use crate::{
    app_state::AppState,
    handlers::device_handler::{
        create_device,
        list_devices,
    },
};

pub fn device_routes() -> Router<AppState> {

    Router::new()

        .route(
            "/",
            post(create_device),
        )

        .route(
            "/",
            get(list_devices),
        )
}