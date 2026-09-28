use axum::{
    extract::{Query, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    Json,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    config::tenant::begin_tenant,
    dto::sensor_dto::{
        AnalyticsPoint, AnalyticsQuery, CreateSensorDataDto, SensorDataQuery, SensorDataResponse,
    },
    error::{ApiError, ApiResult},
    middleware::jwt::AuthUser,
    services::sensor_service::{authenticate_device, ingest},
};

type SensorRow = (DateTime<Utc>, Uuid, Uuid, String, f64, Option<String>);

pub async fn create_sensor_data(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateSensorDataDto>,
) -> ApiResult<(StatusCode, Json<SensorDataResponse>)> {
    let authorization = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    let device = authenticate_device(&state.db, authorization).await?;
    Ok((
        StatusCode::CREATED,
        Json(ingest(&state, device, payload).await?),
    ))
}

pub async fn list_sensor_data(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<SensorDataQuery>,
) -> ApiResult<Json<Vec<SensorDataResponse>>> {
    validate_range(query.from, query.to)?;
    let mut transaction = begin_tenant(&state.db, auth.tenant_id).await?;
    let rows = sqlx::query_as::<_, SensorRow>(
        "SELECT time,event_id,device_id,metric_type,value,unit FROM sensor_data WHERE tenant_id=$1 \
         AND ($2::uuid IS NULL OR device_id=$2) AND ($3::text IS NULL OR metric_type=$3) \
         AND ($4::timestamptz IS NULL OR time >= $4) AND ($5::timestamptz IS NULL OR time <= $5) \
         ORDER BY time DESC LIMIT $6 OFFSET $7",
    )
    .bind(auth.tenant_id).bind(query.device_id).bind(query.metric_type)
    .bind(query.from).bind(query.to).bind(query.limit.unwrap_or(100).clamp(1, 200))
    .bind(query.offset.unwrap_or(0).max(0)).fetch_all(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(Json(rows.into_iter().map(to_response).collect()))
}

pub async fn latest_sensor_data(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<SensorDataQuery>,
) -> ApiResult<Json<Vec<SensorDataResponse>>> {
    let mut transaction = begin_tenant(&state.db, auth.tenant_id).await?;
    let rows = sqlx::query_as::<_, SensorRow>(
        "SELECT DISTINCT ON (device_id,metric_type) time,event_id,device_id,metric_type,value,unit \
         FROM sensor_data WHERE tenant_id=$1 AND ($2::uuid IS NULL OR device_id=$2) \
         AND ($3::text IS NULL OR metric_type=$3) ORDER BY device_id,metric_type,time DESC",
    ).bind(auth.tenant_id).bind(query.device_id).bind(query.metric_type)
    .fetch_all(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(Json(rows.into_iter().map(to_response).collect()))
}

pub async fn analytics(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<AnalyticsQuery>,
) -> ApiResult<Json<Vec<AnalyticsPoint>>> {
    validate_range(Some(query.from), Some(query.to))?;
    let interval = query.interval_minutes.unwrap_or(5).clamp(1, 10_080);
    let mut transaction = begin_tenant(&state.db, auth.tenant_id).await?;
    let rows = sqlx::query_as::<_, (DateTime<Utc>, f64, f64, f64, i64)>(
        "SELECT time_bucket(make_interval(mins => $6),time) AS bucket,avg(value),min(value),max(value),count(*) \
         FROM sensor_data WHERE tenant_id=$1 AND device_id=$2 AND metric_type=$3 AND time >= $4 AND time <= $5 \
         GROUP BY bucket ORDER BY bucket",
    ).bind(auth.tenant_id).bind(query.device_id).bind(query.metric_type)
    .bind(query.from).bind(query.to).bind(interval).fetch_all(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| AnalyticsPoint {
                timestamp: row.0,
                average: row.1,
                min: row.2,
                max: row.3,
                count: row.4,
            })
            .collect(),
    ))
}

fn validate_range(from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> ApiResult<()> {
    if from.zip(to).is_some_and(|(from, to)| from >= to) {
        return Err(ApiError::bad_request(
            "La date 'from' doit précéder la date 'to'",
        ));
    }
    Ok(())
}

fn to_response(row: SensorRow) -> SensorDataResponse {
    SensorDataResponse {
        timestamp: row.0,
        event_id: row.1,
        device_id: row.2,
        metric_type: row.3,
        value: row.4,
        unit: row.5,
    }
}
