use flate2::read::GzDecoder;
use reqwest::StatusCode;
use roxmltree::{Document, Node};
use serde::Serialize;
use std::{io::Read, time::Duration};
use tokio::task::spawn_blocking;
use tokio_stream::StreamExt;
use tokio_util::io::StreamReader;

struct ComponentBuilder {
    id: Option<String>,
}

impl ComponentBuilder {
    pub fn new() -> Self {
        Self { id: None }
    }

    pub fn id(&mut self, id: String) {
        self.id = Some(id);
    }

    pub fn build(self) -> Result<Component, std::io::Error> {
        let Self { id } = self;
        let id = id.ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Component id missing",
        ))?;
        Ok(Component { id: id })
    }
}

#[derive(Serialize, Debug)]
struct Component {
    id: String,
}

impl Component {
    pub fn from_node(node: &Node) -> Result<Self, std::io::Error> {
        let mut builder = ComponentBuilder::new();
        for e in node.children().filter(|e| e.is_element()) {
            match e.tag_name().name() {
                "id" => {
                    builder.id(e
                        .text()
                        .ok_or(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Component id missing",
                        ))?
                        .to_string());
                }
                _ => {}
            };
        }

        builder.build()
    }
}

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

                let doc_text = String::from_utf8(xml_data)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

                let doc = Document::parse(&doc_text)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

                let root = doc.root_element();
                println!(
                    "{:?}",
                    root.children().filter(Node::is_element).map(|e| Component::from_node(&e)).collect::<Vec<_>>()
                );

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
