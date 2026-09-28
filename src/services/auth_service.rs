use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

pub const TOKEN_DURATION_SECONDS: i64 = 86_400;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub tenant_id: Uuid,
    pub role: String,
    pub iat: usize,
    pub exp: usize,
}

pub fn hash_password(password: &str) -> ApiResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| {
            tracing::error!(%error, "échec du hash Argon2");
            ApiError::internal()
        })
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    PasswordHash::new(hash).ok().is_some_and(|parsed| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

pub fn generate_token(user_id: Uuid, tenant_id: Uuid, role: &str) -> ApiResult<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        tenant_id,
        role: role.to_owned(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::seconds(TOKEN_DURATION_SECONDS)).timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret()?.as_bytes()),
    )
    .map_err(|error| {
        tracing::error!(%error, "échec de création du JWT");
        ApiError::internal()
    })
}

pub fn decode_token(token: &str) -> ApiResult<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret()?.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
    .map_err(|_| ApiError::unauthorized("Jeton absent, invalide ou expiré"))
}

fn jwt_secret() -> ApiResult<String> {
    let secret = std::env::var("JWT_SECRET").map_err(|_| {
        ApiError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ERROR",
            "JWT_SECRET est absent",
        )
    })?;
    if secret.len() < 32 {
        return Err(ApiError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ERROR",
            "JWT_SECRET doit contenir au moins 32 caractères",
        ));
    }
    Ok(secret)
}

#[cfg(test)]
mod tests {
    use super::{hash_password, verify_password};

    #[test]
    fn hash_password_ne_conserve_pas_le_secret() {
        let password = "mot-de-passe-solide";
        let hash = hash_password(password).expect("hash valide");
        assert_ne!(hash, password);
        assert!(verify_password(&hash, password));
        assert!(!verify_password(&hash, "mauvais-mot-de-passe"));
    }
}
