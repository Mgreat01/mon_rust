use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    dto::device_dto::{
        CreateDeviceDto, CreatedDeviceResponse, DeviceResponse, Pagination, UpdateDeviceDto,
    },
    error::{ApiError, ApiResult},
    middleware::jwt::AuthUser,
    services::auth_service::hash_password,
};

type DeviceRow = (
    Uuid,
    String,
    Option<String>,
    String,
    String,
    Option<DateTime<Utc>>,
    DateTime<Utc>,
);

pub async fn create_device(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateDeviceDto>,
) -> ApiResult<(StatusCode, Json<CreatedDeviceResponse>)> {
    auth.require_role(&["ADMIN", "OPERATOR"])?;
    validate_name(&payload.name)?;
    let device_type = payload.device_type.unwrap_or_else(|| "generic".to_owned());
    validate_short_text(&device_type, "Type d'appareil")?;
    let device_id = Uuid::new_v4();
    let api_key = format!("dev_{device_id}.{}", Uuid::new_v4().simple());
    let row = sqlx::query_as::<_, DeviceRow>(
        "INSERT INTO devices (id,tenant_id,name,description,device_type,api_key_hash) \
         VALUES ($1,$2,$3,$4,$5,$6) RETURNING id,name,description,device_type,status,last_seen_at,created_at",
    )
    .bind(device_id)
    .bind(auth.tenant_id)
    .bind(payload.name.trim())
    .bind(payload.description)
    .bind(device_type)
    .bind(hash_password(&api_key)?)
    .fetch_one(&state.db)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedDeviceResponse {
            device: to_response(row),
            api_key,
        }),
    ))
}

pub async fn list_devices(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(page): Query<Pagination>,
) -> ApiResult<Json<Vec<DeviceResponse>>> {
    let limit = page.limit.unwrap_or(50).clamp(1, 200);
    let offset = page.offset.unwrap_or(0).max(0);
    let rows = sqlx::query_as::<_, DeviceRow>(
        "SELECT id,name,description,device_type,status,last_seen_at,created_at FROM devices \
         WHERE tenant_id=$1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(auth.tenant_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(to_response).collect()))
}

pub async fn get_device(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<DeviceResponse>> {
    let row = find_device(&state, auth.tenant_id, id).await?;
    Ok(Json(to_response(row)))
}

pub async fn update_device(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateDeviceDto>,
) -> ApiResult<Json<DeviceResponse>> {
    auth.require_role(&["ADMIN", "OPERATOR"])?;
    if let Some(name) = &payload.name {
        validate_name(name)?;
    }
    if let Some(kind) = &payload.device_type {
        validate_short_text(kind, "Type d'appareil")?;
    }
    if let Some(status) = &payload.status {
        if !["ONLINE", "OFFLINE", "DISABLED"].contains(&status.as_str()) {
            return Err(ApiError::bad_request("Statut d'appareil invalide"));
        }
    }
    let row = sqlx::query_as::<_, DeviceRow>(
        "UPDATE devices SET name=COALESCE($3,name), description=COALESCE($4,description), \
         device_type=COALESCE($5,device_type), status=COALESCE($6,status), updated_at=now() \
         WHERE id=$1 AND tenant_id=$2 RETURNING id,name,description,device_type,status,last_seen_at,created_at",
    )
    .bind(id)
    .bind(auth.tenant_id)
    .bind(payload.name.map(|value| value.trim().to_owned()))
    .bind(payload.description)
    .bind(payload.device_type)
    .bind(payload.status)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Appareil"))?;
    Ok(Json(to_response(row)))
}

pub async fn delete_device(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    auth.require_role(&["ADMIN"])?;
    let result = sqlx::query("DELETE FROM devices WHERE id=$1 AND tenant_id=$2")
        .bind(id)
        .bind(auth.tenant_id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Appareil"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn find_device(state: &AppState, tenant_id: Uuid, id: Uuid) -> ApiResult<DeviceRow> {
    sqlx::query_as::<_, DeviceRow>(
        "SELECT id,name,description,device_type,status,last_seen_at,created_at FROM devices WHERE id=$1 AND tenant_id=$2",
    )
    .bind(id)
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Appareil"))
}

fn to_response(row: DeviceRow) -> DeviceResponse {
    DeviceResponse {
        id: row.0,
        name: row.1,
        description: row.2,
        device_type: row.3,
        status: row.4,
        last_seen_at: row.5,
        created_at: row.6,
    }
}

fn validate_name(value: &str) -> ApiResult<()> {
    if value.trim().is_empty() || value.trim().len() > 255 {
        return Err(ApiError::bad_request("Nom d'appareil invalide"));
    }
    Ok(())
}

fn validate_short_text(value: &str, label: &str) -> ApiResult<()> {
    if value.trim().is_empty() || value.trim().len() > 100 {
        return Err(ApiError::bad_request(format!("{label} invalide")));
    }
    Ok(())
}
