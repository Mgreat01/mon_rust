use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::dto::sensor_dto::SensorDataResponse;

#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum AppEvent {
    SensorData {
        #[serde(skip)]
        tenant_id: uuid::Uuid,
        data: SensorDataResponse,
    },
    Alert {
        #[serde(skip)]
        tenant_id: uuid::Uuid,
        data: AlertEvent,
    },
}

impl AppEvent {
    pub fn tenant_id(&self) -> uuid::Uuid {
        match self {
            Self::SensorData { tenant_id, .. } | Self::Alert { tenant_id, .. } => *tenant_id,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct AlertEvent {
    pub id: uuid::Uuid,
    pub device_id: uuid::Uuid,
    pub severity: String,
    pub message: String,
}

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub events: broadcast::Sender<AppEvent>,
}
