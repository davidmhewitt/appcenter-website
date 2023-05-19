use async_once_cell::OnceCell;
use diesel::{Connection, PgConnection};
use diesel_async::{
    pooled_connection::{bb8::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};
use diesel_migrations::MigrationHarness;

static POOL: OnceCell<Pool<AsyncPgConnection>> = OnceCell::new();

async fn create_db_pool() -> Pool<AsyncPgConnection> {
    dotenv::dotenv().ok();

    let settings = backend::settings::get_settings().expect("Failed to read settings.");

    let mut connection = PgConnection::establish(&settings.database.url)
        .expect("Unable to connect to database to run migrations");
    connection
        .run_pending_migrations(backend::startup::MIGRATIONS)
        .expect("Unable to run database migrations");

    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(&settings.database.url);
    Pool::builder()
        .build(manager)
        .await
        .expect("Unable to build database pool")
}

pub(crate) async fn get_db_pool() -> &'static Pool<AsyncPgConnection> {
    POOL.get_or_init(create_db_pool()).await
}
