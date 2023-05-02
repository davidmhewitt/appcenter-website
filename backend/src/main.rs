use reqwest::StatusCode;
use std::time::Duration;
use tokio::task::spawn_blocking;
use tokio_stream::StreamExt;
use tokio_util::io::StreamReader;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    let settings = backend::settings::get_settings().expect("Failed to read settings.");

    let subscriber = backend::telemetry::get_subscriber(settings.clone().debug);
    backend::telemetry::init_subscriber(subscriber);

    let application = backend::startup::Application::build(settings, None).await?;

    tracing::event!(target: "backend", tracing::Level::INFO, "Listening on http://127.0.0.1:{}/", application.port());

    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(60 * 30));
        let client = reqwest::Client::new();
        loop {
            interval.tick().await;
            tracing::info!("Updating AppStream info");
            let res = match client
                .get("https://flatpak.elementary.io/repo/appstream/x86_64/appstream.xml.gz")
                .send()
                .await
            {
                Ok(r) => r,
                Err(_) => continue,
            };

            if res.status() != StatusCode::OK {
                continue;
            }

            let mut out_file = match tokio::fs::File::create("/tmp/appstream.xml.gz").await {
                Ok(f) => f,
                Err(_) => continue,
            };

            let stream = res.bytes_stream().map(|result| {
                result.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
            });

            let mut read = StreamReader::new(stream);

            match tokio::io::copy(&mut read, &mut out_file).await {
                Ok(_) => {}
                Err(_) => continue,
            }

            match spawn_blocking(|| -> Result<(), std::io::Error> {
                let parser = libxml::parser::Parser::default();
                let appstream_xml = parser
                    .parse_file("/tmp/appstream.xml.gz")
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

                Ok(())
            }).await {
                Ok(r) => match r {
                    Ok(_) => {},
                    Err(_) => continue,
                },
                Err(_) => continue,
            };
        }
    });

    application.run_until_stopped().await?;
    Ok(())
}
