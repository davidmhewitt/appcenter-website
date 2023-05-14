use std::fs;
use utoipa::OpenApi;

fn main() {
    fs::write(
        "openapi.json",
        backend::routes::ApiDoc::openapi().to_pretty_json().unwrap(),
    )
    .expect("Unable to write OpenAPI JSON");
}
