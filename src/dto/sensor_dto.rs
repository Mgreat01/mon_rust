use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateSensorDataDto {
    pub device_id: String,
    pub value: f64,
    pub r#type: String,
}

#[derive(Debug, Serialize)]
pub struct SensorDataResponse {
    pub id: i64,
    pub device_id: String,
    pub value: f64,
    pub r#type: String,
}