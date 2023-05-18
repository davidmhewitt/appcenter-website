use crate::schema::*;
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

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

#[derive(Queryable, PartialEq, Debug, Clone, Serialize)]
pub struct App {
    pub id: String,
    pub repository: String,
    pub is_verified: bool,
    pub last_submitted_version: Option<String>,
    pub first_seen: Option<time::OffsetDateTime>,
    pub last_update: Option<time::OffsetDateTime>,
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
