mod github_utils;

pub use github_utils::*;
use once_cell::sync::Lazy;
use secrecy::ExposeSecret;

static OCTO: Lazy<octocrab::Octocrab> = Lazy::new(|| {
    let settings = common::settings::get_settings().expect("Unable to get settings");
    octocrab::OctocrabBuilder::new()
        .personal_token(settings.github.access_token.expose_secret().to_owned())
        .build()
        .expect("Unable to build GitHub client")
});