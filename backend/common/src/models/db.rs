use crate::schema::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Queryable, PartialEq, Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password: Option<String>,
    pub is_active: bool,
    pub is_admin: bool,
    pub date_joined: time::OffsetDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub email: &'a str,
    pub password: Option<String>,
    pub is_active: bool,
    pub is_admin: bool,
}

#[derive(Insertable)]
#[diesel(table_name = user_profile)]
pub struct NewProfile<'a> {
    pub user_id: &'a Uuid,
    pub profile_picture_url: Option<&'a str>,
    pub github_link: Option<&'a str>,
}

#[derive(Queryable, Insertable, PartialEq, Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct App {
    pub id: String,
    pub repository: String,
    pub is_verified: bool,
    pub last_submitted_version: Option<String>,
    pub first_seen: Option<time::OffsetDateTime>,
    pub last_update: Option<time::OffsetDateTime>,
    pub is_published: bool,
    pub stripe_connect_id: Option<String>,
}

#[derive(Insertable, Queryable, PartialEq, Debug, Clone)]
#[diesel(table_name = github_auth)]
pub struct GithubAuth {
    pub user_id: Uuid,
    pub github_user_id: Option<String>,
    pub github_access_token: Option<String>,
    pub github_refresh_token: Option<String>,
}

pub struct NewGithubAuth {
    pub github_user_id: Option<String>,
    pub github_access_token: Option<String>,
    pub github_refresh_token: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct RepoAppFile {
    pub source: String,
    pub commit: String,
    pub version: String,
}
