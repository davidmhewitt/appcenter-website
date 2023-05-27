use std::path::PathBuf;

use secrecy::SecretString;

/// Global settings for exposing all preconfigured variables
#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub debug: bool,
    pub database: DatabaseSettings,
    pub redis: RedisSettings,
    pub secret: Secret,
    pub email: EmailSettings,
    pub github: GithubSettings,
    pub stripe: StripeSettings,
    pub frontend_url: String,
}

/// Application's specific settings to expose `port`,
/// `host`, `protocol`, and possible URL of the application
/// during and after development
#[derive(serde::Deserialize, Clone)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
    pub base_url: String,
}

/// Redis settings for the entire app
#[derive(serde::Deserialize, Clone, Debug)]
pub struct RedisSettings {
    pub uri: String,
}

/// Database settings for the entire app
#[derive(serde::Deserialize, Clone)]
pub struct DatabaseSettings {
    pub url: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct Secret {
    pub secret_key: SecretString,
    pub token_expiration: i64,
    pub hmac_secret: SecretString,
}

#[derive(serde::Deserialize, Clone)]
pub struct EmailSettings {
    pub host: String,
    pub host_user: String,
    pub host_user_password: SecretString,
    pub authentication: bool,
    pub port: u16,
}

#[derive(serde::Deserialize, Clone)]
pub struct GithubSettings {
    pub client_id: String,
    pub client_secret: SecretString,
    pub reviews_url: String,
    pub username: String,
    pub access_token: SecretString,
    pub local_repo_path: PathBuf,
}

#[derive(serde::Deserialize, Clone)]
pub struct StripeSettings {
    pub secret_key: SecretString,
}

/// The possible runtime environment for our application.
pub enum Environment {
    Development,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "development" => Ok(Self::Development),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `development` or `production`.",
                other
            )),
        }
    }
}

/// Multipurpose function that helps detect the current environment the application
/// is running using the `APP_ENVIRONMENT` environment variable.
///
/// \`\`\`
/// APP_ENVIRONMENT = development | production.
/// \`\`\`
///
/// After detection, it loads the appropriate .yaml file
/// then it loads the environment variable that overrides whatever is set in the .yaml file.
/// For this to work, you the environment variable MUST be in uppercase and starts with `APP`,
/// a `_` separator then the category of settings,
/// followed by `__` separator,  and then the variable, e.g.
/// `APP__APPLICATION_PORT=5001` for `port` to be set as `5001`
pub fn get_settings() -> Result<Settings, config::ConfigError> {
    dotenv::dotenv().ok();

    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let settings_directory = base_path.join("settings");

    // Detect the running environment.
    // Default to `development` if unspecified.
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "development".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");
    let environment_filename = format!("{}.yaml", environment.as_str());
    let settings = config::Config::builder()
        .add_source(config::File::from(settings_directory.join("base.yaml")))
        .add_source(config::File::from(
            settings_directory.join(environment_filename),
        ))
        // Add in settings from environment variables (with a prefix of APP and '__' as separator)
        // E.g. `APP_APPLICATION__PORT=5001 would set `Settings.application.port`
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize::<Settings>()
}
