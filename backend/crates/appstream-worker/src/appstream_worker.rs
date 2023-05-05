use appstream::{enums::Icon, Collection, TranslatableString};
use deadpool_redis::Pool;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache};
use reqwest::{Client, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use serde::Serialize;
use std::{io::ErrorKind, time::Duration};
use tokio_stream::StreamExt;
use tokio_util::io::StreamReader;

pub struct AppstreamWorker {
    collection: Option<appstream::Collection>,
    redis_pool: Pool,
}

impl AppstreamWorker {
    pub fn new(redis_uri: String) -> Self {
        let cfg = deadpool_redis::Config::from_url(redis_uri);
        Self {
            collection: None,
            redis_pool: cfg
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .expect("Cannot create deadpool redis"),
        }
    }
}

#[derive(Serialize)]
struct Component {
    id: String,
    name: TranslatableString,
    summary: Option<TranslatableString>,
    icons: Vec<Icon>,
}

pub async fn run_appstream_update() {
    let mut interval = tokio::time::interval(Duration::from_secs(30 * 60));
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

        match download_appstream_xml(&client).await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Error downloading appstream xml: {:?}", e);
                continue;
            }
        };

        let collection = match parse_and_extract_appstream_collection().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Error parsing appstream download: {:?}", e);
                continue;
            }
        };

        summarise_apps(&collection).await;
        download_icons(&collection, &client).await;
    }
}

async fn summarise_apps(collection: &Collection) {
    let collection = collection.to_owned();
    match tokio::task::spawn_blocking(move || -> Result<(), std::io::Error> {
        let mut apps = vec![];
        for c in collection
            .components
            .iter()
            .filter(|c| !c.id.0.starts_with("org.gnome."))
            .collect::<Vec<&appstream::Component>>()
        {
            apps.push(Component {
                id: c.id.0.to_owned(),
                name: c.name.to_owned(),
                summary: c.summary.to_owned(),
                icons: c.icons.to_owned(),
            });
        }

        let out = std::fs::File::create("_apps/summary.json")?;

        serde_json::ser::to_writer(out, &apps)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        Ok(())
    })
    .await
    {
        Ok(r) => {
            if let Err(e) = r {
                tracing::error!("Error writing apps summary: {:?}", e);
            }
        }
        Err(e) => tracing::error!("Error writing apps summary: {:?}", e),
    }
}

async fn download_icons(collection: &Collection, client: &ClientWithMiddleware) {
    for c in &collection.components {
        for icon in &c.icons {
            // TODO: Do we need to handle other icon types?
            if let Icon::Cached {
                path,
                width,
                height,
            } = icon
            {
                let mut dir = String::from("icons");
                if let (Some(width), Some(height)) = (width, height) {
                    dir += &format!("/{}x{}", width, height);
                }

                let res = match client
                    .get(format!(
                        "https://flatpak.elementary.io/repo/appstream/x86_64/{}/{}",
                        dir,
                        path.to_string_lossy()
                    ))
                    .send()
                    .await
                {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                if res.status() != StatusCode::OK {
                    tracing::error!("Flatpak remote returned {} for icon download", res.status(),);
                    continue;
                }

                if let Err(e) = tokio::fs::create_dir_all(format!("_apps/{}", dir)).await {
                    tracing::error!("Error creating directory for appstream icons: {:?}", e);
                    continue;
                }

                let mut icon_file = match tokio::fs::File::create(format!(
                    "_apps/{}/{}",
                    dir,
                    path.to_string_lossy()
                ))
                .await
                {
                    Ok(f) => f,
                    Err(e) => {
                        tracing::error!("Error creating appstream icon: {:?}", e);
                        continue;
                    }
                };

                let stream = res.bytes_stream().map(|result| {
                    result.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
                });

                let mut read = StreamReader::new(stream);

                if let Err(e) = tokio::io::copy(&mut read, &mut icon_file).await {
                    tracing::error!("Error downloading appstream file: {:?}", e);
                    continue;
                }
            }
        }
    }
}

async fn download_appstream_xml(client: &ClientWithMiddleware) -> Result<(), std::io::Error> {
    let res = client
        .get("https://flatpak.elementary.io/repo/appstream/x86_64/appstream.xml.gz")
        .send()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    if res.status() != StatusCode::OK {
        tracing::error!(
            "Flatpak remote returned {} for appstream.xml.gz",
            res.status()
        );

        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Flatpak remote returned {} for appstream.xml.gz",
                res.status()
            ),
        );
    }

    let mut out_file = tokio::fs::File::create("/tmp/appstream.xml.gz")
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let stream = res
        .bytes_stream()
        .map(|result| result.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err)));

    let mut read = StreamReader::new(stream);

    tokio::io::copy(&mut read, &mut out_file).await?;

    Ok(())
}

async fn parse_and_extract_appstream_collection() -> Result<Collection, std::io::Error> {
    match tokio::fs::create_dir("_apps").await {
        Ok(()) => Ok(()),
        Err(e) => {
            if e.kind() == ErrorKind::AlreadyExists {
                Ok(())
            } else {
                Err(std::io::Error::new(std::io::ErrorKind::Other, e))
            }
        }
    }?;

    match tokio::task::spawn_blocking(|| -> Result<Collection, std::io::Error> {
        let collection = Collection::from_gzipped("/tmp/appstream.xml.gz".into())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        for c in &collection.components {
            let out = match std::fs::File::create(format!("_apps/{}.json", c.id)) {
                Ok(f) => f,
                Err(_) => continue,
            };

            if serde_json::ser::to_writer(out, &c).is_err() {
                continue;
            }
        }

        Ok(collection)
    })
    .await
    {
        Ok(r) => r,
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
}
