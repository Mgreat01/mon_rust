use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;

use crate::{
    error::ApiError,
    services::auth_service::decode_token,
};

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub role: String,
}

impl AuthUser {
    pub fn require_role(&self, roles: &[&str]) -> Result<(), ApiError> {
        if roles.contains(&self.role.as_str()) {
            Ok(())
        } else {
            Err(ApiError::forbidden("Permissions insuffisantes"))
        }
    }
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let value = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| ApiError::unauthorized("En-tête Authorization absent"))?;
        let token = value
            .strip_prefix("Bearer ")
            .ok_or_else(|| ApiError::unauthorized("Schéma Bearer attendu"))?;
        let claims = decode_token(token)?;
        Ok(Self {
            user_id: claims.sub,
            tenant_id: claims.tenant_id,
            role: claims.role,
        })
    }
}
