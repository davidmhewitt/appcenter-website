use crate::component_versioning::get_latest_component_version;

use appstream::{builders::ReleaseBuilder, enums::Icon, Collection, Component};
use chrono::TimeZone;
use deadpool_redis::Pool;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache};
use reqwest::{Client, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use semver::Version;
use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    time::Duration,
};
use tokio_stream::StreamExt;
use tokio_util::io::StreamReader;

pub struct AppstreamWorker {
    latest_versions: HashMap<String, Version>,
    redis_pool: Pool,
}

impl AppstreamWorker {
    pub fn new(redis_uri: String) -> Self {
        let cfg = deadpool_redis::Config::from_url(redis_uri);
        Self {
            latest_versions: HashMap::new(),
            redis_pool: cfg
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .expect("Cannot create deadpool redis"),
        }
    }

    pub async fn run_appstream_update(mut self) {
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

            match self.download_appstream_xml(&client).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Error downloading appstream xml: {:?}", e);
                    continue;
                }
            };

            let collection = match self.parse_and_extract_appstream_collection().await {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Error parsing appstream download: {:?}", e);
                    continue;
                }
            };

            self.summarise_apps(&collection).await;
            self.download_icons(&collection, &client).await;
        }
    }

    async fn summarise_apps(&mut self, collection: &Collection) {
        let first_update = self.latest_versions.is_empty();

        let mut redis_con = self
            .redis_pool
            .get()
            .await
            .map_err(|e| {
                tracing::error!("Error getting redis connection: {}", e);
            })
            .expect("Redis connection cannot be gotten.");

        let collection = collection.to_owned();
        let mut collection = collection
            .components
            .iter()
            .filter(|c| !c.id.0.starts_with("org.gnome."))
            .collect::<Vec<&Component>>();

        if first_update {
            // Sort recently released components first
            collection.sort_unstable_by(|a, b| {
                b.releases
                    .first()
                    .unwrap_or(
                        &ReleaseBuilder::new("0.0.0")
                            .date(chrono::Utc.timestamp_opt(0, 0).unwrap())
                            .build(),
                    )
                    .date
                    .unwrap_or(chrono::Utc.timestamp_opt(0, 0).unwrap())
                    .cmp(
                        &a.releases
                            .first()
                            .unwrap_or(
                                &ReleaseBuilder::new("0.0.0")
                                    .date(chrono::Utc.timestamp_opt(0, 0).unwrap())
                                    .build(),
                            )
                            .date
                            .unwrap_or(chrono::Utc.timestamp_opt(0, 0).unwrap()),
                    )
            });

            if let Err(e) = deadpool_redis::redis::Cmd::del("appstream_worker/recently_updated")
                .query_async::<_, i32>(&mut redis_con)
                .await
            {
                tracing::warn!("Error removing recently updated from redis: {}", e);
            }

            for i in 0..20 {
                if let Some(c) = collection.get(i) {
                    if let Err(e) = deadpool_redis::redis::Cmd::rpush(
                        "appstream_worker/recently_updated",
                        serde_json::to_string(c).unwrap(),
                    )
                    .query_async::<_, i32>(&mut redis_con)
                    .await
                    {
                        tracing::warn!("Error adding recently updated to redis: {}", e);
                    }
                } else {
                    break;
                }
            }

            // Sort recently released components first
            collection.sort_unstable_by(|a, b| {
                b.releases
                    .last()
                    .unwrap_or(
                        &ReleaseBuilder::new("0.0.0")
                            .date(chrono::Utc.timestamp_opt(0, 0).unwrap())
                            .build(),
                    )
                    .date
                    .unwrap_or(chrono::Utc.timestamp_opt(0, 0).unwrap())
                    .cmp(
                        &a.releases
                            .last()
                            .unwrap_or(
                                &ReleaseBuilder::new("0.0.0")
                                    .date(chrono::Utc.timestamp_opt(0, 0).unwrap())
                                    .build(),
                            )
                            .date
                            .unwrap_or(chrono::Utc.timestamp_opt(0, 0).unwrap()),
                    )
            });

            if let Err(e) = deadpool_redis::redis::Cmd::del("appstream_worker/recently_added")
                .query_async::<_, i32>(&mut redis_con)
                .await
            {
                tracing::warn!("Error removing recently added from redis: {}", e);
            }

            for i in 0..20 {
                if let Some(c) = collection.get(i) {
                    if let Err(e) = deadpool_redis::redis::Cmd::rpush(
                        "appstream_worker/recently_added",
                        serde_json::to_string(c).unwrap(),
                    )
                    .query_async::<_, i32>(&mut redis_con)
                    .await
                    {
                        tracing::warn!("Error adding recently added to redis: {}", e);
                    }
                } else {
                    break;
                }
            }
        }

        for c in collection {
            if !first_update {
                match self.latest_versions.get(&c.id.0) {
                    Some(old_ver) => {
                        if let Some(new_ver) = get_latest_component_version(c) {
                            if new_ver.gt(old_ver) {
                                if let Err(e) = deadpool_redis::redis::Cmd::lpush(
                                    "appstream_worker/recently_updated",
                                    serde_json::to_string(c).unwrap(),
                                )
                                .query_async::<_, i32>(&mut redis_con)
                                .await
                                {
                                    tracing::warn!("Error adding recently updated to redis: {}", e);
                                }

                                if let Err(e) = deadpool_redis::redis::Cmd::ltrim(
                                    "appstream_worker/recently_updated",
                                    0,
                                    19,
                                )
                                .query_async::<_, String>(&mut redis_con)
                                .await
                                {
                                    tracing::warn!(
                                        "Error truncating recently updated in redis: {}",
                                        e
                                    );
                                }
                            }
                        }
                    }
                    None => {
                        if let Err(e) = deadpool_redis::redis::Cmd::lpush(
                            "appstream_worker/recently_added",
                            serde_json::to_string(c).unwrap(),
                        )
                        .query_async::<_, i32>(&mut redis_con)
                        .await
                        {
                            tracing::warn!("Error adding recently updated to redis: {}", e);
                        }

                        if let Err(e) = deadpool_redis::redis::Cmd::ltrim(
                            "appstream_worker/recently_added",
                            0,
                            19,
                        )
                        .query_async::<_, String>(&mut redis_con)
                        .await
                        {
                            tracing::warn!("Error truncating recently addedd in redis: {}", e);
                        }
                    }
                }
            }

            if let Some(v) = get_latest_component_version(c) {
                self.latest_versions.insert(c.id.0.to_owned(), v);
            }
        }
    }

    async fn download_icons(&self, collection: &Collection, client: &ClientWithMiddleware) {
        for c in &collection.components {
            for icon in &c.icons {
                // TODO: Do we need to handle other icon types?
                if let Icon::Cached {
                    path,
                    width,
                    height,
                } = icon
                {
                    if let Err(e) = download_icon(width, height, client, path).await {
                        tracing::warn!("Error downloading appstream icon: {}", e);
                    }
                }
            }
        }
    }

    async fn download_appstream_xml(&self, client: &ClientWithMiddleware) -> Result<(), Error> {
        let res = client
            .get("https://flatpak.elementary.io/repo/appstream/x86_64/appstream.xml.gz")
            .send()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        if res.status() != StatusCode::OK {
            tracing::error!(
                "Flatpak remote returned {} for appstream.xml.gz",
                res.status()
            );

            Error::new(
                ErrorKind::Other,
                format!(
                    "Flatpak remote returned {} for appstream.xml.gz",
                    res.status()
                ),
            );
        }

        let mut out_file = tokio::fs::File::create("/tmp/appstream.xml.gz")
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        let stream = res
            .bytes_stream()
            .map(|result| result.map_err(|err| Error::new(ErrorKind::Other, err)));

        let mut read = StreamReader::new(stream);

        tokio::io::copy(&mut read, &mut out_file).await?;

        Ok(())
    }

    async fn parse_and_extract_appstream_collection(&self) -> Result<Collection, Error> {
        match tokio::fs::create_dir("_apps").await {
            Ok(()) => Ok(()),
            Err(e) => {
                if e.kind() == ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(Error::new(ErrorKind::Other, e))
                }
            }
        }?;

        match tokio::task::spawn_blocking(|| -> Result<Collection, Error> {
            let collection = Collection::from_gzipped("/tmp/appstream.xml.gz".into())
                .map_err(|e| Error::new(ErrorKind::Other, e))?;

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
            Err(e) => Err(Error::new(ErrorKind::Other, e)),
        }
    }
}

