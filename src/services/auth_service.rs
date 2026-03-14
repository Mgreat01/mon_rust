use argon2::{
    password_hash::{
        PasswordHash,
        PasswordHasher,
        PasswordVerifier,
        SaltString,
    },
    Argon2,
};

use chrono::{Duration, Utc};

use jsonwebtoken::{
    encode,
    EncodingKey,
    Header,
};

use argon2::password_hash::rand_core::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}


pub fn hash_password(
    password: &str,
) -> String {

    let salt =
        SaltString::generate(
            &mut OsRng,
        );

    Argon2::default()
        .hash_password(
            password.as_bytes(),
            &salt,
        )
        .unwrap()
        .to_string()
}

pub fn verify_password(
    hash: &str,
    password: &str,
) -> bool {

    let parsed_hash =
        PasswordHash::new(hash)
            .unwrap();

    Argon2::default()
        .verify_password(
            password.as_bytes(),
            &parsed_hash,
        )
        .is_ok()
}

pub fn generate_token(
    user_id: &str,
) -> String {

    let secret =
        std::env::var(
            "JWT_SECRET",
        )
        .unwrap();

    let claims = Claims {
        sub: user_id.to_string(),
        exp: (
            Utc::now()
                + Duration::hours(24)
        )
        .timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(
            secret.as_bytes(),
        ),
    )
    .unwrap()
}