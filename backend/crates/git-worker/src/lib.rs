mod git_utils;
mod git_worker;

pub use self::git_utils::get_remote_commit_id_from_tag;
pub use self::git_worker::GitWorker;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Git error: {0}")]
    Git(git2::Error),
}
