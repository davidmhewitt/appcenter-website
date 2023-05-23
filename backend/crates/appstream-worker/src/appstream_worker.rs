use crate::redis_utils;
use common::APP_SUMMARIES_REDIS_KEY;

use appstream::{enums::Bundle, Collection, Component};
use reqwest::{blocking::Client, header::ETAG, StatusCode};
use std::{
    io::{Error, ErrorKind},
    path::Path,
};
pub struct AppstreamWorker {
    redis_client: redis::Client,
    http_client: Client,
}

impl Default for AppstreamWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl AppstreamWorker {
    pub fn new() -> Self {
        let settings = common::settings::get_settings().expect("Unable to load settings");
        Self {
            redis_client: redis::Client::open(settings.redis.uri)
                .expect("Cannot create deadpool redis"),
            http_client: Client::new(),
        }
    }

    pub fn run_appstream_update(&self) {
        tracing::info!("Updating AppStream info");

        match self.download_appstream_xml() {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Error downloading appstream xml: {:?}", e);
                return;
            }
        };

        let mut collection = match self.parse_and_extract_appstream_collection() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Error parsing appstream download: {:?}", e);
                return;
            }
        };

        self.summarise_apps(&mut collection);
        self.download_icons(&collection);
    }

    fn summarise_apps(&self, collection: &mut [Component]) {
        let mut redis_con = self
            .redis_client
            .get_connection()
            .map_err(|e| {
                tracing::error!("Error getting redis connection: {}", e);
            })
            .expect("Redis connection cannot be gotten.");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Unable to start tokio runtime for async methods");

        for c in collection.iter() {
            let summary =
                match serde_json::ser::to_string(&common::models::ComponentSummary::from(c)) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!("Error serializing component summary: {}", e);
                        continue;
                    }
                };

            rt.block_on(redis_utils::hset(
                &mut redis_con,
                APP_SUMMARIES_REDIS_KEY,
                &c.id.0,
                &summary,
            ));
        }
    }

    fn download_icons(&self, collection: &Vec<Component>) {
        for c in collection {
            for icon in &c.icons {
                // TODO: Do we need to handle other icon types?
                if let appstream::enums::Icon::Cached {
                    path,
                    width,
                    height,
                } = icon
                {
                    if let Err(e) = download_icon(width, height, &self.http_client, path) {
                        tracing::warn!("Error downloading appstream icon: {}", e);
                    }
                }
            }
        }
    }

    fn download_appstream_xml(&self) -> Result<(), Error> {
        let mut res = self
            .http_client
            .get("https://flatpak.elementary.io/repo/appstream/x86_64/appstream.xml.gz")
            .send()
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

        let mut out_file = std::fs::File::create("/tmp/appstream.xml.gz")
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        res.copy_to(&mut out_file)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(())
    }

    fn parse_and_extract_appstream_collection(&self) -> Result<Vec<Component>, Error> {
        match std::fs::create_dir("_apps") {
            Ok(()) => Ok(()),
            Err(e) => {
                if e.kind() == ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(Error::new(ErrorKind::Other, e))
                }
            }
        }?;

        let collection = Collection::from_gzipped("/tmp/appstream.xml.gz".into())
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        let components: Vec<Component> = collection
            .components
            .into_iter()
            .filter(|c| !c.id.0.starts_with("org.gnome."))
            .filter(|c| match c.bundles.first().unwrap() {
                Bundle::Flatpak {
                    runtime: _,
                    sdk: _,
                    reference,
                } => reference.ends_with("/stable"),
                _ => true,
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
    }
}

fn download_icon(
    width: &Option<u32>,
    height: &Option<u32>,
    client: &Client,
    path: &Path,
) -> Result<(), Error> {
    let mut dir = String::from("icons");
    if let (Some(width), Some(height)) = (width, height) {
        dir += &format!("/{}x{}", width, height);
    }

    let url = format!(
        "https://flatpak.elementary.io/repo/appstream/x86_64/{}/{}",
        dir,
        path.to_string_lossy()
    );

    let mut res = client
        .get(&url)
        .send()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    if res.status() != StatusCode::OK {
        return Err(Error::new(
            ErrorKind::Other,
            format!("Flatpak remote returned {} for icon download", res.status()),
        ));
    }

    if let Err(e) = std::fs::create_dir_all(format!("_apps/{}", dir)) {
        return Err(Error::new(
            ErrorKind::Other,
            format!("Error creating directory for appstream icons: {:?}", e),
        ));
    }

    if let Some(etag) = res.headers().get(ETAG) {
        let cache_key = format!(
            "{}-{}",
            &url,
            etag.to_str()
                .map_err(|_| Error::new(ErrorKind::Other, format!("Etag header was invalid"),))?
        );

        if let Some(mut cached_file) = match cacache::SyncReader::open(".image_cache", &cache_key) {
            Ok(f) => Some(f),
            Err(cacache::Error::EntryNotFound(_, _)) => None,
            Err(e) => return Err(Error::new(ErrorKind::Other, e)),
        } {
            let path = format!("_apps/{}/{}", dir, path.to_string_lossy());
            if std::path::Path::new(&path).exists() {
                return Ok(());
            }

            let mut icon_file =
                std::fs::File::create(&path).map_err(|e| Error::new(ErrorKind::Other, e))?;

            std::io::copy(&mut cached_file, &mut icon_file)?;

            if let Ok(_) = cached_file.check() {
                return Ok(());
            }
        }

        tracing::info!("Downloading body of icon");

        if let Ok(mut cache_writer) = cacache::SyncWriter::create(".image_cache", &cache_key) {
            res.copy_to(&mut cache_writer)
                .map_err(|e| Error::new(ErrorKind::Other, e))?;
            cache_writer.commit().ok();
        }

        let mut icon_file =
            std::fs::File::create(format!("_apps/{}/{}", dir, path.to_string_lossy()))
                .map_err(|e| Error::new(ErrorKind::Other, e))?;

        res.copy_to(&mut icon_file)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        return Ok(());
    }

    Err(Error::new(ErrorKind::Other, "Unable to download file"))
}
