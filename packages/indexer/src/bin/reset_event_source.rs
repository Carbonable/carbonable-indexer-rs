use carbonable_domain::infrastructure::app::configure_application;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let configuration = configure_application().await?;
    let pool = sqlx::PgPool::connect(&configuration.database_uri).await?;

    sqlx::query("TRUNCATE TABLE customer_token CASCADE")
        .execute(&pool)
        .await?;
    sqlx::query!("TRUNCATE TABLE customer_yield CASCADE")
        .execute(&pool)
        .await?;
    sqlx::query!("TRUNCATE TABLE global_yield CASCADE")
        .execute(&pool)
        .await?;
    sqlx::query!("TRUNCATE TABLE snapshot CASCADE")
        .execute(&pool)
        .await?;
    sqlx::query!("TRUNCATE TABLE provision CASCADE")
        .execute(&pool)
        .await?;

    Ok(())
}
