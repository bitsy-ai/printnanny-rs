use std::path::PathBuf;
use thiserror::Error;

use printnanny_edge_db::diesel;

use printnanny_api_client::apis::accounts_api;
use printnanny_api_client::apis::crash_reports_api;
use printnanny_api_client::apis::devices_api;
use printnanny_api_client::apis::octoprint_api;
use printnanny_api_client::apis::videos_api;
use printnanny_api_client::apis::Error as ApiError;

use printnanny_settings::figment;
use printnanny_settings::sys_info;
use printnanny_settings::toml;

use printnanny_settings::error::{PrintNannySettingsError, VersionControlledSettingsError};

use printnanny_nats_client::error::NatsError;

#[derive(Error, Debug)]
pub enum VideoRecordingError {
    #[error(transparent)]
    SqliteDBError(#[from] diesel::result::Error),

    #[error(transparent)]
    VideosCreateError(#[from] ApiError<videos_api::VideosCreateError>),

    #[error(transparent)]
    VideoRecordingPartsCreateError(#[from] ApiError<videos_api::VideoPartsCreateError>),

    #[error(transparent)]
    VideosPartialUpdateError(#[from] ApiError<videos_api::VideosPartialUpdateError>),

    #[error(transparent)]
    VideoRecordingsUpdateOrCreateError(
        #[from] ApiError<videos_api::VideoRecordingsUpdateOrCreateError>,
    ),

    #[error(transparent)]
    VideosRetrieveError(#[from] ApiError<videos_api::VideosRetrieveError>),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum VideoRecordingSyncError {
    #[error(transparent)]
    PrintNannySettingsError(#[from] PrintNannySettingsError),

    #[error("mp4 upload url was not set for VideoRecording with id={id} file_name={file_name}")]
    UploadUrlNotSet { id: String, file_name: String },
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    SqliteDBError(#[from] diesel::result::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    VideoRecordingsUpdateOrCreateError(#[from] VideoRecordingError),
}

#[derive(Error, Debug)]
pub enum PrintNannyCamSettingsError {
    #[error(transparent)]
    FigmentError(#[from] figment::error::Error),
    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum IoError {
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
}

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error(transparent)]
    JsonSerError(#[from] serde_json::Error),
    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),

    #[error(transparent)]
    CrashReportsCreateError(#[from] ApiError<crash_reports_api::CrashReportsCreateError>),

    #[error(transparent)]
    CrashReportsPartialUpdateError(
        #[from] ApiError<crash_reports_api::CrashReportsPartialUpdateError>,
    ),

    #[error(transparent)]
    PisRetrieveError(#[from] ApiError<devices_api::PisRetrieveError>),

    #[error(transparent)]
    PiUpdateOrCreateError(#[from] ApiError<devices_api::PiUpdateOrCreateError>),

    #[error(transparent)]
    PisPartialUpdateError(#[from] ApiError<devices_api::PisPartialUpdateError>),

    #[error(transparent)]
    PisLicenseZipRetrieveError(#[from] ApiError<devices_api::PisLicenseZipRetrieveError>),

    #[error(transparent)]
    SystemInfoCreateError(#[from] ApiError<devices_api::PisSystemInfoCreateError>),

    #[error(transparent)]
    SystemInfoUpdateOrCreateError(#[from] ApiError<devices_api::SystemInfoUpdateOrCreateError>),

    #[error(transparent)]
    OctoprintPartialUpdateError(#[from] ApiError<octoprint_api::OctoprintPartialUpdateError>),

    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    UserRetrieveError(#[from] ApiError<accounts_api::AccountsUserRetrieveError>),

    #[error(transparent)]
    Accounts2faAuthTokenCreateError(
        #[from] ApiError<accounts_api::Accounts2faAuthTokenCreateError>,
    ),
    #[error(transparent)]
    Accounts2faAuthEmailCreateError(
        #[from] ApiError<accounts_api::Accounts2faAuthEmailCreateError>,
    ),

    #[error(transparent)]
    VideoRecordingSyncError(#[from] VideoRecordingSyncError),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("License fingerprint mismatch (expected {expected:?}, found {active:?})")]
    InvalidLicense { expected: String, active: String },

    #[error("Failed to fingerprint {path:?} got stderr {stderr:?}")]
    FingerprintError { path: PathBuf, stderr: String },

    #[error(transparent)]
    PersistError(#[from] tempfile::PersistError),

    #[error(transparent)]
    ProcError(#[from] procfs::ProcError),

    #[error(transparent)]
    SysInfoError(#[from] sys_info::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    FigmentError(#[from] figment::error::Error),
    #[error(transparent)]
    PrintNannySettingsError(#[from] PrintNannySettingsError),

    #[error("Setup incomplete, failed to read {field:?} {detail:?}")]
    SetupIncomplete {
        detail: Option<String>,
        field: String,
    },

    #[error(transparent)]
    VersionControlledSettingsError(#[from] VersionControlledSettingsError),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error(transparent)]
    SqliteDBError(#[from] diesel::result::Error),

    #[error("Error running diesel SQLIte migrations: {msg}")]
    SQLiteMigrationError { msg: String },

    #[error(transparent)]
    TaskJoinError(#[from] tokio::task::JoinError),
}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Failed to parse key=value pair from systemctl output")]
    SystemctlParse { output: String },

    #[error("Failed to deserialize {payload} with error {error}")]
    SerdeJson {
        payload: String,
        error: String,
        source: serde_json::Error,
    },
    #[error(transparent)]
    JsonSerError(#[from] serde_json::Error),
    #[error(transparent)]
    NatsError(#[from] NatsError),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
}
