use flate2::read::GzDecoder;
use libxml::parser::ParserOptions;
use reqwest::StatusCode;
use std::{io::Read, time::Duration};
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
                tracing::error!(
                    "Flatpak remote returned {} for appstream.xml.gz",
                    res.status()
                );
                continue;
            }

            let mut out_file = match tokio::fs::File::create("/tmp/appstream.xml.gz").await {
                Ok(f) => f,
                Err(e) => {
                    tracing::error!(
                        "Error creating temporary file for appstream download: {:?}",
                        e
                    );
                    continue;
                }
            };

            let stream = res.bytes_stream().map(|result| {
                result.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
            });

            let mut read = StreamReader::new(stream);

            match tokio::io::copy(&mut read, &mut out_file).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Error downloading appstream file: {:?}", e);
                    continue;
                }
            }

            match spawn_blocking(|| -> Result<(), std::io::Error> {
                let file = std::fs::File::open("/tmp/appstream.xml.gz")?;

                let mut xml_data = Vec::new();
                let mut decoder = GzDecoder::new(file);
                decoder.read_to_end(&mut xml_data)?;

                let parser = libxml::parser::Parser::default();
                let appstream_xml = parser
                    .parse_string(
                        String::from_utf8(xml_data)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
                    )
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

                let root = match appstream_xml.get_root_element() {
                    Some(r) => r,
                    None => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Couldn't get root element from AppStream XML",
                        ))
                    }
                };

                Ok(())
            })
            .await
            {
                Ok(r) => match r {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("Error parsing appstream file: {:?}", e);
                        continue;
                    }
                },
                Err(_) => continue,
            };
        }
    });

    application.run_until_stopped().await?;
    Ok(())
}
