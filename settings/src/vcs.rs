use std::path::{Path, PathBuf};

use tokio::fs;

use async_trait::async_trait;
use git2::{DiffFormat, DiffOptions, Repository};
use log::{debug, info, warn};
use printnanny_asyncapi_models::{SettingsApp, SettingsFile};
use serde::{Deserialize, Serialize};

use crate::error::VersionControlledSettingsError;
use crate::printnanny::{GitSettings, PrintNannySettings};
use crate::SettingsFormat;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GitCommit {
    pub oid: String,
    pub header: String,
    pub message: String,
    pub ts: i64,
}

#[async_trait]
pub trait VersionControlledSettings {
    type SettingsModel: Serialize;
    async fn to_payload(
        &self,
        app: SettingsApp,
    ) -> Result<SettingsFile, VersionControlledSettingsError> {
        let file_name = self.get_settings_file().display().to_string();
        let file_format = self.get_settings_format();
        let content = match fs::read_to_string(&file_name).await {
            Ok(data) => Ok(data),
            Err(e) => Err(VersionControlledSettingsError::ReadIOError {
                path: file_name.clone(),
                error: e,
            }),
        }?;
        info!("Loaded settings from: {}", &file_name);
        Ok(SettingsFile {
            app: Box::new(app),
            file_name,
            file_format: Box::new(file_format.into()),
            content,
        })
    }

    fn config_git_repo(
        &self,
        repo: &git2::Repository,
        git_settings: &GitSettings,
    ) -> Result<(), git2::Error> {
        let config = repo.config()?;
        let mut localconfig = config.open_level(git2::ConfigLevel::Local)?;
        localconfig.set_str("user.email", &git_settings.email)?;
        localconfig.set_str("user.name", &git_settings.name)?;
        localconfig.set_str("init.defaultBranch", &git_settings.default_branch)?;
        Ok(())
    }

    fn init_git_repo(
        &self,
        target_dir: &PathBuf,
        git_settings: &GitSettings,
    ) -> Result<Repository, git2::Error> {
        let repo = git2::Repository::clone(&git_settings.remote, target_dir)?;
        self.config_git_repo(&repo, git_settings)?;
        Ok(repo)
    }

    fn get_git_repo(&self) -> Result<Repository, git2::Error> {
        let settings = PrintNannySettings::new().unwrap();
        match Repository::open(&settings.paths.settings_dir) {
            Ok(repo) => {
                debug!(
                    "Found existing git repo: {}",
                    settings.paths.settings_dir.display()
                );
                Ok(repo)
            }
            Err(e) => {
                warn!(
                    "Failed to open git repo with error={}, attempting to clone {} to {}",
                    e,
                    &settings.git.remote,
                    settings.paths.settings_dir.display()
                );
                Ok(self.init_git_repo(&settings.paths.settings_dir, &settings.git)?)
            }
        }
    }
    fn git_diff(&self) -> Result<String, git2::Error> {
        let repo = self.get_git_repo()?;
        let mut diffopts = DiffOptions::new();

        let diffopts = diffopts
            .force_text(true)
            .old_prefix("old")
            .new_prefix("new");
        let mut lines: Vec<String> = vec![];
        repo.diff_index_to_workdir(None, Some(diffopts))?.print(
            DiffFormat::Patch,
            |_delta, _hunk, line| {
                lines.push(std::str::from_utf8(line.content()).unwrap().to_string());
                true
            },
        )?;
        Ok(lines.join("\n"))
    }
    async fn read_settings(&self) -> Result<String, VersionControlledSettingsError> {
        let settings_file = self.get_settings_file();
        let result = match fs::read_to_string(&settings_file).await {
            Ok(d) => Ok(d),
            Err(e) => Err(VersionControlledSettingsError::ReadIOError {
                path: (settings_file.display()).to_string(),
                error: e,
            }),
        }?;
        Ok(result)
    }
    async fn write_settings(&self, content: &str) -> Result<(), VersionControlledSettingsError> {
        let output = self.get_settings_file();
        let parent_dir = output.parent().unwrap();
        if !parent_dir.exists() {
            match fs::create_dir_all(parent_dir).await {
                Ok(_) => {
                    info!("Created directory {}", parent_dir.display());
                    Ok(())
                }
                Err(e) => Err(VersionControlledSettingsError::WriteIOError {
                    path: parent_dir.display().to_string(),
                    error: e,
                }),
            }?;
        }
        match fs::write(&output, content).await {
            Ok(_) => Ok(()),
            Err(e) => Err(VersionControlledSettingsError::WriteIOError {
                path: output.display().to_string(),
                error: e,
            }),
        }?;
        info!("Wrote settings to {}", output.display());
        Ok(())
    }
    fn git_add_all(&self) -> Result<(), git2::Error> {
        let repo = self.get_git_repo()?;
        let mut index = repo.index()?;
        index.add_all(["."], git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    fn git_head_commit_parent_count(&self) -> Result<usize, git2::Error> {
        let repo = self.get_git_repo()?;
        let head = repo.head()?;
        let head_commit = head.peel_to_commit()?;
        Ok(head_commit.parent_count())
    }

    fn get_git_commit_message(&self) -> Result<String, git2::Error> {
        let settings_file = self.get_settings_file();
        let settings_filename = settings_file.file_name().unwrap();
        let commit_parent_count = self.git_head_commit_parent_count()? + 1; // add 1 to git count of parent commits
        Ok(format!(
            "PrintNanny updated {:?} - revision #{}",
            &settings_filename, &commit_parent_count
        ))
    }

    fn get_git_head_commit(&self) -> Result<GitCommit, git2::Error> {
        let repo = self.get_git_repo()?;
        let commit = &repo.head()?.peel_to_commit()?;
        Ok(commit.into())
    }

    fn get_rev_list(&self) -> Result<Vec<GitCommit>, git2::Error> {
        let repo = self.get_git_repo()?;
        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TIME)?;
        revwalk.push_head()?;

        revwalk.push_glob(&self.get_settings_file().display().to_string())?;
        let mut result: Vec<GitCommit> = vec![];
        for r in revwalk {
            let commit = match r {
                Ok(oid) => repo.find_commit(oid),
                Err(e) => Err(e),
            }?;
            result.push(commit.into())
        }
        Ok(result)
    }

    fn git_commit(&self, commit_msg: Option<String>) -> Result<git2::Oid, git2::Error> {
        self.git_add_all()?;
        let repo = self.get_git_repo()?;
        let mut index = repo.index()?;
        let oid = index.write_tree()?;
        let signature = repo.signature()?;
        let parent_commit = repo.head()?.peel_to_commit()?;
        let tree = repo.find_tree(oid)?;
        let commit_msg = commit_msg.unwrap_or_else(|| self.get_git_commit_message().unwrap());
        let result = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &commit_msg,
            &tree,
            &[&parent_commit],
        )?;
        info!("Committed settings with msg: {} and {}", commit_msg, oid);
        Ok(result)
    }