async fn download_icon(
    width: &Option<u32>,
    height: &Option<u32>,
    client: &ClientWithMiddleware,
    path: &std::path::PathBuf,
) -> Result<(), Error> {
    let mut dir = String::from("icons");
    if let (Some(width), Some(height)) = (width, height) {
        dir += &format!("/{}x{}", width, height);
    }

    let res = client
        .get(format!(
            "https://flatpak.elementary.io/repo/appstream/x86_64/{}/{}",
            dir,
            path.to_string_lossy()
        ))
        .send()
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    if res.status() != StatusCode::OK {
        return Err(Error::new(
            ErrorKind::Other,
            format!("Flatpak remote returned {} for icon download", res.status()),
        ));
    }

    if let Err(e) = tokio::fs::create_dir_all(format!("_apps/{}", dir)).await {
        return Err(Error::new(
            ErrorKind::Other,
            format!("Error creating directory for appstream icons: {:?}", e),
        ));
    }

    let mut icon_file =
        tokio::fs::File::create(format!("_apps/{}/{}", dir, path.to_string_lossy()))
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

    let stream = res
        .bytes_stream()
        .map(|result| result.map_err(|err| Error::new(ErrorKind::Other, err)));
    let mut read = StreamReader::new(stream);

    if let Err(e) = tokio::io::copy(&mut read, &mut icon_file).await {
        return Err(Error::new(
            ErrorKind::Other,
            format!("Error downloading icon: {:?}", e),
        ));
    }

    Ok(())
}
