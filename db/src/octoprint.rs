use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use log::info;

use crate::connection::establish_sqlite_connection;
use crate::schema::octoprint_servers;

#[derive(
    Queryable, Identifiable, Insertable, Clone, Debug, PartialEq, Default, Serialize, Deserialize,
)]
#[diesel(table_name = octoprint_servers)]
pub struct OctoPrintServer {
    pub id: i32,
    pub user_id: i32,
    pub pi_id: i32,
    pub octoprint_url: String,
    pub base_path: String,
    pub venv_path: String,
    pub pip_path: String,
    pub api_key: Option<String>,
    pub octoprint_version: Option<String>,
    pub pip_version: Option<String>,
    pub printnanny_plugin_version: Option<String>,
}

impl From<printnanny_api_client::models::OctoPrintServer> for OctoPrintServer {
    fn from(obj: printnanny_api_client::models::OctoPrintServer) -> OctoPrintServer {
        OctoPrintServer {
            id: obj.id,
            user_id: obj.user,
            pi_id: obj.pi,
            octoprint_url: obj.base_url,
            base_path: obj.base_path,
            venv_path: obj.venv_path,
            pip_path: obj.pip_path,
            api_key: obj.api_key,
            octoprint_version: obj.octoprint_version,
            pip_version: obj.pip_version,
            printnanny_plugin_version: obj.printnanny_plugin_version,
        }
    }
}

#[derive(Clone, Debug, PartialEq, AsChangeset)]
#[diesel(table_name = octoprint_servers)]
pub struct UpdateOctoPrintServer {
    pub api_key: Option<String>,
    pub octoprint_version: Option<String>,
    pub pip_version: Option<String>,
    pub printnanny_plugin_version: Option<String>,
}

impl From<printnanny_api_client::models::OctoPrintServer> for UpdateOctoPrintServer {
    fn from(obj: printnanny_api_client::models::OctoPrintServer) -> UpdateOctoPrintServer {
        UpdateOctoPrintServer {
            api_key: obj.api_key,
            octoprint_version: obj.octoprint_version,
            pip_version: obj.pip_version,
            printnanny_plugin_version: obj.printnanny_plugin_version,
        }
    }
}

impl OctoPrintServer {
    pub fn get_id(connection_str: &str) -> Result<i32, diesel::result::Error> {
        use crate::schema::octoprint_servers::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);
        let result: i32 = octoprint_servers.select(id).first(connection)?;
        Ok(result)
    }
    pub fn get(connection_str: &str) -> Result<OctoPrintServer, diesel::result::Error> {
        use crate::schema::octoprint_servers::dsl::*;

        let connection = &mut establish_sqlite_connection(connection_str);
        let result: OctoPrintServer = octoprint_servers
            .order_by(id)
            .first::<OctoPrintServer>(connection)?;
        // let result = pis.order_by(id).first(&mut connection)?;
        info!(
            "printnanny_edge_db::cloud::OctoPrintServer get {:#?}",
            &result
        );
        Ok(result)
    }
    pub fn insert(connection_str: &str, row: OctoPrintServer) -> Result<(), diesel::result::Error> {
        let mut connection = establish_sqlite_connection(connection_str);

        let updated = diesel::insert_into(octoprint_servers::dsl::octoprint_servers)
            .values(row)
            .execute(&mut connection)?;
        info!(
            "printnanny_edge_db::cloud::OctoPrintServer created {}",
            &updated
        );
        Ok(())
    }
    pub fn update(
        connection_str: &str,
        pi_id: i32,
        changeset: UpdateOctoPrintServer,
    ) -> Result<(), diesel::result::Error> {
        let mut connection = establish_sqlite_connection(connection_str);
        let result =
            diesel::update(octoprint_servers::table.filter(octoprint_servers::id.eq(pi_id)))
                .set(changeset)
                .execute(&mut connection)?;
        info!(
            "printnanny_edge_db::cloud::OctoPrintServer updated {}",
            &result
        );
        Ok(())
    }
}
