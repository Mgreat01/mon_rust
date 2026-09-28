use axum::{routing::get, Router};

use crate::{
    app_state::AppState,
    handlers::device_handler::{
        create_device, delete_device, get_device, list_devices, revoke_device_credential,
        rotate_device_credential, update_device,
    },
};

pub fn device_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_devices).post(create_device))
        .route(
            "/{id}",
            get(get_device).put(update_device).delete(delete_device),
        )
        .route(
            "/{id}/credentials",
            axum::routing::post(rotate_device_credential).delete(revoke_device_credential),
        )
}
