use crate::{
    appstream_collection_sorters,
    appstream_version_utils::{get_latest_component_version, get_new_and_updated_apps},
    redis_utils, RECENTLY_ADDED_REDIS_KEY, RECENTLY_UPDATED_REDIS_KEY,
};

use appstream::{
    enums::{Bundle, Icon},
    AppId, Collection, Component, TranslatableString,
};
use deadpool_redis::Pool;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache};
use reqwest::{Client, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    path::Path,
    time::Duration,
};
use tokio_stream::StreamExt;
use tokio_util::io::StreamReader;

#[derive(Serialize, Deserialize)]
pub struct ComponentSummary {
    id: AppId,
    name: TranslatableString,
    summary: Option<TranslatableString>,
    icons: Vec<Icon>,
}

impl From<Component> for ComponentSummary {
    fn from(value: Component) -> Self {
        Self {
            id: value.id,
            name: value.name,
            summary: value.summary,
            icons: value.icons,
        }
    }
}

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

            let mut collection = match self.parse_and_extract_appstream_collection().await {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Error parsing appstream download: {:?}", e);
                    continue;
                }
            };

            self.summarise_apps(&mut collection).await;
            self.download_icons(&collection, &client).await;
        }
    }

    async fn summarise_apps(&mut self, collection: &mut Vec<Component>) {
        let first_update = self.latest_versions.is_empty();

        let mut redis_con = self
            .redis_pool
            .get()
            .await
            .map_err(|e| {
                tracing::error!("Error getting redis connection: {}", e);
            })
            .expect("Redis connection cannot be gotten.");

        if first_update {
            collection.sort_unstable_by(|a, b| {
                appstream_collection_sorters::sort_newly_released_components_first(a, b)
            });

            redis_utils::del(&mut redis_con, RECENTLY_UPDATED_REDIS_KEY).await;

            for i in 0..20 {
                if let Some(c) = collection.get(i) {
                    let c: ComponentSummary = c.clone().into();
                    redis_utils::rpush(
                        &mut redis_con,
                        RECENTLY_UPDATED_REDIS_KEY,
                        &serde_json::to_string(&c).unwrap(),
                    )
                    .await;
                } else {
                    break;
                }
            }

            collection.sort_unstable_by(|a, b| {
                appstream_collection_sorters::sort_recent_initial_release_components_first(a, b)
            });

            redis_utils::del(&mut redis_con, RECENTLY_ADDED_REDIS_KEY).await;

            for i in 0..20 {
                if let Some(c) = collection.get(i) {
                    let c: ComponentSummary = c.clone().into();
                    redis_utils::rpush(
                        &mut redis_con,
                        RECENTLY_ADDED_REDIS_KEY,
                        &serde_json::to_string(&c).unwrap(),
                    )
                    .await;
                } else {
                    break;
                }
            }
        }

        if !first_update {
            let (new_apps, updated_apps) =
                get_new_and_updated_apps(&self.latest_versions, collection);
            for c in new_apps {
                let c: ComponentSummary = c.clone().into();
                redis_utils::lpush(
                    &mut redis_con,
                    RECENTLY_ADDED_REDIS_KEY,
                    &serde_json::to_string(&c).unwrap(),
                )
                .await;

                redis_utils::ltrim(&mut redis_con, RECENTLY_ADDED_REDIS_KEY, 0, 19).await;
            }

            for c in updated_apps {
                let c: ComponentSummary = c.clone().into();
                redis_utils::lpush(
                    &mut redis_con,
                    RECENTLY_UPDATED_REDIS_KEY,
                    &serde_json::to_string(&c).unwrap(),
                )
                .await;

                redis_utils::ltrim(&mut redis_con, RECENTLY_UPDATED_REDIS_KEY, 0, 19).await;
            }
        }

        for c in collection {
            if let Some(v) = get_latest_component_version(c) {
                self.latest_versions.insert(c.id.0.to_owned(), v);
            }
        }
    }

    async fn download_icons(&self, collection: &Vec<Component>, client: &ClientWithMiddleware) {
        for c in collection {
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

    async fn parse_and_extract_appstream_collection(&self) -> Result<Vec<Component>, Error> {
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

        match tokio::task::spawn_blocking(|| -> Result<Vec<Component>, Error> {
            let collection = Collection::from_gzipped("/tmp/appstream.xml.gz".into())
                .map_err(|e| Error::new(ErrorKind::Other, e))?;

            let components: Vec<Component> = collection
                .components
                .to_owned()
                .into_iter()
                .filter(|c| !c.id.0.starts_with("org.gnome."))
                .filter(|c| match c.bundles.first().unwrap() {
                    Bundle::Flatpak {
                        runtime: _,
                        sdk: _,
                        reference,
                    } => return reference.ends_with("/stable"),
                    _ => return true,
                })
                .collect();

            for c in &components {
                let out = match std::fs::File::create(format!("_apps/{}.json", c.id)) {
                    Ok(f) => f,
                    Err(_) => continue,
                };

                if serde_json::ser::to_writer(out, &c).is_err() {
                    continue;
                }
            }

            Ok(components)
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
    path: &Path,
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
