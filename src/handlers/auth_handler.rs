use axum::{
    extract::State,
    Json,
};

use uuid::Uuid;

use crate::{
    app_state::AppState,
    dto::auth_dto::*,
    services::auth_service::*,
};

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> Json<AuthResponse> {

    let id =
        Uuid::new_v4();

    let hash =
        hash_password(
            &payload.password,
        );

    let result = sqlx::query(
    "INSERT INTO users (id,email,password_hash,role)
     VALUES ($1,$2,$3,'USER')"
)
.bind(id)
.bind(&payload.email)
.bind(hash)
.execute(&state.db)
.await;

match result {
    Ok(_) => {}
    Err(err) => {
        println!("Erreur SQL : {:?}", err);

        return Json(AuthResponse {
            token: "EMAIL_ALREADY_EXISTS".to_string(),
        });
    }
}

    let token =
        generate_token(
            &id.to_string(),
        );

    Json(AuthResponse {
        token,
    })
}

use sqlx::Row;

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginDto>,
) -> Json<AuthResponse> {

    let row =
        sqlx::query(
            "
            SELECT
                id,
                password_hash
            FROM users
            WHERE email = $1
            ",
        )
        .bind(&payload.email)
        .fetch_one(&state.db)
        .await
        .unwrap();

    let id: Uuid =
        row.get("id");

    let hash: String =
        row.get(
            "password_hash",
        );

    if !verify_password(
        &hash,
        &payload.password,
    ) {
        panic!("Bad credentials");
    }

    let token =
        generate_token(
            &id.to_string(),
        );

    Json(AuthResponse {
        token,
    })
}