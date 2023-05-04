use appstream::{enums::Icon, Collection};
use http_cache_reqwest::{Cache, HttpCache, CacheMode, CACacheManager};
use reqwest::{Client, StatusCode};
use reqwest_middleware::ClientBuilder;
use std::{io::ErrorKind, time::Duration};
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
        let client = ClientBuilder::new(Client::new())
            .with(Cache(HttpCache {
                mode: CacheMode::Default,
                manager: CACacheManager::default(),
                options: None,
            }))
            .build();

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

            match tokio::fs::create_dir_all("_apps/icons").await {
                Ok(_) => {}
                Err(e) => {
                    if e.kind() != ErrorKind::AlreadyExists {
                        tracing::error!("Error creating directory for appstream jsons: {:?}", e);
                        continue;
                    }
                }
            }

            let collection =
                match tokio::task::spawn_blocking(|| -> Result<Collection, std::io::Error> {
                    let collection = Collection::from_gzipped("/tmp/appstream.xml.gz".into())
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

                    for c in &collection.components {
                        let out = std::fs::File::create(format!("_apps/{}.json", c.id))?;
                        serde_json::ser::to_writer(out, &c)?;
                    }

                    Ok(collection)
                })
                .await
                {
                    Ok(r) => match r {
                        Ok(c) => c,
                        Err(e) => {
                            tracing::error!("Error parsing appstream file: {:?}", e);
                            continue;
                        }
                    },
                    Err(_) => continue,
                };

            for c in &collection.components {
                for icon in &c.icons {
                    match icon {
                        Icon::Stock(_) => todo!(),
                        Icon::Cached {
                            path,
                            width,
                            height,
                        } => {
                            let mut dir = String::from("_apps/icons");
                            if let (Some(width), Some(height)) = (width, height) {
                                dir += &format!("/{}x{}", width, height);
                            }

                            match tokio::fs::create_dir_all(&dir).await {
                                Ok(_) => {}
                                Err(e) => {
                                    if e.kind() != ErrorKind::AlreadyExists {
                                        tracing::error!(
                                            "Error creating directory for appstream icons: {:?}",
                                            e
                                        );
                                        continue;
                                    }
                                }
                            }

                            match tokio::fs::File::create(format!(
                                "{}/{}",
                                dir,
                                path.to_string_lossy()
                            ))
                            .await
                            {
                                Ok(_) => {}
                                Err(e) => {
                                    tracing::error!("Error creating appstream icon: {:?}", e);
                                    continue;
                                }
                            }
                        }
                        Icon::Remote { url, width, height } => todo!(),
                        Icon::Local {
                            path,
                            width,
                            height,
                        } => todo!(),
                    }
                }
            }
        }
    });

    application.run_until_stopped().await?;
    Ok(())
}
