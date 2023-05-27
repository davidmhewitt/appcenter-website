use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct App {
    #[cfg_attr(feature = "openapi", schema(example = "com.github.davidmhewitt.torrential"))]
    pub id: String,
    #[cfg_attr(feature = "openapi", schema(example = "https://github.com/davidmhewitt/torrential.git"))]
    pub repository: String,
    #[cfg_attr(feature = "openapi", schema(example = true))]
    pub is_verified: bool,
    #[cfg_attr(feature = "openapi", schema(example = "3.0.0"))]
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]

pub struct CreateApp {
    #[cfg_attr(feature = "openapi", schema(example = "com.github.davidmhewitt.torrential"))]
    pub app_id: String,
    #[cfg_attr(feature = "openapi", schema(example = "https://github.com/davidmhewitt/torrential.git"))]
    pub repository: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]

pub struct AppUpdateSubmission {
    #[cfg_attr(feature = "openapi", schema(example = "com.github.davidmhewitt.torrential"))]
    pub app_id: String,
    #[cfg_attr(feature = "openapi", schema(example = "3.0.0"))]
    pub version_tag: String,
}
