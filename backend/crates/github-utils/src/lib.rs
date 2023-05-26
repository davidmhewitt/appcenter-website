use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use secrecy::{ExposeSecret, SecretString};
use url::Url;

#[derive(Debug)]
pub enum GithubOwner {
    User(String),
    Org(String),
}

pub async fn get_github_repo_owner_id(org: &str, repo: &str) -> Result<GithubOwner> {
    let owner = OCTO.repos(org, repo).get().await.map(|r| r.owner)?;

    if let Some(owner) = owner {
        if owner.r#type == "Organization" {
            return Ok(GithubOwner::Org(owner.id.0.to_string()));
        } else {
            return Ok(GithubOwner::User(owner.id.0.to_string()));
        }
    }

    Err(anyhow!("Unable to find repository owner"))
}

pub async fn is_user_admin_member_of_github_org(
    _access_token: &SecretString,
    _org_id: &str,
) -> Result<bool> {
    // TODO: Awaiting https://github.com/XAMPPRocky/octocrab/pull/357

    Ok(false)
}

pub async fn create_pull_request(
    title: &str,
    src_branch: &str,
    dst_branch: &str,
    body: &str,
) -> Result<()> {
    let settings = common::settings::get_settings().expect("Unable to get settings");

    let url = Url::parse(&settings.github.reviews_url)?;
    let path_segments = match url.path_segments() {
        Some(s) => s,
        None => {
            return Err(anyhow!("Unable to get path segments from URL"));
        }
    }
    .collect::<Vec<&str>>();

    let path_org_name = path_segments
        .first()
        .ok_or(anyhow!("Couldn't get reviews repo owner"))?;
    let path_repo_name = path_segments.get(1).ok_or(anyhow!("Couldn't get reviews repo name"))?;
    let path_repo_name = if path_repo_name.ends_with(".git") {
        path_repo_name.strip_suffix(".git").unwrap()
    } else {
        path_repo_name
    };

    OCTO.pulls(*path_org_name, path_repo_name)
        .create(title, src_branch, dst_branch)
        .body(body)
        .send()
        .await?;

    Ok(())
}

static OCTO: Lazy<octocrab::Octocrab> = Lazy::new(|| {
    let settings = common::settings::get_settings().expect("Unable to get settings");
    octocrab::OctocrabBuilder::new()
        .personal_token(settings.github.access_token.expose_secret().to_owned())
        .build()
        .expect("Unable to build GitHub client")
});
