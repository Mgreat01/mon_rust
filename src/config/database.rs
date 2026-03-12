use sqlx::PgPool;

pub async fn connect_db() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL manquant");

    PgPool::connect(&url)
        .await
        .expect("Connexion PostgreSQL impossible")
}