    fn git_revert(&self, oid: Option<git2::Oid>) -> Result<(), git2::Error> {
        let repo = self.get_git_repo()?;
        let commit = match oid {
            Some(sha) => repo.find_commit(sha)?,
            None => repo.head().unwrap().peel_to_commit()?,
        };
        repo.revert(&commit, None)
    }

    async fn git_revert_hooks(
        &self,
        oid: Option<git2::Oid>,
    ) -> Result<(), VersionControlledSettingsError> {
        self.pre_save().await?;
        self.git_revert(oid)?;
        self.pre_save().await?;
        Ok(())
    }

    async fn save_and_commit(
        &self,
        content: &str,
        commit_msg: Option<String>,
    ) -> Result<(), VersionControlledSettingsError> {
        // first, get repo (clone will run if repo is not present, which requires empty path)
        self.get_git_repo()?;
        // then run any pre-save hooks
        self.pre_save().await?;
        // write settings file
        self.write_settings(content).await?;
        // commit changes
        self.git_add_all()?;
        self.git_commit(commit_msg)?;
        // run post-save hooks
        self.post_save().await?;
        Ok(())
    }

    fn from_dir(settings_dir: &Path) -> Self::SettingsModel;

    fn get_settings_format(&self) -> SettingsFormat;
    fn get_settings_file(&self) -> PathBuf;

    async fn pre_save(&self) -> Result<(), VersionControlledSettingsError>;
    async fn post_save(&self) -> Result<(), VersionControlledSettingsError>;
    fn validate(&self) -> Result<(), VersionControlledSettingsError>;
}

impl<'repo> From<&git2::Commit<'repo>> for GitCommit {
    fn from(commit: &git2::Commit<'repo>) -> GitCommit {
        GitCommit {
            oid: commit.id().to_string(),
            header: commit.raw_header().unwrap().to_string(),
            message: commit.message().unwrap().to_string(),
            ts: commit.time().seconds(),
        }
    }
}
impl<'repo> From<git2::Commit<'repo>> for GitCommit {
    fn from(commit: git2::Commit<'repo>) -> GitCommit {
        GitCommit {
            oid: commit.id().to_string(),
            header: commit.raw_header().unwrap().to_string(),
            message: commit.message().unwrap().to_string(),
            ts: commit.time().seconds(),
        }
    }
}

impl From<&printnanny_asyncapi_models::GitCommit> for GitCommit {
    fn from(commit: &printnanny_asyncapi_models::GitCommit) -> GitCommit {
        GitCommit {
            oid: commit.oid.clone(),
            header: commit.header.clone(),
            message: commit.message.clone(),
            ts: commit.ts,
        }
    }
}

impl From<&GitCommit> for printnanny_asyncapi_models::GitCommit {
    fn from(commit: &GitCommit) -> printnanny_asyncapi_models::GitCommit {
        printnanny_asyncapi_models::GitCommit {
            oid: commit.oid.clone(),
            header: commit.header.clone(),
            message: commit.message.clone(),
            ts: commit.ts,
        }
    }
}
