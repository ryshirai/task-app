use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use sqlx::{Pool, Postgres};

pub async fn seed_data(pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password("admin123".as_bytes(), &salt)
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?
        .to_string();

    let org_id: (i32,) = sqlx::query_as("INSERT INTO organizations (name) VALUES ('Default Organization') ON CONFLICT DO NOTHING RETURNING id")
        .fetch_optional(pool)
        .await?
        .unwrap_or_else(|| (1,));

    sqlx::query("INSERT INTO users (organization_id, name, username, password_hash, role) VALUES ($1, $2, $3, $4, $5) 
                 ON CONFLICT (username) DO UPDATE SET password_hash = $4")
        .bind(org_id.0)
        .bind("Administrator")
        .bind("admin")
        .bind(password_hash)
        .bind("admin")
        .execute(pool)
        .await?;

    let _ = sqlx::query("UPDATE tasks SET organization_id = $1 WHERE organization_id IS NULL")
        .bind(org_id.0)
        .execute(pool)
        .await;
    let _ =
        sqlx::query("UPDATE daily_reports SET organization_id = $1 WHERE organization_id IS NULL")
            .bind(org_id.0)
            .execute(pool)
            .await;
    let _ =
        sqlx::query("UPDATE activity_logs SET organization_id = $1 WHERE organization_id IS NULL")
            .bind(org_id.0)
            .execute(pool)
            .await;

    println!("Admin user verified/updated.");
    Ok(())
}
