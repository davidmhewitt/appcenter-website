use secrecy::SecretString;

pub struct User {
    pub id: uuid::Uuid,
    pub email: String,
    pub password_hash: Option<SecretString>,
    pub is_active: bool,
    pub is_admin: bool,
    pub date_joined: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UserVisible {
    pub id: uuid::Uuid,
    pub email: String,
    pub is_active: bool,
    pub is_admin: bool,
    pub date_joined: chrono::DateTime<chrono::Utc>,
}
#[derive(serde::Serialize)]
pub struct LoggedInUser {
    pub id: uuid::Uuid,
    pub email: String,
    pub password: String,
    pub is_admin: bool,
}

pub struct CreateNewUser {
    pub email: String,
    pub password: Option<SecretString>,
    pub is_active: bool,
    pub github_id: Option<i64>,
}
