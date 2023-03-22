use std::str::FromStr;

use serde::{Deserialize, Serialize};
use zbus_systemd::systemd1::UnitProxy;

use crate::error::SystemdError;
use printnanny_os_models;

pub const PRINTNANNY_RECORDING_SERVICE_TEMPLATE: &str = "printnanny-recording-sync@";

/// State value that reflects whether the configuration file of this unit has been loaded
/// https://www.freedesktop.org/wiki/Software/systemd/dbus/ LoadState property
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum SystemdLoadState {
    #[serde(rename = "loaded")]
    Loaded,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "masked")]
    Masked,
    #[serde(rename = "not-found")]
    NotFound,
}

impl FromStr for SystemdLoadState {
    type Err = SystemdError;
    fn from_str(input: &str) -> Result<SystemdLoadState, Self::Err> {
        match input {
            "loaded" => Ok(SystemdLoadState::Loaded),
            "error" => Ok(SystemdLoadState::Error),
            "masked" => Ok(SystemdLoadState::Masked),
            "not-found" => Ok(SystemdLoadState::NotFound),
            _ => Err(SystemdError::InvalidUnitLoadState {
                state: input.to_string(),
            }),
        }
    }
}

/// State value that reflects whether the configuration file of this unit has been loaded
/// https://www.freedesktop.org/wiki/Software/systemd/dbus/ ActiveState property
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum SystemdActiveState {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "activating")]
    Activating,
    #[serde(rename = "deactivating")]
    Deactivating,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "inactive")]
    Inactive,
    #[serde(rename = "reloading")]
    Reloading,
    #[serde(rename = "loaded")]
    Loaded,
}

impl FromStr for SystemdActiveState {
    type Err = SystemdError;
    fn from_str(input: &str) -> Result<SystemdActiveState, Self::Err> {
        match input {
            "active" => Ok(SystemdActiveState::Active),
            "activating" => Ok(SystemdActiveState::Activating),
            "deactivating" => Ok(SystemdActiveState::Deactivating),
            "failed" => Ok(SystemdActiveState::Failed),
            "inactive" => Ok(SystemdActiveState::Inactive),
            "reloading" => Ok(SystemdActiveState::Reloading),
            "loaded" => Ok(SystemdActiveState::Loaded),
            _ => Err(SystemdError::InvalidUnitActiveState {
                state: input.to_string(),
            }),
        }
    }
}

/// encodes the install state of the unit file of FragmentPath.
/// https://www.freedesktop.org/wiki/Software/systemd/dbus/ UnitFileState property
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum SystemdUnitFileState {
    #[serde(rename = "enabled")]
    Enabled,
    #[serde(rename = "enabled-runtime")]
    EnabledRuntime,
    #[serde(rename = "linked")]
    Linked,
    #[serde(rename = "linked-runtime")]
    LinkedRuntime,
    #[serde(rename = "masked")]
    Masked,
    #[serde(rename = "masked-runtime")]
    MaskedRuntime,
    #[serde(rename = "static")]
    Static,
    #[serde(rename = "disabled")]
    Disabled,
    #[serde(rename = "invalid")]
    Invalid,
}

impl FromStr for SystemdUnitFileState {
    type Err = SystemdError;
    fn from_str(input: &str) -> Result<SystemdUnitFileState, Self::Err> {
        match input {
            "enabled" => Ok(SystemdUnitFileState::Enabled),
            "enabled-runtime" => Ok(SystemdUnitFileState::EnabledRuntime),
            "linked" => Ok(SystemdUnitFileState::Linked),
            "linked-runtime" => Ok(SystemdUnitFileState::LinkedRuntime),
            "masked" => Ok(SystemdUnitFileState::Masked),
            "masked-runtime" => Ok(SystemdUnitFileState::MaskedRuntime),
            "static" => Ok(SystemdUnitFileState::Static),
            "disabled" => Ok(SystemdUnitFileState::Disabled),
            "invalid" => Ok(SystemdUnitFileState::Invalid),
            _ => Err(SystemdError::InvalidUnitFileState {
                state: input.to_string(),
            }),
        }
    }
}

