use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// Ouvre une transaction et fixe le tenant avec une portée strictement locale
/// à celle-ci. La valeur est automatiquement oubliée au commit ou au rollback,
/// ce qui évite toute fuite de contexte lors du retour de la connexion au pool.
pub async fn begin_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant_id.to_string())
        .execute(&mut *transaction)
        .await?;
    Ok(transaction)
}
