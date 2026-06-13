use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct SensorData {
    pub id: i64,
    pub device_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub r#type: String,
}