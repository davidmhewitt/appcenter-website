pub mod tasks;

use diesel::{r2d2, PgConnection};
use fang::{queue::Task, Queue, QueueError, Queueable, Runnable};
use git_worker::GitWorker;
use once_cell::sync::Lazy;

fn new_git_worker() -> GitWorker {
    let settings = common::settings::get_settings().expect("Failed to read settings.");
    GitWorker::new(
        settings.github.local_repo_path,
        settings.github.reviews_url,
        settings.github.username,
        settings.github.access_token,
    )
    .expect("Unable to create git worker")
}

pub static GIT_WORKER: Lazy<GitWorker> = Lazy::new(new_git_worker);
pub static QUEUE: Lazy<Queue> = Lazy::new(|| {
    let settings = common::settings::get_settings().expect("Failed to read settings.");
    let manager = r2d2::ConnectionManager::<PgConnection>::new(settings.database.url);
    let pg_pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Couldn't build postgres pool");
    Queue::builder().connection_pool(pg_pool).build()
});

pub fn insert_task(task: &dyn Runnable) -> Result<Task, QueueError> {
    QUEUE.insert_task(task)
}
