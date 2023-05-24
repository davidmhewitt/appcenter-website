use crate::redis_utils;
use common::APP_SUMMARIES_REDIS_KEY;

use appstream::{enums::Bundle, Collection, Component};
use reqwest::{header::ETAG, Client, StatusCode};
use std::{
    io::{Error, ErrorKind},
    path::Path,
};
use tokio_stream::StreamExt;
use tokio_util::io::StreamReader;
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

        match self.download_appstream_xml_sync() {
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
                    if let Err(e) = download_icon_sync(
                        "https://flatpak.elementary.io",
                        width,
                        height,
                        &self.http_client,
                        path,
                    ) {
                        tracing::warn!("Error downloading appstream icon: {}", e);
                    }
                }
            }
        }
    }

    fn download_appstream_xml_sync(&self) -> Result<(), Error> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        rt.block_on(self.download_appstream_xml())
    }

    async fn download_appstream_xml(&self) -> Result<(), Error> {
        let res = self
            .http_client
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

        let mut reader = StreamReader::new(
            res.bytes_stream()
                .map(|result| result.map_err(|err| Error::new(ErrorKind::Other, err))),
        );

        tokio::io::copy(&mut reader, &mut out_file).await?;

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

fn download_icon_sync(
    base_url: &str,
    width: &Option<u32>,
    height: &Option<u32>,
    client: &reqwest::Client,
    path: &Path,
) -> Result<(), Error> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(download_icon(base_url, width, height, client, path))
}

async fn download_icon(
    base_url: &str,
    width: &Option<u32>,
    height: &Option<u32>,
    client: &reqwest::Client,
    path: &Path,
) -> Result<(), Error> {
    let mut dir = String::from("icons");
    if let (Some(width), Some(height)) = (width, height) {
        dir += &format!("/{}x{}", width, height);
    }

    let url = format!(
        "{}/repo/appstream/x86_64/{}/{}",
        base_url,
        dir,
        path.to_string_lossy()
    );

    let res = client
        .head(&url)
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

    if let Some(etag) = res.headers().get(ETAG) {
        let cache_key = format!(
            "{}-{}",
            &url,
            etag.to_str()
                .map_err(|_| Error::new(ErrorKind::Other, format!("Etag header was invalid"),))?
        );

        if let Some(mut cached_file) = match cacache::Reader::open(".image_cache", &cache_key).await
        {
            Ok(f) => Some(f),
            Err(cacache::Error::EntryNotFound(_, _)) => None,
            Err(e) => return Err(Error::new(ErrorKind::Other, e)),
        } {
            let path = format!("_apps/{}/{}", dir, path.to_string_lossy());
            if std::path::Path::new(&path).exists() {
                return Ok(());
            }

            let mut icon_file = tokio::fs::File::create(&path)
                .await
                .map_err(|e| Error::new(ErrorKind::Other, e))?;

            tokio::io::copy(&mut cached_file, &mut icon_file).await?;

            if let Ok(_) = cached_file.check() {
                return Ok(());
            }
        }

        let res = client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        if res.status() != StatusCode::OK {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Flatpak remote returned {} for icon download", res.status()),
            ));
        }

        let mut reader = StreamReader::new(
            res.bytes_stream()
                .map(|result| result.map_err(|err| Error::new(ErrorKind::Other, err))),
        );

        if let Ok(mut cache_writer) = cacache::Writer::create(".image_cache", &cache_key).await {
            tokio::io::copy(&mut reader, &mut cache_writer).await?;
            cache_writer.commit().await.ok();
        }

        let mut icon_file =
            tokio::fs::File::create(format!("_apps/{}/{}", dir, path.to_string_lossy()))
                .await
                .map_err(|e| Error::new(ErrorKind::Other, e))?;

        tokio::io::copy(&mut reader, &mut icon_file).await?;

        return Ok(());
    }

    Err(Error::new(ErrorKind::Other, "Unable to download file"))
}

#[cfg(test)]
mod tests {
    use super::*;

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_download_icon() -> Result<(), Error> {
        let mock_server = MockServer::start().await;

        let http_client = Client::new();

        Mock::given(method("HEAD"))
            .and(path(
                "/repo/appstream/x86_64/icons/64x64/com.github.fakeorg.fakeapp.png",
            ))
            .respond_with(ResponseTemplate::new(200).append_header(ETAG, "1234"))
            .expect(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repo/appstream/x86_64/icons/64x64/com.github.fakeorg.fakeapp.png",
            ))
            .respond_with(ResponseTemplate::new(200).append_header(ETAG, "1234"))
            .expect(1)
            .mount(&mock_server)
            .await;

        download_icon(
            &mock_server.uri(),
            &Some(64),
            &Some(64),
            &http_client,
            &Path::new("com.github.fakeorg.fakeapp.png"),
        )
        .await?;

        download_icon(
            &mock_server.uri(),
            &Some(64),
            &Some(64),
            &http_client,
            &Path::new("com.github.fakeorg.fakeapp.png"),
        )
        .await?;

        Ok(())
    }
}
