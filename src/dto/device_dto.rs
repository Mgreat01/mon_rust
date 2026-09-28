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
    pub api_key_version: i32,
    pub credential_status: &'static str,
    pub credential_updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CreatedDeviceResponse {
    #[serde(flatten)]
    pub device: DeviceResponse,
    pub api_key: String,
}

#[derive(Debug, Serialize)]
pub struct RotatedCredentialResponse {
    pub device_id: Uuid,
    pub api_key: String,
    pub api_key_version: i32,
    pub issued_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
