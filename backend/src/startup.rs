#[cfg(feature = "cors")]
use actix_cors::Cors;
use actix_files as fs;
use actix_session::config::PersistentSession;
use actix_web::cookie::time::Duration;
use base64::{engine::general_purpose, Engine as _};
use common::settings::DatabaseSettings;
use diesel::{Connection, PgConnection};
use diesel_async::pooled_connection::bb8::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};
use secrecy::ExposeSecret;

const SECS_IN_WEEK: i64 = 60 * 60 * 24 * 7;
pub const MIGRATIONS: EmbeddedMigrations = diesel_migrations::embed_migrations!("migrations/");
pub struct Application {
    server: actix_web::dev::Server,
}

pub async fn async_connection_pool(settings: &DatabaseSettings) -> Pool<AsyncPgConnection> {
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(&settings.url);
    Pool::builder()
        .build(manager)
        .await
        .expect("Unable to build database pool")
}

impl Application {
    pub async fn build(settings: common::settings::Settings) -> Result<Self, std::io::Error> {
        let mut connection = PgConnection::establish(&settings.database.url)
            .expect("Unable to connect to database to run migrations");
        connection
            .run_pending_migrations(MIGRATIONS)
            .expect("Unable to run database migrations");

        let connection_pool = async_connection_pool(&settings.database).await;

        let server = run(connection_pool, settings).await?;

        Ok(Self { server })
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

async fn run(
    db_pool: Pool<AsyncPgConnection>,
    settings: common::settings::Settings,
) -> Result<actix_web::dev::Server, std::io::Error> {
    // Database connection pool application state
    let pool = actix_web::web::Data::new(db_pool);

    // Redis connection pool
    let cfg = deadpool_redis::Config::from_url(settings.clone().redis.uri);
    let redis_pool = cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Cannot create deadpool redis");

    let redis_pool_data = actix_web::web::Data::new(redis_pool);

    let secret_key = actix_web::cookie::Key::from(
        &general_purpose::STANDARD
            .decode(settings.secret.hmac_secret.expose_secret())
            .expect("Couldn't decode base64 HMAC secret"),
    );

    tracing::event!(target: "backend", tracing::Level::INFO, "Listening on http://{}:{}/", &settings.application.host, &settings.application.port);

    let server = actix_web::HttpServer::new(move || {
        #[cfg(feature = "cors")]
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")
            .allowed_methods(vec!["GET", "POST"])
            .supports_credentials()
            .allow_any_header()
            .max_age(3600);

        #[cfg(feature = "cors")]
        return actix_web::App::new()
            .wrap(
                actix_session::SessionMiddleware::builder(
                    actix_session::storage::CookieSessionStore::default(),
                    secret_key.clone(),
                )
                .session_lifecycle(
                    PersistentSession::default().session_ttl(Duration::seconds(SECS_IN_WEEK)),
                )
                .cookie_secure(!settings.debug)
                .build(),
            )
            .wrap(cors)
            .service(crate::routes::health_check)
            .configure(crate::routes::auth_routes_config)
            .configure(crate::routes::apps_routes_config)
            .configure(crate::routes::dashboard_routes_config)
            .service(fs::Files::new("/static/apps", "_apps"))
            .app_data(pool.clone())
            .app_data(redis_pool_data.clone());

        #[cfg(not(feature = "cors"))]
        actix_web::App::new()
            .wrap(
                actix_session::SessionMiddleware::builder(
                    actix_session::storage::CookieSessionStore::default(),
                    secret_key.clone(),
                )
                .session_lifecycle(
                    PersistentSession::default().session_ttl(Duration::seconds(SECS_IN_WEEK)),
                )
                .cookie_secure(!settings.debug)
                .build(),
            )
            .service(crate::routes::health_check)
            .configure(crate::routes::auth_routes_config)
            .configure(crate::routes::apps_routes_config)
            .configure(crate::routes::dashboard_routes_config)
            .service(fs::Files::new("/static/apps", "_apps"))
            .app_data(pool.clone())
            .app_data(redis_pool_data.clone())
    })
    .bind((settings.application.host, settings.application.port))?
    .run();

    Ok(server)
}
