use axum::{extract::State, http::StatusCode, Json};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    config::tenant::begin_tenant,
    dto::auth_dto::{AuthResponse, CurrentUserResponse, LoginDto, RegisterDto},
    error::{ApiError, ApiResult},
    middleware::jwt::AuthUser,
    services::auth_service::{
        generate_token, hash_password, verify_password, TOKEN_DURATION_SECONDS,
    },
};

type UserRow = (Uuid, Uuid, String, String, String, bool);

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> ApiResult<(StatusCode, Json<AuthResponse>)> {
    let email = normalize_and_validate(&payload.email, &payload.password, &payload.tenant_name)?;
    let tenant_id = Uuid::new_v4();
    let mut transaction = begin_tenant(&state.db, tenant_id).await?;
    sqlx::query("INSERT INTO tenants (id,name) VALUES ($1,$2)")
        .bind(tenant_id)
        .bind(payload.tenant_name.trim())
        .execute(&mut *transaction)
        .await?;
    let user_id = Uuid::new_v4();
    let role = "ADMIN";
    sqlx::query(
        "INSERT INTO users (id, tenant_id, email, password_hash, role) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(&email)
    .bind(hash_password(&payload.password)?)
    .bind(role)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(auth_response(user_id, tenant_id, email, role)?),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginDto>,
) -> ApiResult<Json<AuthResponse>> {
    let email = payload.email.trim().to_lowercase();
    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id,tenant_id,email,password_hash,role,is_active FROM app_private.user_for_login($1)",
    )
    .bind(email)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::unauthorized("Identifiants invalides"))?;

    if !user.5 || !verify_password(&user.3, &payload.password) {
        return Err(ApiError::unauthorized("Identifiants invalides"));
    }
    Ok(Json(auth_response(user.0, user.1, user.2, &user.4)?))
}

pub async fn me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<CurrentUserResponse>> {
    let mut transaction = begin_tenant(&state.db, auth.tenant_id).await?;
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, String)>(
        "SELECT id, tenant_id, email, role FROM users WHERE id = $1 AND tenant_id = $2 AND is_active",
    )
    .bind(auth.user_id)
    .bind(auth.tenant_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| ApiError::unauthorized("Utilisateur inactif ou introuvable"))?;
    transaction.commit().await?;
    Ok(Json(CurrentUserResponse {
        id: row.0,
        tenant_id: row.1,
        email: row.2,
        role: row.3,
    }))
}

fn normalize_and_validate(email: &str, password: &str, tenant: &str) -> ApiResult<String> {
    let email = email.trim().to_lowercase();
    if !email.contains('@') || email.len() > 255 {
        return Err(ApiError::bad_request("Adresse email invalide"));
    }
    if password.len() < 12 || password.len() > 128 {
        return Err(ApiError::bad_request(
            "Le mot de passe doit contenir entre 12 et 128 caractères",
        ));
    }
    if tenant.trim().is_empty() || tenant.trim().len() > 255 {
        return Err(ApiError::bad_request("Nom de tenant invalide"));
    }
    Ok(email)
}

fn auth_response(id: Uuid, tenant_id: Uuid, email: String, role: &str) -> ApiResult<AuthResponse> {
    Ok(AuthResponse {
        token: generate_token(id, tenant_id, role)?,
        token_type: "Bearer",
        expires_in: TOKEN_DURATION_SECONDS,
        user: CurrentUserResponse {
            id,
            tenant_id,
            email,
            role: role.to_owned(),
        },
    })
}
