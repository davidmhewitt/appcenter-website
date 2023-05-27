use diesel::{Connection, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use fang::{
    serde::{Deserialize, Serialize},
    typetag, FangError, Queueable, Runnable,
};
use uuid::Uuid;

use crate::GIT_WORKER;

#[derive(Serialize, Deserialize)]
pub struct SubmitAppUpdate {
    app_id: String,
    version_tag: String,
    user_uuid: Uuid,
}

impl SubmitAppUpdate {
    pub fn new(app_id: String, version_tag: String, user_uuid: Uuid) -> Self {
        Self {
            app_id,
            version_tag,
            user_uuid,
        }
    }
}

#[typetag::serde]
impl Runnable for SubmitAppUpdate {
    fn run(&self, _queueable: &dyn Queueable) -> Result<(), FangError> {
        let settings: common::settings::Settings =
            common::settings::get_settings().expect("Failed to read settings.");

        let mut con = PgConnection::establish(&settings.database.url)
            .expect("Unable to connect to database to insert apps");

        let repo_url = match get_repo_url_from_db(&mut con, &self.app_id, &self.user_uuid) {
            Ok(r) => r,
            Err(_) => {
                return Err(FangError {
                    description: "Unable to get repository URL for app".into(),
                });
            }
        };

        let branch_name = format!("appcenter-website/{}-{}", self.app_id, self.version_tag);
        let commit_message = format!("{} version {}", self.app_id, self.version_tag);

        let commit_id =
            match git_worker::get_remote_commit_id_from_tag(&repo_url, &self.version_tag) {
                Ok(id) => id,
                Err(_) => {
                    return Err(FangError {
                        description: "Unable to get commit ID for app".into(),
                    })
                }
            };

        let info = common::models::RepoAppFile {
            source: repo_url,
            commit: commit_id,
            version: self.version_tag.to_owned(),
        };

        if let Err(e) = GIT_WORKER.checkout_branch("main") {
            tracing::error!("Error checking out main branch: {}", e);
            return Err(FangError {
                description: "Error checking out main branch".into(),
            });
        }

        if let Err(e) = GIT_WORKER.update_repo() {
            tracing::error!("Error updating git repo: {}", e);
            return Err(FangError {
                description: "Error updating git repo".into(),
            });
        }

        if let Err(e) = GIT_WORKER.create_branch(&branch_name) {
            tracing::error!("Error creating branch: {}", e);

            if let Err(e) = GIT_WORKER.delete_local_branch(&branch_name) {
                tracing::error!("Error deleting local branch: {}", e);
            }

            return Err(FangError {
                description: "Error creating branch".into(),
            });
        }

        if let Err(e) = std::fs::write(
            GIT_WORKER
                .repo_path
                .join("applications")
                .join(format!("{}.json", self.app_id)),
            serde_json::ser::to_string_pretty(&info).unwrap(),
        ) {
            tracing::error!("Error writing app info to repo: {}", e);
            if let Err(e) = GIT_WORKER.checkout_branch("main") {
                tracing::error!("Error changing local branch: {}", e);
            }

            if let Err(e) = GIT_WORKER.delete_local_branch(&branch_name) {
                tracing::error!("Error deleting local branch: {}", e);
            }

            return Err(FangError {
                description: "Error writing app info to repo".into(),
            });
        }

        if let Err(e) = GIT_WORKER.add_and_commit(
            &["applications"],
            &commit_message,
            &settings.github.username,
            "builds@elementary.io",
        ) {
            tracing::error!("Error committing app: {}", e);
            if let Err(e) = GIT_WORKER.checkout_branch("main") {
                tracing::error!("Error changing local branch: {}", e);
            }

            if let Err(e) = GIT_WORKER.delete_local_branch(&branch_name) {
                tracing::error!("Error deleting local branch: {}", e);
            }

            return Err(FangError {
                description: "Error commiting to git repo".into(),
            });
        }

        if let Err(e) = GIT_WORKER.push(&branch_name) {
            tracing::error!("Error pushing app: {}", e);
            return Err(FangError {
                description: "Error pushing to git repo".into(),
            });
        }

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| FangError {
                description: "Unable to start tokio runtime for async methods".into(),
            })?;

        if let Err(e) = rt.block_on(github_utils::create_pull_request(
            &commit_message,
            &branch_name,
            "main",
            "This pull request was automatically generated by the AppCenter website.",
        )) {
            tracing::error!("Error opening pull request: {}", e);
            return Err(FangError {
                description: "Error opening pull request".into(),
            });
        }

        Ok(())
    }
}

pub fn get_repo_url_from_db(
    con: &mut PgConnection,
    app_id: &str,
    uuid: &Uuid,
) -> Result<String, diesel::result::Error> {
    use common::schema::app_owners;
    use common::schema::apps::dsl::*;

    apps.inner_join(app_owners::table)
        .select(repository)
        .filter(app_owners::user_id.eq(uuid))
        .filter(id.eq(app_id))
        .filter(app_owners::verified_owner.eq(true))
        .get_result::<String>(con)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_url_from_db() -> Result<(), diesel::result::Error> {
        use common::schema::app_owners::dsl::*;
        use common::schema::apps::dsl::*;
        use common::schema::users::{dsl::users, email, is_active, is_admin};

        std::env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../"))
            .expect("Couldn't set working directory for test");

        let settings: common::settings::Settings =
            common::settings::get_settings().expect("Failed to read settings.");

        let mut con = PgConnection::establish(&settings.database.url)
            .expect("Unable to connect to database to insert apps");

        con.begin_test_transaction()
            .expect("Unable to begin test transaction");

        diesel::insert_into(apps)
            .values((
                id.eq("com.github.fakeorg.fakeapp"),
                repository.eq("https://github.com/fakeorg/fakeapp"),
                is_verified.eq(true),
                is_published.eq(true),
            ))
            .execute(&mut con)?;

        let user1_id = Uuid::new_v4();
        diesel::insert_into(users)
            .values((
                common::schema::users::id.eq(user1_id),
                email.eq("test@example.com"),
                is_active.eq(false),
                is_admin.eq(false),
            ))
            .execute(&mut con)?;

        diesel::insert_into(app_owners)
            .values((
                user_id.eq(user1_id),
                app_id.eq("com.github.fakeorg.fakeapp"),
                verified_owner.eq(true),
            ))
            .execute(&mut con)?;

        let repo = get_repo_url_from_db(&mut con, "com.github.fakeorg.fakeapp", &user1_id)
            .expect("Unable to get repository");

        assert_eq!(repo, "https://github.com/fakeorg/fakeapp");

        Ok(())
    }
}
