use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use background_worker::{tasks, QUEUE};
use diesel::{Connection, PgConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};
use fang::{Queue, Queueable, RetentionMode, WorkerPool};

pub const MIGRATIONS: EmbeddedMigrations = diesel_migrations::embed_migrations!();

fn main() {
    let settings = common::settings::get_settings().expect("Failed to read settings.");

    let subscriber = common::telemetry::get_subscriber(settings.debug);
    common::telemetry::init_subscriber(subscriber);

    let mut connection = PgConnection::establish(&settings.database.url)
        .expect("Unable to connect to database to run migrations");
    connection
        .run_pending_migrations(MIGRATIONS)
        .expect("Unable to run database migrations");

    let mut worker_pool = WorkerPool::<Queue>::builder()
        .queue(QUEUE.clone())
        .retention_mode(RetentionMode::RemoveFinished)
        .number_of_workers(3_u32)
        .task_type(fang::DEFAULT_TASK_TYPE)
        .build();

    QUEUE
        .insert_task(&tasks::VersionsFromRepo::default())
        .expect("Unable to queue background worker");
    QUEUE
        .insert_task(&tasks::AppdataUpdate::default())
        .expect("Unable to queue background worker");

    worker_pool
        .start()
        .expect("Unable to start background worker pool");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
