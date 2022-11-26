use std::str::FromStr;

use serde::{Deserialize, Serialize};
use zbus_systemd::systemd1::UnitProxy;

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
}

impl FromStr for SystemdLoadState {
    type Err = String;
    fn from_str(input: &str) -> Result<SystemdLoadState, Self::Err> {
        match input {
            "loaded" => Ok(SystemdLoadState::Loaded),
            "error" => Ok(SystemdLoadState::Error),
            "masked" => Ok(SystemdLoadState::Masked),
            _ => Err(format!("Invalid value for SystemdLoadState: {}", input)),
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
    #[serde(rename = "inactive")]
    Inactive,
    #[serde(rename = "reloading")]
    Reloading,
    #[serde(rename = "loaded")]
    Loaded,
}

impl FromStr for SystemdActiveState {
    type Err = String;
    fn from_str(input: &str) -> Result<SystemdActiveState, Self::Err> {
        match input {
            "active" => Ok(SystemdActiveState::Active),
            "activating" => Ok(SystemdActiveState::Activating),
            "deactivating" => Ok(SystemdActiveState::Deactivating),
            "inactive" => Ok(SystemdActiveState::Inactive),
            "reloading" => Ok(SystemdActiveState::Reloading),
            "loaded" => Ok(SystemdActiveState::Loaded),
            _ => Err(format!("Invalid value for SystemdActiveState: {}", input)),
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
    type Err = String;
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
            _ => Err(format!("Invalid value for SystemdUnitFileState: {}", input)),
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
    ) -> Result<SystemdUnit, zbus::Error> {
        let connection = zbus::Connection::system().await?;
        let unit = UnitProxy::new(&connection, path).await?;

        let unit_file_state = unit.unit_file_state().await?;
        let load_path = unit.load_state().await?;
        let active_state = unit.active_state().await?;

        let result = SystemdUnit {
            id: unit.id().await?,
            fragment_path: unit.fragment_path().await?,
            load_state: SystemdLoadState::from_str(&load_path).unwrap(),
            active_state: SystemdActiveState::from_str(&active_state).unwrap(),
            unit_file_state: SystemdUnitFileState::from_str(&unit_file_state).unwrap(),
            load_error: unit.load_error().await?,
        };

        Ok(result)
    }
}
