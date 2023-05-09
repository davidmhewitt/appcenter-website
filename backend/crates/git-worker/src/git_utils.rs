use std::path::Path;

use git2::{build::RepoBuilder, Cred, FetchOptions, RemoteCallbacks, Repository};
use secrecy::{ExposeSecret, SecretString};

use crate::git_worker::Error;

pub(crate) fn get_fetch_options<'a>(
    git_repo_url: &'a str,
    git_username: &'a str,
    git_password: &'a SecretString,
) -> FetchOptions<'a> {
    let remote_callbacks = get_remote_auth_callbacks(git_repo_url, git_username, git_password);

    let mut options = FetchOptions::default();
    options.remote_callbacks(remote_callbacks);

    options
}

pub(crate) fn get_remote_auth_callbacks<'a>(
    git_repo_url: &'a str,
    git_username: &'a str,
    git_password: &'a secrecy::Secret<String>,
) -> RemoteCallbacks<'a> {
    let mut remote_callbacks = RemoteCallbacks::default();

    remote_callbacks.credentials(move |url, _username_from_url, _allowed_types| {
        if url == git_repo_url {
            return Cred::userpass_plaintext(git_username, git_password.expose_secret());
        }

        Err(git2::Error::from_str("Couldn't find credentials for URL"))
    });

    remote_callbacks
}

pub(crate) fn open_repo(
    repo_path: &Path,
    git_repo_url: &str,
    git_username: &str,
    git_password: &SecretString,
) -> Result<Repository, Error> {
    match Repository::open(repo_path) {
        Ok(r) => Ok(r),
        Err(_) => clone_repo(repo_path, git_repo_url, git_username, git_password),
    }
}

pub(crate) fn clone_repo(
    repo_path: &Path,
    git_repo_url: &str,
    git_username: &str,
    git_password: &SecretString,
) -> Result<Repository, Error> {
    RepoBuilder::new()
        .fetch_options(get_fetch_options(git_repo_url, git_username, git_password))
        .clone(git_repo_url, Path::new(repo_path))
        .map_err(Error::Git)
}
