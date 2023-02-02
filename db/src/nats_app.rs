use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use log::info;

use crate::connection::establish_sqlite_connection;
use crate::schema::nats_apps;

#[derive(
    Queryable, Identifiable, Insertable, Clone, Debug, PartialEq, Default, Serialize, Deserialize,
)]
#[diesel(table_name = nats_apps)]
pub struct NatsApp {
    pub id: i32,
    pub app_name: String,
    pub pi_id: i32,
    pub organization_id: i32,
    pub organization_name: String,
    pub nats_server_uri: String,
    pub nats_ws_uri: String,
    pub mqtt_broker_host: String,
    pub mqtt_broker_port: i32,
}

impl From<printnanny_api_client::models::PiNatsApp> for NatsApp {
    fn from(obj: printnanny_api_client::models::PiNatsApp) -> NatsApp {
        NatsApp {
            id: obj.id,
            app_name: obj.app_name.unwrap_or_else(|| "unknown".to_string()),
            pi_id: obj.pi,
            organization_id: obj.organization.id,
            organization_name: obj.organization.name,
            nats_server_uri: obj.nats_server_uri,
            nats_ws_uri: obj.nats_ws_uri,
            mqtt_broker_host: obj.mqtt_broker_host,
            mqtt_broker_port: obj.mqtt_broker_port,
        }
    }
}

#[derive(Clone, Debug, PartialEq, AsChangeset)]
#[diesel(table_name = nats_apps)]
pub struct UpdateNatsApp {
    pub app_name: Option<String>,
    pub pi_id: Option<i32>,
    pub organization_id: Option<i32>,
    pub organization_name: Option<String>,
    pub nats_server_uri: Option<String>,
    pub nats_ws_uri: Option<String>,
    pub mqtt_broker_host: Option<String>,
    pub mqtt_broker_port: Option<i32>,
}

impl From<printnanny_api_client::models::PiNatsApp> for UpdateNatsApp {
    fn from(obj: printnanny_api_client::models::PiNatsApp) -> UpdateNatsApp {
        let organization = *obj.organization;
        UpdateNatsApp {
            app_name: obj.app_name,
            organization_name: Some(organization.name),
            organization_id: Some(organization.id),
            pi_id: Some(obj.pi),
            nats_server_uri: Some(obj.nats_server_uri),
            nats_ws_uri: Some(obj.nats_ws_uri),
            mqtt_broker_host: Some(obj.mqtt_broker_host),
            mqtt_broker_port: Some(obj.mqtt_broker_port),
        }
    }
}

impl NatsApp {
    pub fn get_id(connection_str: &str) -> Result<i32, diesel::result::Error> {
        use crate::schema::nats_apps::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);
        let result: i32 = nats_apps.select(id).first(connection)?;
        Ok(result)
    }
    pub fn get(connection_str: &str) -> Result<NatsApp, diesel::result::Error> {
        use crate::schema::nats_apps::dsl::*;

        let connection = &mut establish_sqlite_connection(connection_str);
        let result: NatsApp = nats_apps.order_by(id).first::<NatsApp>(connection)?;
        info!("printnanny_edge_db::nats_app::NatsApp get {:#?}", &result);
        Ok(result)
    }
    pub fn insert(connection_str: &str, row: NatsApp) -> Result<(), diesel::result::Error> {
        let mut connection = establish_sqlite_connection(connection_str);
        info!(
            "printnanny_edge_db::nats_app::NatsApp attempting to insert row: {:#?}",
            &row
        );
        let updated = diesel::insert_into(nats_apps::dsl::nats_apps)
            .values(row)
            .execute(&mut connection)?;
        info!("printnanny_edge_db::nats_app::NatsApp created {}", &updated);
        Ok(())
    }
    pub fn update(
        connection_str: &str,
        row_id: i32,
        row: UpdateNatsApp,
    ) -> Result<(), diesel::result::Error> {
        let mut connection = establish_sqlite_connection(connection_str);
        info!(
            "printnanny_edge_db::nats_app::NatsApp attempting to update row with id={} : {:#?}",
            &row_id, &row
        );
        let result = diesel::update(nats_apps::table.filter(nats_apps::id.eq(row_id)))
            .set(row)
            .execute(&mut connection)?;
        info!(
            "printnanny_edge_db::nats_app::NatsAppwith id={} updated",
            &result
        );
        Ok(())
    }
}
