use diesel::prelude::*;
use diesel::sql_types::SqlType;
use diesel_derive_enum::DbEnum;

use serde::{Deserialize, Serialize};

use printnanny_api_client;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, DbEnum, SqlType)]
pub enum SbcEnum {
    #[serde(rename = "rpi4")]
    Rpi4,
}

impl Default for SbcEnum {
    fn default() -> Self {
        SbcEnum::Rpi4
    }
}

impl From<printnanny_api_client::models::SbcEnum> for SbcEnum {
    fn from(obj: printnanny_api_client::models::SbcEnum) -> SbcEnum {
        match obj {
            printnanny_api_client::models::SbcEnum::Rpi4 => SbcEnum::Rpi4,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, DbEnum, Debug, SqlType)]
pub enum PreferredDnsType {
    #[serde(rename = "multicast")]
    Multicast,
    #[serde(rename = "tailscale")]
    Tailscale,
}

impl Default for PreferredDnsType {
    fn default() -> Self {
        PreferredDnsType::Multicast
    }
}

impl From<printnanny_api_client::models::PreferredDnsType> for PreferredDnsType {
    fn from(obj: printnanny_api_client::models::PreferredDnsType) -> PreferredDnsType {
        match obj {
            printnanny_api_client::models::PreferredDnsType::Multicast => {
                PreferredDnsType::Multicast
            }
            printnanny_api_client::models::PreferredDnsType::Tailscale => {
                PreferredDnsType::Tailscale
            }
        }
    }
}

impl From<PreferredDnsType> for printnanny_api_client::models::PreferredDnsType {
    fn from(obj: PreferredDnsType) -> printnanny_api_client::models::PreferredDnsType {
        match obj {
            PreferredDnsType::Multicast => {
                printnanny_api_client::models::PreferredDnsType::Multicast
            }
            PreferredDnsType::Tailscale => {
                printnanny_api_client::models::PreferredDnsType::Tailscale
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, DbEnum, SqlType)]
pub enum RecordingStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "inprogress")]
    InProgress,
    #[serde(rename = "done")]
    Done,
}

impl Default for RecordingStatus {
    fn default() -> Self {
        RecordingStatus::Pending
    }
}
