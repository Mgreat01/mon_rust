use sqlx::{postgres::PgPoolOptions, PgPool};

pub async fn connect_db() -> Result<PgPool, sqlx::Error> {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL manquant");
    let migration_url = std::env::var("MIGRATION_DATABASE_URL").unwrap_or_else(|_| url.clone());
    let migration_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&migration_url)
        .await?;
    sqlx::migrate!().run(&migration_pool).await?;
    migration_pool.close().await;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&url)
        .await?;
    let (is_superuser, bypasses_rls) = sqlx::query_as::<_, (bool, bool)>(
        "SELECT rolsuper,rolbypassrls FROM pg_roles WHERE rolname=current_user",
    )
    .fetch_one(&pool)
    .await?;
    if is_superuser || bypasses_rls {
        pool.close().await;
        return Err(sqlx::Error::Configuration(
            std::io::Error::other(
                "DATABASE_URL doit utiliser un rôle non-superutilisateur sans BYPASSRLS",
            )
            .into(),
        ));
    }
    Ok(pool)
}
