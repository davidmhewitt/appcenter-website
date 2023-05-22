use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let settings = common::settings::get_settings().expect("Failed to read settings.");

    let subscriber = common::telemetry::get_subscriber(settings.clone().debug);
    common::telemetry::init_subscriber(subscriber);

    let application = backend::startup::Application::build(settings.clone()).await?;

    application.run_until_stopped().await?;
    Ok(())
}
