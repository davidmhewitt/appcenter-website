use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct App {
    #[schema(example = "com.github.davidmhewitt.torrential")]
    pub id: String,
    #[schema(example = "https://github.com/davidmhewitt/torrential.git")]
    pub repository: String,
    #[schema(example = true)]
    pub is_verified: bool,
    #[schema(example = "3.0.0")]
    pub version: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApp {
    #[schema(example = "com.github.davidmhewitt.torrential")]
    pub app_id: String,
    #[schema(example = "https://github.com/davidmhewitt/torrential.git")]
    pub repository: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AppUpdateSubmission {
    #[schema(example = "com.github.davidmhewitt.torrential")]
    pub app_id: String,
    #[schema(example = "3.0.0")]
    pub version_tag: String,
}
