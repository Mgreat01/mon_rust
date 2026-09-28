use sqlx::{postgres::PgPoolOptions, PgPool};

pub async fn connect_db() -> Result<PgPool, sqlx::Error> {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL manquant");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&url)
        .await?;

    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}
