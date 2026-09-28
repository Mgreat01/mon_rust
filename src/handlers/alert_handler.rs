use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    config::tenant::begin_tenant,
    error::{ApiError, ApiResult},
    middleware::jwt::AuthUser,
};

#[derive(Deserialize)]
pub struct CreateRule {
    pub device_id: Uuid,
    pub name: String,
    pub metric_type: String,
    pub operator: String,
    pub threshold: f64,
    pub severity: String,
    pub cooldown_seconds: Option<i32>,
}
#[derive(Serialize)]
pub struct Rule {
    pub id: Uuid,
    pub device_id: Uuid,
    pub name: String,
    pub metric_type: String,
    pub operator: String,
    pub threshold: f64,
    pub severity: String,
    pub cooldown_seconds: i32,
    pub enabled: bool,
}
#[derive(Deserialize)]
pub struct Page {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
#[derive(Serialize)]
pub struct Alert {
    pub id: Uuid,
    pub device_id: Uuid,
    pub rule_id: Uuid,
    pub value: f64,
    pub severity: String,
    pub message: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}
#[derive(Deserialize)]
pub struct ChangeStatus {
    pub status: String,
}
#[derive(Serialize)]
pub struct Notification {
    pub id: Uuid,
    pub alert_id: Option<Uuid>,
    pub title: String,
    pub message: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

type RuleRow = (Uuid, Uuid, String, String, String, f64, String, i32, bool);

pub async fn create_rule(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateRule>,
) -> ApiResult<(StatusCode, Json<Rule>)> {
    auth.require_role(&["ADMIN", "OPERATOR"])?;
    if body.name.trim().is_empty()
        || body.metric_type.trim().is_empty()
        || !body.threshold.is_finite()
        || !["gt", "gte", "lt", "lte", "eq"].contains(&body.operator.as_str())
        || !["INFO", "WARNING", "CRITICAL"].contains(&body.severity.as_str())
    {
        return Err(ApiError::bad_request("Règle d'alerte invalide"));
    }
    let mut transaction = begin_tenant(&state.db, auth.tenant_id).await?;
    let row = sqlx::query_as::<_,RuleRow>("INSERT INTO alert_rules (tenant_id,device_id,name,metric_type,operator,threshold,severity,cooldown_seconds) SELECT $1,$2,$3,$4,$5,$6,$7,$8 WHERE EXISTS (SELECT 1 FROM devices WHERE tenant_id=$1 AND id=$2) RETURNING id,device_id,name,metric_type,operator,threshold,severity,cooldown_seconds,enabled")
        .bind(auth.tenant_id).bind(body.device_id).bind(body.name.trim()).bind(body.metric_type.trim()).bind(body.operator).bind(body.threshold).bind(body.severity).bind(body.cooldown_seconds.unwrap_or(300).max(0))
        .fetch_optional(&mut *transaction).await?.ok_or_else(|| ApiError::not_found("Appareil"))?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(rule(row))))
}

pub async fn list_rules(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<Rule>>> {
    let mut transaction = begin_tenant(&state.db, auth.tenant_id).await?;
    let rows=sqlx::query_as::<_,RuleRow>("SELECT id,device_id,name,metric_type,operator,threshold,severity,cooldown_seconds,enabled FROM alert_rules WHERE tenant_id=$1 ORDER BY created_at DESC").bind(auth.tenant_id).fetch_all(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(Json(rows.into_iter().map(rule).collect()))
}

pub async fn delete_rule(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    auth.require_role(&["ADMIN", "OPERATOR"])?;
    let mut transaction = begin_tenant(&state.db, auth.tenant_id).await?;
    let result = sqlx::query("DELETE FROM alert_rules WHERE tenant_id=$1 AND id=$2")
        .bind(auth.tenant_id)
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Règle"));
    }
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_alerts(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(page): Query<Page>,
) -> ApiResult<Json<Vec<Alert>>> {
    let mut transaction = begin_tenant(&state.db, auth.tenant_id).await?;
    let rows=sqlx::query_as::<_,(Uuid,Uuid,Uuid,f64,String,String,String,DateTime<Utc>)>("SELECT id,device_id,rule_id,value,severity,message,status,created_at FROM alerts WHERE tenant_id=$1 ORDER BY created_at DESC LIMIT $2 OFFSET $3")
        .bind(auth.tenant_id).bind(page.limit.unwrap_or(100).clamp(1,200)).bind(page.offset.unwrap_or(0).max(0)).fetch_all(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| Alert {
                id: r.0,
                device_id: r.1,
                rule_id: r.2,
                value: r.3,
                severity: r.4,
                message: r.5,
                status: r.6,
                created_at: r.7,
            })
            .collect(),
    ))
}

pub async fn change_alert_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<ChangeStatus>,
) -> ApiResult<StatusCode> {
    auth.require_role(&["ADMIN", "OPERATOR"])?;
    if !["ACKNOWLEDGED", "RESOLVED"].contains(&body.status.as_str()) {
        return Err(ApiError::bad_request("Statut d'alerte invalide"));
    }
    let mut transaction = begin_tenant(&state.db, auth.tenant_id).await?;
    let result=sqlx::query("UPDATE alerts SET status=$3,acknowledged_at=CASE WHEN $3='ACKNOWLEDGED' THEN now() ELSE acknowledged_at END,resolved_at=CASE WHEN $3='RESOLVED' THEN now() ELSE NULL END WHERE tenant_id=$1 AND id=$2")
        .bind(auth.tenant_id).bind(id).bind(body.status).execute(&mut *transaction).await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Alerte"));
    }
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_notifications(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(page): Query<Page>,
) -> ApiResult<Json<Vec<Notification>>> {
    let mut transaction = begin_tenant(&state.db, auth.tenant_id).await?;
    let rows=sqlx::query_as::<_,(Uuid,Option<Uuid>,String,String,bool,DateTime<Utc>)>("SELECT id,alert_id,title,message,is_read,created_at FROM notifications WHERE tenant_id=$1 AND user_id=$2 ORDER BY created_at DESC LIMIT $3 OFFSET $4")
        .bind(auth.tenant_id).bind(auth.user_id).bind(page.limit.unwrap_or(100).clamp(1,200)).bind(page.offset.unwrap_or(0).max(0)).fetch_all(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| Notification {
                id: r.0,
                alert_id: r.1,
                title: r.2,
                message: r.3,
                is_read: r.4,
                created_at: r.5,
            })
            .collect(),
    ))
}

pub async fn read_notification(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let mut transaction = begin_tenant(&state.db, auth.tenant_id).await?;
    let result = sqlx::query(
        "UPDATE notifications SET is_read=true WHERE tenant_id=$1 AND user_id=$2 AND id=$3",
    )
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(id)
    .execute(&mut *transaction)
    .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Notification"));
    }
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

fn rule(r: RuleRow) -> Rule {
    Rule {
        id: r.0,
        device_id: r.1,
        name: r.2,
        metric_type: r.3,
        operator: r.4,
        threshold: r.5,
        severity: r.6,
        cooldown_seconds: r.7,
        enabled: r.8,
    }
}
