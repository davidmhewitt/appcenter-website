use anyhow::Result;
use octocrab::Octocrab;
use std::{path::PathBuf, sync::Mutex};
use tempdir::TempDir;

use git2::{
    build::CheckoutBuilder, FetchOptions, IndexAddOption, ObjectType, PushOptions, RemoteCallbacks,
    Repository,
};
use secrecy::{ExposeSecret, SecretString};

use crate::git_utils;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Git error: {0}")]
    Git(git2::Error),
    #[error("Github error: {0}")]
    GitHub(octocrab::Error),
    #[error("Repository owner not found")]
    RepoOwnerNotFound,
}

#[derive(Debug)]
pub enum GithubOwner {
    User(String),
    Org(String),
}

pub struct GitWorker {
    pub repo_path: PathBuf,
    git_repo_url: String,
    git_username: String,
    git_password: SecretString,
    repo: Mutex<Repository>,
    octo: Octocrab,
}

impl GitWorker {
    pub fn new(
        repo_path: PathBuf,
        git_repo_url: String,
        git_username: String,
        git_password: SecretString,
    ) -> Result<Self> {
        let repo = git_utils::open_repo(&repo_path, &git_repo_url, &git_username, &git_password)?;
        let octocrab = octocrab::OctocrabBuilder::new()
            .personal_token(git_password.expose_secret().to_owned())
            .build()
            .map_err(Error::GitHub)?;

        Ok(Self {
            repo_path,
            git_repo_url,
            git_username,
            git_password,
            repo: Mutex::new(repo),
            octo: octocrab,
        })
    }

    fn fetch_options(&self) -> FetchOptions {
        git_utils::get_fetch_options(&self.git_repo_url, &self.git_username, &self.git_password)
    }

    fn remote_auth_callbacks(&self) -> RemoteCallbacks {
        git_utils::get_remote_auth_callbacks(
            &self.git_repo_url,
            &self.git_username,
            &self.git_password,
        )
    }

    pub async fn create_pull_request(
        &self,
        title: String,
        src_branch: String,
        dst_branch: String,
        body: String,
    ) -> Result<(), Error> {
        self.octo
            .pulls("davidmhewitt", "appcenter-reviews")
            .create(title, src_branch, dst_branch)
            .body(body)
            .send()
            .await
            .map_err(Error::GitHub)?;
        Ok(())
    }

    pub async fn is_user_admin_member_of_github_org(
        &self,
        access_token: &SecretString,
        _org_id: &str,
    ) -> Result<bool, Error> {
        let _octo = octocrab::OctocrabBuilder::new()
            .oauth(octocrab::auth::OAuth {
                access_token: access_token.to_owned(),
                token_type: "Bearer".into(),
                scope: vec![],
            })
            .build()
            .map_err(Error::GitHub)?;

        // TODO: Awaiting https://github.com/XAMPPRocky/octocrab/pull/357

        return Ok(false);
    }

    pub async fn get_github_repo_owner_id(
        &self,
        org: &str,
        repo: &str,
    ) -> Result<GithubOwner, Error> {
        let owner = self
            .octo
            .repos(org, repo)
            .get()
            .await
            .and_then(|r| Ok(r.owner))
            .map_err(Error::GitHub)?;

        if let Some(owner) = owner {
            if owner.r#type == "Organization" {
                return Ok(GithubOwner::Org(owner.id.0.to_string()));
            } else {
                return Ok(GithubOwner::User(owner.id.0.to_string()));
            }
        }

