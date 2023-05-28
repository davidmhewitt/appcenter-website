use argon2::password_hash::rand_core::{OsRng, RngCore};
use diesel::{Connection, ExpressionMethods, PgConnection};
use diesel_async::{pooled_connection::bb8::Pool, AsyncPgConnection, RunQueryDsl};
use diesel_migrations::MigrationHarness;

use crate::startup::{async_connection_pool, MIGRATIONS};

#[inline]
pub async fn db_pool() -> Pool<AsyncPgConnection> {
    let settings = common::settings::get_settings().expect("Failed to read settings.");

    let mut connection = PgConnection::establish(&settings.database.url)
        .expect("Unable to connect to database to run migrations");
    connection
        .run_pending_migrations(MIGRATIONS)
        .expect("Unable to run database migrations");

    async_connection_pool(&settings.database).await
}

pub async fn create_user(con: &mut AsyncPgConnection, active: bool) -> anyhow::Result<uuid::Uuid> {
    use common::schema::users::dsl::*;

    let new_user_id = uuid::Uuid::new_v4();

    let random_email: String = {
        let mut buff = [0_u8; 8];
        OsRng.fill_bytes(&mut buff);
        hex::encode(buff)
    };

    diesel::insert_into(users)
        .values((
            id.eq(new_user_id),
            email.eq(format!("{}@example.com", random_email)),
            is_active.eq(active),
            is_admin.eq(false),
        ))
        .execute(con)
        .await?;

    Ok(new_user_id)
}
