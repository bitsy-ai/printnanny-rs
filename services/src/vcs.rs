use std::fs;
use std::path::PathBuf;

use git2::{DiffFormat, DiffOptions, Repository};
use log::info;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::settings::SettingsFormat;

#[derive(Error, Debug)]
pub enum VersionControlledSettingsError {
    #[error("Failed to write {path} - {error}")]
    WriteIOError { path: String, error: std::io::Error },
    #[error("Failed to read {path} - {error}")]
    ReadIOError { path: String, error: std::io::Error },
    #[error("Failed to copy {src:?} to {dest:?} - {error}")]
    CopyIOError {
        src: PathBuf,
        dest: PathBuf,
        error: std::io::Error,
    },
    #[error(transparent)]
    PrintNannyCloudDataError(#[from] git2::Error),
}

pub trait VersionControlledSettings {
    type SettingsModel: Serialize;
    fn get_git_repo(&self) -> Result<Repository, git2::Error> {
        let settings = PrintNannySettings::new().unwrap();
        Repository::open(self.settings.paths.settings_dir)
    }
    fn get_git_diff_options(&self) -> DiffOptions {
        DiffOptions::new()
            .force_text(true)
            .old_prefix("old")
            .new_prefix("new")
    }
    fn git_diff(&self, repo: &Path) -> Result<String, git2::Error> {
        let repo = self.get_git_repo()?;
        let diffopts = self.get_git_diff_options();
        let mut lines: Vec<String> = vec![];
        repo.diff_index_to_workdir(None, diffopts)
            .print(DiffFormat::Patch, |_delta, _hunk, line| {
                lines.push(str::from_utf8(line.content()).unwrap())
            });
        Ok(lines.join("\n"))
    }
    fn write_settings(&self, content: &str) -> Result<(), IoError> {
        let output = self.get_settings_file();
        fs::write(output, content)?;
        info!("Wrote settings to {}", output.display());
        Ok(())
    }
    fn git_add_all(&self) -> Result<(), git2::Error> {
        let repo = self.get_git_repo()?;
        let mut index = repo.index()?;
        index.add_all(["."], git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
    }

    fn git_head_commit_parent_count(&self) -> Result<usize, git2::Error> {
        let repo = self.get_git_repo()?;
        let head = repo.head()?;
        let head_commit = head.peel_to_commit()?;
        head_commit.parent_count()
    }

    fn get_git_commit_message(&self) -> Result<String, git2::Error> {
        let settings_filename = self.get_settings_file().file_name().unwrap();
        let commit_parent_count = self.git_head_commit_parent_count()? + 1; // add 1 to git count of parent commits
        format!(
            "PrintNanny updated {} - revision #{}",
            settings_filename, commit_parent_count
        )
    }

    fn git_commit(&self, commit_msg: Option<String>) -> Result<git2::Oid, git2::Error> {
        &self.git_add_all()?;
        let repo = self.get_git_repo()?;
        let mut index = repo.index()?;
        let oid = index.write_tree()?;
        let signature = repo.signature()?;
        let parent_commit = repo.head()?.peel_to_commit()?;
        let tree = repo.find_tree(oid)?;
        let commit_msg = commit_msg.unwrap_or_else(self.get_git_commit_message()?);
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

    fn git_revert(&self, commit: Option<git2::Commit>) -> Result<(), git2::Error> {
        let repo = self.get_git_repo()?;
        let commit = commit.unwrap_or_else(|| repo.head()?.peel_to_commit()?);
        repo.revert(commit, None)
    }

    fn save(
        &self,
        content: &str,
        commit_msg: Option<String>,
    ) -> Result<(), VersionControlledSettingsError> {
        self.pre_save()?;
        self.write_settings(content)?;
        self.git_add_all()?;
        self.git_commit(commit_msg)?;
        self.post_save()?;
    }

    fn get_settings_format(&self) -> SettingsFormat;
    fn get_settings_file(&self) -> PathBuf;

    fn pre_save(&self) -> Result<()>;
    fn post_save(&self) -> Result<()>;
    fn validate(&self) -> bool;
}
