use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};

use log::info;

use crate::schema::pi;
use crate::schema::user;
use crate::sql_types::*;

use crate::connection::establish_sqlite_connection;

#[derive(
    Clone, Debug, PartialEq, Default, Serialize, Deserialize, Queryable, Identifiable, AsChangeset,
)]
#[diesel(table_name = "pi")]
pub struct Pi {
    pub id: i32,
    pub last_boot: Option<String>,
    pub hostname: String,
    pub sbc: SbcEnumMapping,
    pub created_dt: String,
    pub moonraker_api_url: String,
    pub mission_control_url: String,
    pub octoprint_url: String,
    pub swupdate_url: String,
    pub syncthing_url: String,
    pub preferred_dns: PreferredDnsTypeMapping,
    pub octoprint_server_id: Option<i32>,
    pub system_info_id: Option<i32>,
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
        let octoprint_server_id = match obj.octoprint_server {
            Some(octoprint_server) => Some(octoprint_server.id),
            None => None,
        };

        let system_info_id = match obj.system_info {
            Some(system_info) => Some(system_info.id),
            None => None,
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
            octoprint_server_id,
            system_info_id,
        }
    }
}

// #[derive(AsChangeset)]
// #[diesel(table_name = pi)]
// pub struct UpdatePi {
//     pub id: i32,
//     pub last_boot: Option<String>,
//     pub hostname: String,
//     pub sbc: String,
//     pub created_dt: String,
//     pub moonraker_api_url: String,
//     pub mission_control_url: String,
//     pub octoprint_url: String,
//     pub swupdate_url: String,
//     pub syncthing_url: String,
//     pub preferred_dns: String,
//     pub octoprint_server_id: Option<i32>,
//     pub system_info_id: Option<i32>,
// }

// impl From<printnanny_api_client::models::Pi> for UpdatePi {
//     fn from(obj: printnanny_api_client::models::Pi) -> UpdatePi {
//         let urls = *obj.urls;
//         let preferred_dns = match obj.network_settings {
//             Some(network_settings) => match network_settings.preferred_dns {
//                 Some(result) => result.into(),
//                 None => PreferredDnsType::Multicast,
//             },
//             None => PreferredDnsType::Multicast,
//         };
//         let octoprint_server_id = match obj.octoprint_server {
//             Some(octoprint_server) => Some(octoprint_server.id),
//             None => None,
//         };

//         let system_info_id = match obj.system_info {
//             Some(system_info) => Some(system_info.id),
//             None => None,
//         };

//         UpdatePi {
//             id: obj.id,
//             last_boot: obj.last_boot,
//             hostname: obj.hostname,
//             sbc: obj.sbc.to_string(),
//             created_dt: obj.created_dt,
//             moonraker_api_url: urls.moonraker_api,
//             mission_control_url: urls.mission_control,
//             octoprint_url: urls.octoprint,
//             swupdate_url: urls.swupdate,
//             syncthing_url: urls.syncthing,
//             preferred_dns,
//             octoprint_server_id,
//             system_info_id,
//         }
//     }
// }

impl Pi {
    pub fn get() -> Result<Pi, diesel::result::Error> {
        let mut connection = establish_sqlite_connection();
        pi::dsl::pi.first(&mut connection)
    }
    pub fn upsert(pi: Pi) -> Result<(), diesel::result::Error> {
        let mut connection = establish_sqlite_connection();

        let updated = diesel::insert_into(pi::table)
            .values(&pi)
            .on_conflict(pi::dsl::id)
            .do_update()
            .set(&pi)
            .execute(&mut connection)?;
        info!("printnanny_edge_db::cloud::Pi updated {}", updated);
        Ok(())
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

// #[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, Queryable, Identifiable)]
// #[diesel(table_name = printnanny_cloud_api_config)]
// pub struct PrintNannyCloudApi {
//     pub id: i32,
//     pub base_url: String,
//     pub bearer_access_token: Option<String>,
// }

// impl PrintNannyCloudApiConfig {
//     pub fn insert_ignore(
//         base_url: &str,
//         bearer_access_token: &str,
//     ) -> Result<(), diesel::result::Error> {
//         let mut connection = establish_sqlite_connection();

//         diesel::insert_or_ignore_into(printnanny_cloud_api_config::table)
//             .values((
//                 printnanny_cloud_api_config::dsl::bearer_access_token.eq(bearer_access_token),
//                 printnanny_cloud_api_config::dsl::base_url.eq(base_url),
//             ))
//             .execute(&mut connection)?;

//         Ok(())
//     }
// }
