use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateDeviceDto {
    pub name: String,
    pub description: Option<String>,
    pub device_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDeviceDto {
    pub name: Option<String>,
    pub description: Option<String>,
    pub device_type: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeviceResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub device_type: String,
    pub status: String,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CreatedDeviceResponse {
    #[serde(flatten)]
    pub device: DeviceResponse,
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
