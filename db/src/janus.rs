use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use log::info;

use crate::connection::establish_sqlite_connection;
use crate::schema::webrtc_edge_servers;

#[derive(
    Queryable, Identifiable, Insertable, Clone, Debug, PartialEq, Default, Serialize, Deserialize,
)]
#[diesel(table_name = webrtc_edge_servers)]
pub struct WebrtcEdgeServer {
    pub id: i32,
    pub pi_id: i32,
    pub admin_secret: String,
    pub admin_port: i32,
    pub admin_url: String,
    pub api_token: String,
    pub api_domain: String,
    pub api_port: i32,
    pub pt: i32,
    pub rtp_domain: String,
    pub video_rtp_port: i32,
    pub data_rtp_port: i32,
    pub rtpmap: String,
    pub ws_port: i32,
}

impl From<printnanny_api_client::models::WebrtcStream> for WebrtcEdgeServer {
    fn from(obj: printnanny_api_client::models::WebrtcStream) -> WebrtcEdgeServer {
        WebrtcEdgeServer {
            id: obj.id,
            pi_id: obj.pi,
            admin_secret: obj.admin_secret,
            admin_port: obj.admin_port,
            admin_url: obj.admin_url,
            api_token: obj.api_token,
            api_domain: obj.api_domain,
            api_port: obj.api_port,
            pt: obj.pt,
            rtp_domain: obj.rtp_domain,
            video_rtp_port: obj.video_rtp_port.unwrap_or(20001),
            data_rtp_port: obj.data_rtp_port.unwrap_or(20003),
            rtpmap: obj.rtpmap,
            ws_port: obj.ws_port,
        }
    }
}

impl WebrtcEdgeServer {
    pub fn get_id(connection_str: &str) -> Result<i32, diesel::result::Error> {
        use crate::schema::webrtc_edge_servers::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);
        let result: i32 = webrtc_edge_servers.select(id).first(connection)?;
        Ok(result)
    }
    pub fn get(connection_str: &str) -> Result<WebrtcEdgeServer, diesel::result::Error> {
        use crate::schema::webrtc_edge_servers::dsl::*;

        let connection = &mut establish_sqlite_connection(connection_str);
        let result: WebrtcEdgeServer = webrtc_edge_servers
            .order_by(id)
            .first::<WebrtcEdgeServer>(connection)?;
        info!(
            "printnanny_edge_db::janus::WebrtcEdgeServer get {:#?}",
            &result
        );
        Ok(result)
    }
    pub fn insert(
        connection_str: &str,
        row: WebrtcEdgeServer,
    ) -> Result<(), diesel::result::Error> {
        let mut connection = establish_sqlite_connection(connection_str);

        let updated = diesel::insert_into(webrtc_edge_servers::dsl::webrtc_edge_servers)
            .values(row)
            .execute(&mut connection)?;
        info!(
            "printnanny_edge_db::janus::WebrtcEdgeServer created {}",
            &updated
        );
        Ok(())
    }
}
