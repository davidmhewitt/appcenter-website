use diesel::{upsert::excluded, Connection, ExpressionMethods, PgConnection, RunQueryDsl};
use fang::{
    serde::{Deserialize, Serialize},
    typetag, FangError, Queueable, Runnable, Scheduled,
};

use common::models::App;

use crate::GIT_WORKER;

#[derive(Serialize, Deserialize)]
pub struct VersionsFromRepo {}

impl VersionsFromRepo {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for VersionsFromRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[typetag::serde]
impl Runnable for VersionsFromRepo {
    fn run(&self, _queue: &dyn Queueable) -> Result<(), FangError> {
        tracing::info!("Checking appcenter reviews repo...");

        use common::schema::apps::dsl::*;

        let files = GIT_WORKER.get_file_touch_times().map_err(|e| FangError {
            description: e.to_string(),
        })?;

        let settings: common::settings::Settings =
            common::settings::get_settings().expect("Failed to read settings.");

        let mut connection = PgConnection::establish(&settings.database.url)
            .expect("Unable to connect to database to insert apps");

        let apps_to_insert = files
            .iter()
            .map(|f| App {
                id: f.0.file_stem().unwrap().to_string_lossy().to_string(),
                is_verified: false,
                is_published: true,
                repository: f.1.repository.to_owned(),
                last_submitted_version: Some(f.1.version.to_owned()),
                first_seen: Some(f.1.first),
                last_update: Some(f.1.last),
            })
            .collect::<Vec<_>>();

        diesel::insert_into(apps)
            .values(&apps_to_insert)
            .on_conflict(id)
            .do_update()
            .set((
                repository.eq(excluded(repository)),
                last_submitted_version.eq(excluded(last_submitted_version)),
                first_seen.eq(excluded(first_seen)),
                last_update.eq(excluded(last_update)),
                is_published.eq(excluded(is_published)),
            ))
            .execute(&mut connection)
            .map_err(|e| FangError {
                description: e.to_string(),
            })?;

        tracing::info!("Done!");

        Ok(())
    }

    fn uniq(&self) -> bool {
        true
    }

    fn cron(&self) -> Option<Scheduled> {
        Some(Scheduled::CronPattern("0 0/5 * * * *".to_string()))
    }
}
