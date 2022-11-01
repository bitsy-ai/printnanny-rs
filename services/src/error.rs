use std::path::PathBuf;
use thiserror::Error;

use printnanny_api_client::apis::accounts_api;
use printnanny_api_client::apis::devices_api;
use printnanny_api_client::apis::octoprint_api;
use printnanny_api_client::apis::Error as ApiError;

#[derive(Error, Debug)]
pub enum PrintNannyConfigError {
    #[error("Failed to load license from {pattern:?}. Please download a license from https://printnanny.ai/dashboard/ and save to /boot")]
    PatternNotFound { pattern: String },
    #[error("Refusing to overwrite existing file at {path:?}.")]
    FileExists { path: PathBuf },

    #[error("PRINTNANNY_CONFIG was set {path:?} but file was not found")]
    ConfigFileNotFound { path: PathBuf },

    #[error("Failed to unpack file {filename} from archive {archive:?}")]
    ArchiveMissingFile { filename: String, archive: PathBuf },

    #[error("Failed to read {path:?}. Please download a license from https://printnanny.ai/dashboard/ and save to ")]
    LicenseMissing { path: String },

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

    #[error("Failed to copy {src:?} to {dest:?} - {error}")]
    CopyIOError {
        src: PathBuf,
        dest: PathBuf,
        error: std::io::Error,
    },

    #[error("Failed to parse OctoPrintServer field: {field} {detail:?}")]
    OctoPrintServerConfigError {
        field: String,
        detail: Option<String>,
    },

    #[error("Failed to handle invalid config value {value:?}")]
    InvalidValue { value: String },
    #[error("Refusing to overwrite existing keypair at {path:?}.")]
    KeypairExists { path: PathBuf },

    #[error(transparent)]
    JsonSerError(#[from] serde_json::Error),
    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),
    #[error(transparent)]
    FigmentError(#[from] figment::error::Error),
    #[error(transparent)]
    ZipError(#[from] zip::result::ZipError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("Setup incomplete, failed to read {field:?} {detail:?}")]
    SetupIncomplete {
        detail: Option<String>,
        field: String,
    },
}

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error(transparent)]
    JsonSerError(#[from] serde_json::Error),
    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),

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
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("License fingerprint mismatch (expected {expected:?}, found {active:?})")]
    InvalidLicense { expected: String, active: String },

    #[error("Failed to fingerprint {path:?} got stderr {stderr:?}")]
    FingerprintError { path: PathBuf, stderr: String },

    #[error(transparent)]
    ProcError(#[from] procfs::ProcError),

    #[error(transparent)]
    SysInfoError(#[from] sys_info::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    FigmentError(#[from] figment::error::Error),
    #[error(transparent)]
    PrintNannyConfigError(#[from] PrintNannyConfigError),

    #[error("Setup incomplete, failed to read {field:?} {detail:?}")]
    SetupIncomplete {
        detail: Option<String>,
        field: String,
    },
}

#[derive(Error, Debug)]
pub enum NatsError {
    #[error("Connection to {path} failed")]
    UnixSocketNotFound { path: String },
    #[error("NatsConnection error {msg}")]
    NatsConnection { msg: String },

    #[error("Nats PublishError {error}")]
    PublishError { error: String },
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
