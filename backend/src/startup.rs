use std::io::ErrorKind;

use actix_files as fs;
use actix_session::config::PersistentSession;
use actix_web::cookie::time::Duration;
use git_worker::GitWorker;
use secrecy::ExposeSecret;

const SECS_IN_WEEK: i64 = 60 * 60 * 24 * 7;

pub struct Application {
    port: u16,
    server: actix_web::dev::Server,
}

impl Application {
    pub async fn build(
        settings: crate::settings::Settings,
        test_pool: Option<sqlx::postgres::PgPool>,
    ) -> Result<Self, std::io::Error> {
        let connection_pool = if let Some(pool) = test_pool {
            pool
        } else {
            get_connection_pool(&settings.database).await
        };

        sqlx::migrate!()
            .run(&connection_pool)
            .await
            .expect("Failed to migrate the database.");

        let address = format!(
            "{}:{}",
            settings.application.host, settings.application.port
        );

        let listener = std::net::TcpListener::bind(&address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, connection_pool, settings).await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub async fn get_connection_pool(
    settings: &crate::settings::DatabaseSettings,
) -> sqlx::postgres::PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(settings.connect_to_db())
}

async fn run(
    listener: std::net::TcpListener,
    db_pool: sqlx::postgres::PgPool,
    settings: crate::settings::Settings,
) -> Result<actix_web::dev::Server, std::io::Error> {
    // Database connection pool application state
    let pool = actix_web::web::Data::new(db_pool);

    // Redis connection pool
    let cfg = deadpool_redis::Config::from_url(settings.clone().redis.uri);
    let redis_pool = cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Cannot create deadpool redis");

    let redis_pool_data = actix_web::web::Data::new(redis_pool);

    let git_worker = GitWorker::new(
        settings.github.local_repo_path,
        settings.github.reviews_url,
        settings.github.username,
        settings.github.access_token,
    )
    .map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;

    let git_worker_data = actix_web::web::Data::new(git_worker);

    let secret_key =
        actix_web::cookie::Key::from(settings.secret.hmac_secret.expose_secret().as_bytes());
    let server = actix_web::HttpServer::new(move || {
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
            .service(fs::Files::new("/static/apps", "_apps"))
            .service(fs::Files::new("/", "_static").index_file("index.html"))
            .app_data(pool.clone())
            .app_data(redis_pool_data.clone())
            .app_data(git_worker_data.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
