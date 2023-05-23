use crate::OCTO;
use anyhow::{anyhow, Result};
use secrecy::SecretString;

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
    title: String,
    src_branch: String,
    dst_branch: String,
    body: String,
) -> Result<()> {
    OCTO.pulls("davidmhewitt", "appcenter-reviews")
        .create(title, src_branch, dst_branch)
        .body(body)
        .send()
        .await?;

    Ok(())
}
