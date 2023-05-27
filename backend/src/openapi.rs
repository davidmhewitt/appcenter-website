#[cfg(feature = "openapi")]
use std::fs;
#[cfg(feature = "openapi")]
use utoipa::OpenApi;

fn main() {
    #[cfg(feature = "openapi")]
    fs::write(
        "openapi.json",
        backend::routes::ApiDoc::openapi().to_pretty_json().unwrap(),
    )
    .expect("Unable to write OpenAPI JSON");
}
