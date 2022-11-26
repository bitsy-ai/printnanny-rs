// This module contains misc utils that should probably be dbus messages someday
use std::collections::HashMap;
use std::process;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{error::CommandError, settings::SystemdUnit};

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

/// encodes states of the same state machine that ActiveState covers, but knows more fine-grained states that are unit-type-specific.
/// https://www.freedesktop.org/wiki/Software/systemd/dbus/ SubState property
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum SystemdSubState {}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemctlUnit {
    // TODO
    // pub sub_state: SystemdSubSubState, // encodes states of the same state machine that ActiveState covers, but knows more fine-grained states that are unit-type-specific.
    pub id: String,
    pub fragment_path: String,
    pub load_state: SystemdLoadState, // state value that reflects whether the configuration file of this unit has been loaded
    pub load_error: (String, String), // a pair of strings. If the unit failed to load (as encoded in LoadState, see above), then this will include a D-Bus error pair consisting of the error ID and an explanatory human readable string of what happened. If it loaded successfully, this will be a pair of empty strings.
    pub active_state: SystemdActiveState, // a state value that reflects whether the unit is currently active or not
    pub unit_file_state: SystemdUnit, // encodes the install state of the unit file of FragmentPath.
}
