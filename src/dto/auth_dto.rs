use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct RegisterDto {
    pub tenant_name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginDto {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    pub user: CurrentUserResponse,
}

#[derive(Debug, Serialize)]
pub struct CurrentUserResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub role: String,
}
