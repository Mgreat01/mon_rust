use axum::{
    extract::State,
    Json,
};

use uuid::Uuid;

use crate::{
    app_state::AppState,
    dto::device_dto::{
        CreateDeviceDto,
        DeviceResponse,
    },
};


pub async fn create_device(
    State(state): State<AppState>,
    Json(payload): Json<CreateDeviceDto>,
) -> Json<DeviceResponse> {

    let id = Uuid::new_v4();

    sqlx::query(
        "
        INSERT INTO devices
        (
            id,
            name,
            description
        )
        VALUES
        (
            $1,
            $2,
            $3
        )
        "
    )
    .bind(id)
    .bind(&payload.name)
    .bind(&payload.description)
    .execute(&state.db)
    .await
    .unwrap();

    Json(DeviceResponse {
        id: id.to_string(),
        name: payload.name,
        description: payload.description,
    })
}

use sqlx::Row;

pub async fn list_devices(
    State(state): State<AppState>,
) -> Json<Vec<DeviceResponse>> {

    let rows = sqlx::query(
        "
        SELECT
            id,
            name,
            description
        FROM devices
        "
    )
    .fetch_all(&state.db)
    .await
    .unwrap();

    let devices = rows
        .into_iter()
        .map(|row| DeviceResponse {
            id: row
                .get::<Uuid,_>("id")
                .to_string(),

            name: row.get("name"),

            description:
                row.get("description"),
        })
        .collect();

    Json(devices)
}