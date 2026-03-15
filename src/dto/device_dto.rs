use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateDeviceDto {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeviceResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}