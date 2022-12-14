use thiserror::Error;

#[derive(Error, Debug)]
pub enum SystemdError {
    #[error(transparent)]
    ZbusError(#[from] zbus::Error),

    #[error("Systemd unit not found: {unit}")]
    UnitNotFound { unit: String },

    #[error("Invalid value for SystemdUnitFileState: {state}")]
    InvalidUnitFileState { state: String },
    #[error("Invalid value for SystemdActiveState: {state}")]
    InvalidUnitActiveState { state: String },
    #[error("Invalid value for SystemdLoadState: {state}")]
    InvalidUnitLoadState { state: String },
}