        Err(Error::RepoOwnerNotFound)
    }

    pub fn update_repo(&self) -> Result<()> {
        let repo = self.repo.lock().unwrap();

        let mut remote = repo.find_remote("origin").map_err(Error::Git)?;
        remote
            .fetch(&["main"], Some(&mut self.fetch_options()), None)
            .map_err(Error::Git)?;

        let fetch_head = repo.find_reference("FETCH_HEAD").map_err(Error::Git)?;
        let fetch_commit = repo
            .reference_to_annotated_commit(&fetch_head)
            .map_err(Error::Git)?;

        let analysis = repo.merge_analysis(&[&fetch_commit]).map_err(Error::Git)?;
        if analysis.0.is_up_to_date() {
            return Ok(());
        }

        if analysis.0.is_fast_forward() {
            let mut reference = repo.find_reference("refs/heads/main").map_err(Error::Git)?;
            let msg = format!("Fast-Forward: Setting main to id: {}", fetch_commit.id());
            reference
                .set_target(fetch_commit.id(), &msg)
                .map_err(Error::Git)?;
            repo.set_head("refs/heads/main").map_err(Error::Git)?;
            repo.checkout_head(Some(CheckoutBuilder::default().force()))
                .map_err(Error::Git)?;

            return Ok(());
        }

        Err(Error::Git(git2::Error::from_str("Can't fast forward repo")).into())
    }

    pub fn create_branch(&self, branch_name: &str) -> Result<()> {
        let repo = self.repo.lock().unwrap();

        let head = repo
            .head()
            .map_err(Error::Git)?
            .peel_to_commit()
            .map_err(Error::Git)?;
        repo.branch(branch_name, &head, false).map_err(Error::Git)?;

        let treeish = repo.revparse_single(branch_name).map_err(Error::Git)?;
        repo.checkout_tree(&treeish, None).map_err(Error::Git)?;
        repo.set_head(&format!("refs/heads/{}", branch_name))
            .map_err(Error::Git)?;

        Ok(())
    }

    pub fn delete_local_branch(&self, branch_name: &str) -> Result<()> {
        let repo = self.repo.lock().unwrap();

        let mut branch = repo
            .find_branch(branch_name, git2::BranchType::Local)
            .map_err(Error::Git)?;
        branch.delete().map_err(Error::Git)?;

        Ok(())
    }

    pub fn checkout_branch(&self, branch_name: &str) -> Result<()> {
        let repo = self.repo.lock().unwrap();

        let treeish = repo.revparse_single(branch_name).map_err(Error::Git)?;
        repo.checkout_tree(&treeish, None).map_err(Error::Git)?;
        repo.set_head(&format!("refs/heads/{}", branch_name))
            .map_err(Error::Git)?;

        Ok(())
    }

    pub fn add_and_commit(&self, file_names: &[&str], message: &str) -> Result<()> {
        let repo = self.repo.lock().unwrap();

        let mut index = repo.index().map_err(Error::Git)?;

        index
            .add_all(file_names, IndexAddOption::DEFAULT, None)
            .map_err(Error::Git)?;
        let oid = index.write_tree().map_err(Error::Git)?;
        let sig = repo.signature().map_err(Error::Git)?;
        let tree = repo.find_tree(oid).map_err(Error::Git)?;

        let obj = repo
            .head()
            .and_then(|r| r.resolve())
            .and_then(|x| x.peel(ObjectType::Commit));

        if obj.is_ok() {
            let parent_commit = obj
                .unwrap()
                .into_commit()
                .map_err(|_| Error::Git(git2::Error::from_str("Couldn't find commit")))?;

            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent_commit])
                .map_err(Error::Git)?;
        } else {
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .map_err(Error::Git)?;
        }

        Ok(())
    }

    pub fn push(&self, branch_name: &str) -> Result<()> {
        let repo = self.repo.lock().unwrap();

        let mut remote = repo.find_remote("origin").map_err(Error::Git)?;

        let mut push_options = PushOptions::default();
        push_options.remote_callbacks(self.remote_auth_callbacks());

        remote
            .push(
                &[format!("refs/heads/{}", branch_name)],
                Some(&mut push_options),
            )
            .map_err(Error::Git)?;

        Ok(())
    }

    pub fn get_remote_commit_id_from_tag(&self, repo_url: &str, tag_name: &str) -> Result<String> {
        let temp_repo_dir = TempDir::new("remote")?;
        let temp_repo = git2::Repository::init(temp_repo_dir.path())?;
        let mut remote = temp_repo.remote("origin", repo_url)?;
        remote.connect(git2::Direction::Fetch)?;

        let refs = remote.list()?;
        for r in refs {
            if r.name() == format!("refs/tags/{}", tag_name) {
                return Ok(r.oid().to_string());
            }
        }

        Err(anyhow::format_err!("Couldn't find commit id"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, process::Command};

    #[tokio::test]
    async fn test_clone_repo() -> Result<()> {
        let remote_dir = TempDir::new("remote").expect("Couldn't create temporary remote dir");
        let remote_path = remote_dir.path().to_string_lossy();

        let local_dir = TempDir::new("local").expect("Couldn't create temporary local dir");
        let local_path = local_dir.path().to_path_buf();

        Command::new("git")
            .args(["init", &remote_path])
            .output()
            .expect("failed to init git repo");

        File::create(remote_dir.path().join("first_file.txt"))
            .expect("Couldn't create empty test file for git");

        Command::new("git")
            .args(["add", "first_file.txt"])
            .current_dir(remote_dir.path())
            .output()
            .expect("failed to add git file");

        Command::new("git")
            .args(["commit", "-m", "\"Test commit\""])
            .current_dir(remote_dir.path())
            .output()
            .expect("failed to commit git file");

        let worker = GitWorker::new(
            local_path,
            remote_path.clone().into_owned(),
            "test".into(),
            SecretString::new("test".into()),
        )?;

        File::create(remote_dir.path().join("new_file.txt"))
            .expect("Couldn't create empty test file for git");

        Command::new("git")
            .args(["add", "new_file.txt"])
            .current_dir(remote_dir.path())
            .output()
            .expect("failed to add git file");

        Command::new("git")
            .args(["commit", "-m", "\"Test commit\""])
            .current_dir(remote_dir.path())
            .output()
            .expect("failed to add git file");

        worker.update_repo()?;

        assert!(local_dir.path().join("new_file.txt").exists());

        // Test pulling from the remote again to check that the no updates needed
        // case works
        worker.update_repo()?;

        Ok(())
    }

    #[tokio::test]
    async fn test_create_branch() -> Result<()> {
        let remote_dir = TempDir::new("remote").expect("Couldn't create temporary remote dir");
        let remote_path = remote_dir.path().to_string_lossy();

        let local_dir = TempDir::new("local").expect("Couldn't create temporary local dir");
        let local_path = local_dir.path().to_path_buf();

        Command::new("git")
            .args(["init", &remote_path])
            .output()
            .expect("failed to init git repo");

        File::create(remote_dir.path().join("first_file.txt"))
            .expect("Couldn't create empty test file for git");

        Command::new("git")
            .args(["add", "first_file.txt"])
            .current_dir(remote_dir.path())
            .output()
            .expect("failed to add git file");

        Command::new("git")
            .args(["commit", "-m", "\"Test commit\""])
            .current_dir(remote_dir.path())
            .output()
            .expect("failed to commit git file");

        let worker = GitWorker::new(
            local_path,
            remote_path.clone().into_owned(),
            "test".into(),
            SecretString::new("test".into()),
        )?;

        worker.create_branch("feature_branch")?;

        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(local_dir.path())
            .output()
            .expect("failed to init git repo");

        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "feature_branch\n"
        );

        worker.checkout_branch("main")?;

        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(local_dir.path())
            .output()
            .expect("failed to init git repo");

        assert_eq!(String::from_utf8(output.stdout).unwrap(), "main\n");

        Ok(())
    }

    #[tokio::test]
    async fn test_commit() -> Result<()> {
        let remote_dir = TempDir::new("remote").expect("Couldn't create temporary remote dir");
        let remote_path = remote_dir.path().to_string_lossy();

        let local_dir = TempDir::new("local").expect("Couldn't create temporary local dir");
        let local_path = local_dir.path().to_path_buf();

        Command::new("git")
            .args(["init", "--bare", &remote_path])
            .output()
            .expect("failed to init git repo");

        let worker = GitWorker::new(
            local_path,
            remote_path.clone().into_owned(),
            "test".into(),
            SecretString::new("test".into()),
        )?;

        File::create(local_dir.path().join("second_file.txt"))
            .expect("Couldn't create empty test file for git");

        worker.add_and_commit(&["second_file.txt"], "commit")?;
        worker.push("main")?;

        let local_dir = TempDir::new("local").expect("Couldn't create temporary local dir");
        let local_path = local_dir.path().to_path_buf();

        GitWorker::new(
            local_path,
            remote_path.clone().into_owned(),
            "test".into(),
            SecretString::new("test".into()),
        )?;

        assert!(local_dir.path().join("second_file.txt").exists());

        Ok(())
    }
}