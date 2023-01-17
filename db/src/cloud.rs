use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};

use crate::schema::pi;
use crate::schema::printnanny_cloud_api_config;
use crate::schema::user;
use printnanny_api_client;

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum, Deserialize, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = pi)]
pub struct Pi {
    pub id: i32,
    pub last_boot: Option<String>,
    pub hostname: String,
    pub sbc: SbcEnum,
    pub created_dt: String,
    pub moonraker_api_url: String,
    pub mission_control_url: String,
    pub octoprint_url: String,
    pub swupdate_url: String,
    pub syncthing_url: String,
    pub preferred_dns: PreferredDnsType,
}

impl From<printnanny_api_client::models::Pi> for Pi {
    fn from(obj: printnanny_api_client::models::Pi) -> Pi {
        let urls = *obj.urls;
        let preferred_dns = match obj.network_settings {
            Some(network_settings) => match network_settings.preferred_dns {
                Some(result) => result.into(),
                None => PreferredDnsType::Multicast,
            },
            None => PreferredDnsType::Multicast,
        };
        Pi {
            id: obj.id,
            last_boot: obj.last_boot,
            hostname: obj.hostname,
            sbc: obj.sbc.into(),
            created_dt: obj.created_dt,
            moonraker_api_url: urls.moonraker_api,
            mission_control_url: urls.mission_control,
            octoprint_url: urls.octoprint,
            swupdate_url: urls.swupdate,
            syncthing_url: urls.syncthing,
            preferred_dns,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum, Deserialize, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = user)]
pub struct User {
    pub email: String,
    pub id: i32,
    #[serde(rename = "first_name", skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(rename = "last_name", skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
}

impl From<printnanny_api_client::models::User> for User {
    fn from(obj: printnanny_api_client::models::User) -> User {
        User {
            id: obj.id,
            email: obj.email,
            first_name: obj.first_name,
            last_name: obj.last_name,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = printnanny_cloud_api_config)]
#[diesel(primary_key(user_id))]
pub struct PrintNannyCloudApiConfig {
    pub user_id: i32,
    pub base_url: String,
    pub bearer_access_token: Option<String>,
}
