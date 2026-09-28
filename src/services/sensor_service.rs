use chrono::{DateTime, Duration, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    app_state::{AlertEvent, AppEvent, AppState},
    dto::sensor_dto::{CreateSensorDataDto, SensorDataResponse},
    error::{ApiError, ApiResult},
    services::auth_service::verify_password,
};

pub struct DeviceIdentity {
    pub id: Uuid,
    pub tenant_id: Uuid,
}

pub async fn authenticate_device(
    pool: &PgPool,
    authorization: Option<&str>,
) -> ApiResult<DeviceIdentity> {
    let api_key = authorization
        .and_then(|value| value.strip_prefix("Device "))
        .ok_or_else(|| ApiError::unauthorized("Clé d'appareil absente"))?;
    let id_part = api_key
        .strip_prefix("dev_")
        .and_then(|value| value.split_once('.').map(|parts| parts.0))
        .ok_or_else(|| ApiError::unauthorized("Clé d'appareil invalide"))?;
    let device_id =
        Uuid::parse_str(id_part).map_err(|_| ApiError::unauthorized("Clé d'appareil invalide"))?;
    let row = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT tenant_id, api_key_hash, status FROM devices WHERE id=$1",
    )
    .bind(device_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::unauthorized("Clé d'appareil invalide"))?;
    if row.2 == "DISABLED" || !verify_password(&row.1, api_key) {
        return Err(ApiError::unauthorized("Clé d'appareil invalide"));
    }
    Ok(DeviceIdentity {
        id: device_id,
        tenant_id: row.0,
    })
}

pub async fn ingest(
    state: &AppState,
    device: DeviceIdentity,
    payload: CreateSensorDataDto,
) -> ApiResult<SensorDataResponse> {
    validate(&payload)?;
    let timestamp = payload.timestamp.unwrap_or_else(Utc::now);
    if timestamp < Utc::now() - Duration::days(30) || timestamp > Utc::now() + Duration::minutes(5)
    {
        return Err(ApiError::bad_request(
            "Horodatage de mesure hors de la fenêtre autorisée",
        ));
    }
    let event_id = payload.event_id.unwrap_or_else(Uuid::new_v4);
    let mut tx = state.db.begin().await?;
    let existing = sqlx::query_as::<_, (DateTime<Utc>, String, f64, Option<String>)>(
        "SELECT time,metric_type,value,unit FROM sensor_data WHERE tenant_id=$1 AND device_id=$2 AND event_id=$3 LIMIT 1",
    )
    .bind(device.tenant_id).bind(device.id).bind(event_id)
    .fetch_optional(&mut *tx).await?;
    if let Some(row) = existing {
        tx.rollback().await?;
        return Ok(SensorDataResponse {
            event_id,
            device_id: device.id,
            timestamp: row.0,
            metric_type: row.1,
            value: row.2,
            unit: row.3,
        });
    }
    sqlx::query("INSERT INTO sensor_data (time,event_id,tenant_id,device_id,metric_type,value,unit) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(timestamp).bind(event_id).bind(device.tenant_id).bind(device.id)
        .bind(payload.metric_type.trim()).bind(payload.value).bind(&payload.unit)
        .execute(&mut *tx).await?;
    sqlx::query("UPDATE devices SET status='ONLINE',last_seen_at=now(),updated_at=now() WHERE id=$1 AND tenant_id=$2")
        .bind(device.id).bind(device.tenant_id).execute(&mut *tx).await?;
    let alerts = evaluate_alerts(&mut tx, &device, &payload.metric_type, payload.value).await?;
    tx.commit().await?;
    let response = SensorDataResponse {
        event_id,
        device_id: device.id,
        timestamp,
        metric_type: payload.metric_type,
        value: payload.value,
        unit: payload.unit,
    };
    let _ = state.events.send(AppEvent::SensorData {
        tenant_id: device.tenant_id,
        data: response.clone(),
    });
    for alert in alerts {
        let _ = state.events.send(AppEvent::Alert {
            tenant_id: device.tenant_id,
            data: alert,
        });
    }
    Ok(response)
}

async fn evaluate_alerts(
    tx: &mut Transaction<'_, Postgres>,
    device: &DeviceIdentity,
    metric: &str,
    value: f64,
) -> ApiResult<Vec<AlertEvent>> {
    let rules = sqlx::query_as::<_, (Uuid, String, String, f64, String, i32)>(
        "SELECT id,name,operator,threshold,severity,cooldown_seconds FROM alert_rules WHERE tenant_id=$1 AND device_id=$2 AND metric_type=$3 AND enabled",
    ).bind(device.tenant_id).bind(device.id).bind(metric).fetch_all(&mut **tx).await?;
    let mut events = Vec::new();
    for (rule_id, name, operator, threshold, severity, cooldown) in rules {
        let triggered = match operator.as_str() {
            "gt" => value > threshold,
            "gte" => value >= threshold,
            "lt" => value < threshold,
            "lte" => value <= threshold,
            "eq" => (value - threshold).abs() < f64::EPSILON,
            _ => false,
        };
        if !triggered {
            continue;
        }
        let message = format!("{name}: valeur {value} (seuil {operator} {threshold})");
        let id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO alerts (tenant_id,device_id,rule_id,value,severity,message) \
             SELECT $1,$2,$3,$4,$5,$6 WHERE NOT EXISTS (SELECT 1 FROM alerts WHERE tenant_id=$1 AND rule_id=$3 AND status!='RESOLVED' AND created_at > now()-make_interval(secs => $7)) RETURNING id",
        ).bind(device.tenant_id).bind(device.id).bind(rule_id).bind(value).bind(&severity).bind(&message).bind(cooldown)
        .fetch_optional(&mut **tx).await?;
        if let Some(id) = id {
            sqlx::query("INSERT INTO notifications (tenant_id,user_id,alert_id,title,message) SELECT $1,id,$2,'Nouvelle alerte',$3 FROM users WHERE tenant_id=$1 AND is_active")
                .bind(device.tenant_id).bind(id).bind(&message).execute(&mut **tx).await?;
            events.push(AlertEvent {
                id,
                device_id: device.id,
                severity,
                message,
            });
        }
    }
    Ok(events)
}

fn validate(payload: &CreateSensorDataDto) -> ApiResult<()> {
    if !payload.value.is_finite() {
        return Err(ApiError::bad_request("La valeur doit être finie"));
    }
    if payload.metric_type.trim().is_empty() || payload.metric_type.len() > 100 {
        return Err(ApiError::bad_request("Type de métrique invalide"));
    }
    if payload.unit.as_ref().is_some_and(|unit| unit.len() > 30) {
        return Err(ApiError::bad_request("Unité trop longue"));
    }
    Ok(())
}
