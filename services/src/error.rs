use std::path::PathBuf;
use thiserror::Error;

use printnanny_api_client::apis::alert_settings_api;
use printnanny_api_client::apis::auth_api;
use printnanny_api_client::apis::config_api;
use printnanny_api_client::apis::devices_api;
use printnanny_api_client::apis::janus_api;
use printnanny_api_client::apis::octoprint_api;
use printnanny_api_client::apis::users_api;
use printnanny_api_client::apis::Error as ApiError;

#[derive(Error, Debug)]
pub enum PrintNannyConfigError {
    #[error("Failed to read {path:?}. Please download a license from https://printnanny.ai/dashboard/ and save to {path:?}")]
    LicenseMissing { path: String },

    #[error("Command {cmd} exited with code {code:?} stdout: {stdout} stderr: {stderr}")]
    CommandError {
        cmd: String,
        code: Option<i32>,
        stdout: String,
        stderr: String,
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
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    OpenSSLError(#[from] openssl::error::ErrorStack),

    #[error(transparent)]
    JsonSerError(#[from] serde_json::Error),
    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),
    #[error(transparent)]
    FigmentError(#[from] figment::error::Error),
}

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error(transparent)]
    JsonSerError(#[from] serde_json::Error),
    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),
    #[error(transparent)]
    AlertSettingsGetOrCreateRetrieveError(
        #[from] ApiError<alert_settings_api::AlertSettingsGetOrCreateRetrieveError>,
    ),

    #[error(transparent)]
    ApiConfigRetreiveError(#[from] ApiError<config_api::ApiConfigRetreiveError>),
    #[error(transparent)]
    AuthTokenCreateError(#[from] ApiError<auth_api::AuthTokenCreateError>),
    #[error(transparent)]
    AuthEmailCreateError(#[from] ApiError<auth_api::AuthEmailCreateError>),
    #[error(transparent)]
    CloudiotDeviceUpdateOrCreateError(
        #[from] ApiError<devices_api::CloudiotDeviceUpdateOrCreateError>,
    ),

    #[error(transparent)]
    DevicesCreateError(#[from] ApiError<devices_api::DevicesCreateError>),

    #[error(transparent)]
    DevicesRetrieveError(#[from] ApiError<devices_api::DevicesRetrieveError>),

    #[error(transparent)]
    DevicesPartialUpdateError(#[from] ApiError<devices_api::DevicesPartialUpdateError>),

    #[error(transparent)]
    DevicesRetrieveHostnameError(#[from] ApiError<devices_api::DevicesRetrieveHostnameError>),
    #[error(transparent)]
    SystemInfoCreateError(#[from] ApiError<devices_api::DevicesSystemInfoCreateError>),
    #[error(transparent)]
    SystemInfoUpdateOrCreateError(#[from] ApiError<devices_api::SystemInfoUpdateOrCreateError>),

    #[error(transparent)]
    OctoprintServerUpdateOrCreateError(
        #[from] ApiError<octoprint_api::OctoprintServerUpdateOrCreateError>,
    ),

    #[error(transparent)]
    PublicKeyUpdateOrCreate(#[from] ApiError<devices_api::PublicKeyUpdateOrCreateError>),

    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    UsersRetrieveError(#[from] ApiError<users_api::UsersMeRetrieveError>),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("License fingerprint mismatch (expected {expected:?}, found {active:?})")]
    InvalidLicense { expected: String, active: String },

    #[error("Failed to fingerprint {path:?} got stderr {stderr:?}")]
    FingerprintError { path: PathBuf, stderr: String },

    #[error(transparent)]
    ProcError(#[from] procfs::ProcError),

    #[error(transparent)]
    FigmentError(#[from] figment::Error),

    #[error(transparent)]
    SysInfoError(#[from] sys_info::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    PrintNannyConfigError(#[from] PrintNannyConfigError),

    #[error("Setup incomplete, failed to read {field:?} {detail:?}")]
    SetupIncomplete {
        detail: Option<String>,
        field: String,
    },
}
