use std::path::PathBuf;
use thiserror::Error;

use printnanny_dbus::zbus;

#[derive(Error, Debug)]
pub enum PrintNannySettingsError {
    #[error("PRINTNANNY_SETTINGS was set {path:?} but file was not found")]
    ConfigFileNotFound { path: PathBuf },

    #[error("Failed to unpack file {filename} from archive {archive:?}")]
    ArchiveMissingFile { filename: String, archive: PathBuf },

    #[error("Command {cmd} exited with code {code:?} stdout: {stdout} stderr: {stderr}")]
    CommandError {
        cmd: String,
        code: Option<i32>,
        stdout: String,
        stderr: String,
    },

    #[error("Failed to write {path} - {error}")]
    WriteIOError {
        path: PathBuf,
        error: std::io::Error,
    },

    #[error("Failed to read {path} - {error}")]
    ReadIOError {
        path: PathBuf,
        error: std::io::Error,
    },

    #[error("Failed to parse OctoPrintServer field: {field} {detail:?}")]
    OctoPrintServerConfigError {
        field: String,
        detail: Option<String>,
    },

    #[error("Failed to handle invalid config value {value:?}")]
    InvalidValue { value: String },

    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    JsonSerError(#[from] serde_json::Error),
    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),
    #[error(transparent)]
    TomlDeError(#[from] toml::de::Error),
    #[error(transparent)]
    FigmentError(#[from] figment::error::Error),
    #[error(transparent)]
    ZipError(#[from] zip::result::ZipError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    GitError(#[from] git2::Error),

    #[error(transparent)]
    TaskJoinError(#[from] tokio::task::JoinError),
}

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
    GitError(#[from] git2::Error),
    #[error(transparent)]
    ZbusError(#[from] zbus::Error),

    #[error(transparent)]
    PrintNannySettingsError(#[from] PrintNannySettingsError),
}
