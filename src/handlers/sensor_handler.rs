use axum::{
    extract::State,
    Json,
};

use sqlx::Row;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    dto::sensor_dto::{
        CreateSensorDataDto,
        SensorDataResponse,
    },
};

pub async fn create_sensor_data(
    State(state): State<AppState>,
    Json(payload): Json<CreateSensorDataDto>,
) -> Json<SensorDataResponse> {

    let device_id =
        Uuid::parse_str(
            &payload.device_id
        )
        .unwrap();

    let row = sqlx::query(
        "
        INSERT INTO sensor_data
        (
            device_id,
            value,
            type
        )
        VALUES
        (
            $1,
            $2,
            $3
        )
        RETURNING id
        "
    )
    .bind(device_id)
    .bind(payload.value)
    .bind(&payload.r#type)
    .fetch_one(&state.db)
    .await
    .unwrap();

    Json(
        SensorDataResponse {
            id: row.get("id"),
            device_id:
                device_id.to_string(),
            value: payload.value,
            r#type: payload.r#type,
        }
    )
}

pub async fn list_sensor_data(
    State(state): State<AppState>,
) -> Json<Vec<SensorDataResponse>> {

    let rows = sqlx::query(
        "
        SELECT
            id,
            device_id,
            value,
            type
        FROM sensor_data
        ORDER BY timestamp DESC
        "
    )
    .fetch_all(&state.db)
    .await
    .unwrap();

    let data = rows
        .into_iter()
        .map(|row| {
            SensorDataResponse {
                id: row.get("id"),

                device_id:
                    row
                        .get::<Uuid,_>(
                            "device_id"
                        )
                        .to_string(),

                value:
                    row.get("value"),

                r#type:
                    row.get("type"),
            }
        })
        .collect();

    Json(data)
}