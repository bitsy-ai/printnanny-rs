use std::convert::TryFrom;
use std::fs;
use std::path::{ PathBuf };
use std::time::Duration;

use chrono;
use log::{ debug, info };
use rumqttc::{MqttOptions, AsyncClient, QoS, Transport, Event, Packet, Outgoing };
use anyhow::{ Context, Result };
use serde::{Serialize, Deserialize};
use jsonwebtoken::{encode, Header, Algorithm, EncodingKey};

use printnanny_api_client::models::{ Device, CloudiotDevice };
use crate::paths::PrintNannyPath;
use crate::services::device::PrintNannyService;

/// Our claims struct, it needs to derive `Serialize` and/or `Deserialize`
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Claims {
    aud: String, // Google Cloud Project id
    iat: i64, // Issued At (as UTC timestamp)
    exp: i64, // Expiration
}

#[derive(Debug, Clone)]
pub struct MQTTWorker {
    service: PrintNannyService::<Device>,
    claims: Claims,
    config_topic: String,
    task_topic: String,
    mqttoptions: MqttOptions, 
}

fn encode_jwt(keypath: &PathBuf, claims: &Claims) -> Result<String> {
    let contents = fs::read(keypath)
        .context(format!("Failed to read file {:?}", keypath))?;
    let key = EncodingKey::from_ec_pem(&contents)
        .context(format!("Failed to encode EC pem from {:#?}", &keypath))?;
    let result = encode(&Header::new(Algorithm::ES256), &claims, &key)?;
    Ok(result)
}

impl MQTTWorker {

    fn mqttoptions(cloudiot_device: &CloudiotDevice, paths: &PrintNannyPath, token: &str) -> Result<MqttOptions> {
        let mqtt_port = u16::try_from(cloudiot_device.mqtt_bridge_port)?;

        let mut mqttoptions = MqttOptions::new(
            &cloudiot_device.mqtt_client_id, 
            &cloudiot_device.mqtt_bridge_hostname,
            mqtt_port
        );
        mqttoptions.set_keep_alive(Duration::new(5, 0));
        mqttoptions.set_credentials("unused", &token);

        let mut roots = rustls::RootCertStore::empty();

        let root_ca_bytes =  std::fs::read(&paths.ca_cert)
            .context(format!("Failed to read file {:?}", &paths.ca_cert))?;

        let root_cert = rustls::Certificate(root_ca_bytes);
        roots.add(&root_cert)?;

        let mut client_config = rumqttc::ClientConfig::new();
        client_config.root_store = roots;
        client_config.versions = vec!(rustls::ProtocolVersion::TLSv1_2);
        mqttoptions.set_transport(Transport::tls_with_config(client_config.into()));
        Ok(mqttoptions)
    }

    pub async fn new(config: &str) -> Result<MQTTWorker> {
        let service = PrintNannyService::<Device>::new(config)?;
        let device = service.retrieve().await?;
        let cloudiot_device = device.cloudiot_device.as_ref().unwrap();
        let gcp_project_id: String = cloudiot_device.gcp_project_id.clone();

        let iat = chrono::offset::Utc::now().timestamp(); // issued at (seconds since epoch)
        let exp = iat + 86400; // 24 hours later
        let claims = Claims { iat: iat, exp: exp, aud: gcp_project_id };
        let token = encode_jwt(&service.paths.private_key, &claims)?;
        let mqttoptions = MQTTWorker::mqttoptions(&cloudiot_device, &service.paths, &token)?;

        let result = MQTTWorker{
            claims: claims,
            config_topic: cloudiot_device.config_topic.clone(),
            task_topic: cloudiot_device.task_topic.clone(),
            mqttoptions: mqttoptions,
        };
        Ok(result)
    }

    pub async fn run(self) -> Result<()> {
        let (client, mut eventloop) = AsyncClient::new(self.mqttoptions.clone(), 64);
        client.subscribe(&self.config_topic, QoS::AtLeastOnce).await.unwrap();
        client.subscribe(&self.task_topic, QoS::AtLeastOnce).await.unwrap();
        loop {
            let notification = eventloop.poll().await?;
            match notification {
                Event::Incoming(Packet::PingResp) => {
                    debug!("Received = {:?}", notification)
                },
                Event::Outgoing(Outgoing::PingReq)=> {
                    debug!("Received = {:?}", notification)
                },
                _ => info!("Received = {:?}", notification)
            }
        }
    }
}