/// encodes states of the same state machine that ActiveState covers, but knows more fine-grained states that are unit-type-specific.
/// https://www.freedesktop.org/wiki/Software/systemd/dbus/ SubState property
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum SystemdSubState {}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdUnit {
    // TODO
    // pub sub_state: SystemdSubSubState, // encodes states of the same state machine that ActiveState covers, but knows more fine-grained states that are unit-type-specific.
    pub id: String,
    pub fragment_path: String,
    pub load_state: SystemdLoadState, // state value that reflects whether the configuration file of this unit has been loaded
    pub load_error: (String, String), // a pair of strings. If the unit failed to load (as encoded in LoadState, see above), then this will include a D-Bus error pair consisting of the error ID and an explanatory human readable string of what happened. If it loaded successfully, this will be a pair of empty strings.
    pub active_state: SystemdActiveState, // a state value that reflects whether the unit is currently active or not
    pub unit_file_state: SystemdUnitFileState, // encodes the install state of the unit file of FragmentPath.
}

impl SystemdUnit {
    pub async fn from_owned_object_path(
        path: zbus::zvariant::OwnedObjectPath,
    ) -> Result<SystemdUnit, SystemdError> {
        let connection = zbus::Connection::system().await?;
        let unit = UnitProxy::new(&connection, path.clone()).await?;

        let unit_file_state = unit.unit_file_state().await?;
        let load_path = unit.load_state().await?;
        let active_state = unit.active_state().await?;

        let load_state = SystemdLoadState::from_str(&load_path)?;
        if load_state == SystemdLoadState::NotFound {
            return Err(SystemdError::UnitNotFound {
                unit: path.to_string(),
            });
        }

        let result = SystemdUnit {
            load_state,
            id: unit.id().await?,
            fragment_path: unit.fragment_path().await?,
            active_state: SystemdActiveState::from_str(&active_state)?,
            unit_file_state: SystemdUnitFileState::from_str(&unit_file_state)?,
            load_error: unit.load_error().await?,
        };

        Ok(result)
    }
}

impl From<SystemdUnit> for printnanny_os_models::SystemdUnit {
    fn from(unit: SystemdUnit) -> printnanny_os_models::SystemdUnit {
        let active_state = match unit.active_state {
            SystemdActiveState::Active => printnanny_os_models::SystemdUnitActiveState::Active,
            SystemdActiveState::Loaded => printnanny_os_models::SystemdUnitActiveState::Loaded,
            SystemdActiveState::Activating => {
                printnanny_os_models::SystemdUnitActiveState::Activating
            }
            SystemdActiveState::Inactive => printnanny_os_models::SystemdUnitActiveState::Inactive,
            SystemdActiveState::Reloading => {
                printnanny_os_models::SystemdUnitActiveState::Reloading
            }
            SystemdActiveState::Deactivating => {
                printnanny_os_models::SystemdUnitActiveState::Deactivating
            }
            SystemdActiveState::Failed => printnanny_os_models::SystemdUnitActiveState::Failed,
        };

        let load_state = match unit.load_state {
            SystemdLoadState::Masked => printnanny_os_models::SystemdUnitLoadState::Masked,
            SystemdLoadState::Error => printnanny_os_models::SystemdUnitLoadState::Error,
            SystemdLoadState::Loaded => printnanny_os_models::SystemdUnitLoadState::Loaded,
            SystemdLoadState::NotFound => printnanny_os_models::SystemdUnitLoadState::NotMinusFound,
        };

        let unit_file_state = match unit.unit_file_state {
            SystemdUnitFileState::Enabled => printnanny_os_models::SystemdUnitFileState::Enabled,
            SystemdUnitFileState::EnabledRuntime => {
                printnanny_os_models::SystemdUnitFileState::EnabledMinusRuntime
            }
            SystemdUnitFileState::Disabled => printnanny_os_models::SystemdUnitFileState::Disabled,
            SystemdUnitFileState::Linked => printnanny_os_models::SystemdUnitFileState::Linked,
            SystemdUnitFileState::LinkedRuntime => {
                printnanny_os_models::SystemdUnitFileState::LinkedMinusRuntime
            }
            SystemdUnitFileState::Masked => printnanny_os_models::SystemdUnitFileState::Masked,
            SystemdUnitFileState::MaskedRuntime => {
                printnanny_os_models::SystemdUnitFileState::MaskedMinusRuntime
            }
            SystemdUnitFileState::Static => printnanny_os_models::SystemdUnitFileState::Static,
            SystemdUnitFileState::Invalid => printnanny_os_models::SystemdUnitFileState::Invalid,
        };

        printnanny_os_models::SystemdUnit {
            id: unit.id,
            fragment_path: unit.fragment_path,
            active_state: Box::new(active_state),
            load_state: Box::new(load_state),
            unit_file_state: Box::new(unit_file_state),
        }
    }
}
