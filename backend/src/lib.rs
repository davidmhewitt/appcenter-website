use diesel::{r2d2, PgConnection};
use once_cell::sync::Lazy;

pub mod extractors;
pub mod routes;
pub mod startup;
pub mod types;
pub mod utils;

pub static ENV: once_cell::sync::Lazy<minijinja::Environment<'static>> =
    once_cell::sync::Lazy::new(|| {
        let mut env = minijinja::Environment::new();
        env.set_source(minijinja::Source::from_path("templates"));
        env
    });

pub fn sync_connection_pool() -> r2d2::Pool<r2d2::ConnectionManager<PgConnection>> {
    let settings = common::settings::get_settings().expect("Failed to read settings.");

    let manager = r2d2::ConnectionManager::<PgConnection>::new(settings.database.url);

    r2d2::Pool::builder().max_size(10).build(manager).unwrap()
}

pub static SYNC_PG_POOL: Lazy<r2d2::Pool<r2d2::ConnectionManager<PgConnection>>> =
    Lazy::new(sync_connection_pool);
