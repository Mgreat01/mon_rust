use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateSensorDataDto {
    pub event_id: Option<Uuid>,
    #[serde(alias = "type")]
    pub metric_type: String,
    pub value: f64,
    pub unit: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SensorDataResponse {
    pub event_id: Uuid,
    pub device_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub metric_type: String,
    pub value: f64,
    pub unit: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SensorDataQuery {
    pub device_id: Option<Uuid>,
    #[serde(alias = "type")]
    pub metric_type: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub device_id: Uuid,
    #[serde(alias = "type")]
    pub metric_type: String,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub interval_minutes: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsPoint {
    pub timestamp: DateTime<Utc>,
    pub average: f64,
    pub min: f64,
    pub max: f64,
    pub count: i64,
}
