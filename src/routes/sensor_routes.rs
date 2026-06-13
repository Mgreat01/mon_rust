use axum::{
    routing::{
        get,
        post,
    },
    Router,
};

use crate::{
    app_state::AppState,
    handlers::sensor_handler::{
        create_sensor_data,
        list_sensor_data,
    },
};

pub fn sensor_routes()
    -> Router<AppState>
{
    Router::new()

        .route(
            "/",
            post(create_sensor_data),
        )

        .route(
            "/",
            get(list_sensor_data),
        )
}