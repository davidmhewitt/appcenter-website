#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    let settings = backend::settings::get_settings().expect("Failed to read settings.");

    let subscriber = backend::telemetry::get_subscriber(settings.clone().debug);
    backend::telemetry::init_subscriber(subscriber);

    let application = backend::startup::Application::build(settings.clone(), None).await?;

    tracing::event!(target: "backend", tracing::Level::INFO, "Listening on http://127.0.0.1:{}/", application.port());

    let worker = appstream_worker::AppstreamWorker::new(settings.redis.uri);
    tokio::spawn(async {
        worker.run_appstream_update().await;
    });

    application.run_until_stopped().await?;
    Ok(())
}
