use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let settings = backend::settings::get_settings().expect("Failed to read settings.");

    let subscriber = backend::telemetry::get_subscriber(settings.clone().debug);
    backend::telemetry::init_subscriber(subscriber);

    let application = backend::startup::Application::build(settings.clone(), None).await?;

    tracing::event!(target: "backend", tracing::Level::INFO, "Listening on http://{}:{}/", settings.application.host, settings.application.port);

    let appstream_worker = appstream_worker::AppstreamWorker::new(settings.redis.uri);
    tokio::spawn(async {
        appstream_worker.run_appstream_update().await;
    });

    application.run_until_stopped().await?;
    Ok(())
}
