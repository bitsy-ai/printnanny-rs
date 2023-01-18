use diesel::prelude::*;
use diesel::sql_types::SqlType;
use diesel_derive_enum::DbEnum;

use serde::{Deserialize, Serialize};

use printnanny_api_client;

#[derive(Debug, PartialEq, Serialize, Deserialize, DbEnum, SqlType)]
#[diesel(sqlite_type(name = "SbcEnum"))]
pub enum SbcEnum {
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

#[derive(PartialEq, Serialize, Deserialize, DbEnum, Debug, SqlType)]
#[diesel(sqlite_type(name = "PreferredDnsType"))]
#[diesel(table_name = pi)]
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
#[diesel(sqlite_type(name = "RecordingStatus"))]
#[diesel(table_name = video_recordings)]
pub enum RecordingStatus {
    Pending,
    InProgress,
    Done,
}

impl Default for RecordingStatus {
    fn default() -> Self {
        RecordingStatus::Pending
    }
}
