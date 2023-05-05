use appstream::{enums::Icon, Collection, Component};
use deadpool_redis::Pool;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache};
use reqwest::{Client, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use semver::Version;
use std::{collections::HashMap, io::ErrorKind, time::Duration};
use tokio_stream::StreamExt;
use tokio_util::io::StreamReader;

pub struct AppstreamWorker {
    latest_versions: HashMap<String, Version>,
    recently_updated: Vec<Component>,
    recently_added: Vec<Component>,
    redis_pool: Pool,
}

impl AppstreamWorker {
    pub fn new(redis_uri: String) -> Self {
        let cfg = deadpool_redis::Config::from_url(redis_uri);
        Self {
            latest_versions: HashMap::new(),
            recently_added: vec![],
            recently_updated: vec![],
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

    fn get_latest_component_version(component: &Component) -> Option<Version> {
        let mut versions = component.releases.to_owned();
        versions.sort_unstable_by(|a, b| {
            if let (Some(a), Some(b)) = (a.date, b.date) {
                if a != b {
                    return b.cmp(&a);
                }
            }

            let a_ver = lenient_semver::parse(&a.version)
                .unwrap_or_else(|_| lenient_semver::parse("0.0.0").unwrap());
            let b_ver = lenient_semver::parse(&b.version)
                .unwrap_or_else(|_| lenient_semver::parse("0.0.0").unwrap());

            b_ver.cmp(&a_ver)
        });

        if let Some(v) = versions.first() {
            if let Ok(v) = lenient_semver::parse(&v.version) {
                return Some(v);
            }
        }

        None
    }

    async fn summarise_apps(&mut self, collection: &Collection) {
        let first_update = self.latest_versions.is_empty();

        let collection = collection.to_owned();
        for c in collection
            .components
            .iter()
            .filter(|c| !c.id.0.starts_with("org.gnome."))
            .collect::<Vec<&Component>>()
        {
            if !first_update {
                match self.latest_versions.get(&c.id.0) {
                    Some(old_ver) => {
                        if let Some(new_ver) = Self::get_latest_component_version(&c) {
                            if new_ver.gt(old_ver) {
                                self.recently_updated.truncate(19);
                                self.recently_updated.insert(0, c.clone());
                            }
                        }
                    }
                    None => {
                        self.recently_added.truncate(19);
                        self.recently_added.insert(0, c.clone());
                    }
                }
            }

            if let Some(v) = Self::get_latest_component_version(&c) {
                self.latest_versions.insert(c.id.0.to_owned(), v);
            }
        }

        println!("{:?}", self.latest_versions);
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
                        tracing::error!(
                            "Flatpak remote returned {} for icon download",
                            res.status(),
                        );
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

    async fn download_appstream_xml(
        &self,
        client: &ClientWithMiddleware,
    ) -> Result<(), std::io::Error> {
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

        let stream = res.bytes_stream().map(|result| {
            result.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
        });

        let mut read = StreamReader::new(stream);

        tokio::io::copy(&mut read, &mut out_file).await?;

        Ok(())
    }

    async fn parse_and_extract_appstream_collection(&self) -> Result<Collection, std::io::Error> {
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
}

#[cfg(test)]
mod tests {
    use super::AppstreamWorker;
    use appstream::{
        builders::{ComponentBuilder, ReleaseBuilder},
        TranslatableString,
    };
    use chrono::TimeZone;

    #[test]
    fn version_comparison() {
        let c1: appstream::Component = ComponentBuilder::default()
            .id("com.example.foobar".into())
            .name(TranslatableString::with_default("Foo Bar"))
            .metadata_license("CC0-1.0".into())
            .summary(TranslatableString::with_default("A foo-ish bar"))
            .release(ReleaseBuilder::new("1.2").build())
            .release(ReleaseBuilder::new("1.3").build())
            .release(ReleaseBuilder::new("1.3.19").build())
            .build();

        assert_eq!(
            Some(lenient_semver::parse("1.3.19").unwrap()),
            AppstreamWorker::get_latest_component_version(&c1)
        );

        let c2: appstream::Component = ComponentBuilder::default()
            .id("com.example.foobar".into())
            .name(TranslatableString::with_default("Foo Bar"))
            .metadata_license("CC0-1.0".into())
            .summary(TranslatableString::with_default("A foo-ish bar"))
            .release(ReleaseBuilder::new("0.1").build())
            .release(ReleaseBuilder::new("1.0").build())
            .release(
                ReleaseBuilder::new("1.0.2")
                    .date(
                        chrono::Utc
                            .with_ymd_and_hms(2023, 01, 01, 12, 12, 13)
                            .unwrap(),
                    )
                    .build(),
            )
            .release(
                ReleaseBuilder::new("1.0.12")
                    .date(
                        chrono::Utc
                            .with_ymd_and_hms(2023, 01, 01, 12, 12, 10)
                            .unwrap(),
                    )
                    .build(),
            )
            .build();

        assert_eq!(
            Some(lenient_semver::parse("1.0.2").unwrap()),
            AppstreamWorker::get_latest_component_version(&c2)
        );
    }
}
