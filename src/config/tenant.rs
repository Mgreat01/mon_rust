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

#[cfg(test)]
mod tests {
    use super::begin_tenant;
    use sqlx::postgres::PgPoolOptions;
    use uuid::Uuid;

    #[tokio::test]
    async fn la_rls_masque_et_protege_les_appareils_des_autres_tenants() {
        let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
            eprintln!("TEST_DATABASE_URL absent : test RLS ignoré");
            return;
        };
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .expect("connexion à la base de test");
        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("migrations de test");

        let tenant_a = Uuid::new_v4();
        let tenant_b = Uuid::new_v4();
        let device_a = Uuid::new_v4();
        let device_b = Uuid::new_v4();
        seed_tenant_and_device(&pool, tenant_a, device_a).await;
        seed_tenant_and_device(&pool, tenant_b, device_b).await;

        let mut transaction = begin_tenant(&pool, tenant_a).await.expect("tenant A");
        let visible: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM devices ORDER BY id")
            .fetch_all(&mut *transaction)
            .await
            .expect("lecture isolée");
        assert_eq!(visible, vec![device_a]);
        let update = sqlx::query("UPDATE devices SET name='intrusion' WHERE id=$1")
            .bind(device_b)
            .execute(&mut *transaction)
            .await
            .expect("mise à jour filtrée");
        assert_eq!(update.rows_affected(), 0);
        transaction.rollback().await.expect("rollback");

        cleanup(&pool, tenant_a).await;
        cleanup(&pool, tenant_b).await;
    }

    async fn seed_tenant_and_device(pool: &sqlx::PgPool, tenant: Uuid, device: Uuid) {
        let mut transaction = begin_tenant(pool, tenant).await.expect("contexte tenant");
        sqlx::query("INSERT INTO tenants (id,name) VALUES ($1,$2)")
            .bind(tenant)
            .bind(format!("test-{tenant}"))
            .execute(&mut *transaction)
            .await
            .expect("tenant de test");
        sqlx::query("INSERT INTO devices (id,tenant_id,name,api_key_hash) VALUES ($1,$2,$3,'test')")
            .bind(device)
            .bind(tenant)
            .bind(format!("device-{device}"))
            .execute(&mut *transaction)
            .await
            .expect("appareil de test");
        transaction.commit().await.expect("commit du jeu de test");
    }

    async fn cleanup(pool: &sqlx::PgPool, tenant: Uuid) {
        let mut transaction = begin_tenant(pool, tenant).await.expect("contexte de nettoyage");
        sqlx::query("DELETE FROM tenants WHERE id=$1")
            .bind(tenant)
            .execute(&mut *transaction)
            .await
            .expect("nettoyage");
        transaction.commit().await.expect("commit du nettoyage");
    }
}
