use std::convert::TryFrom;
use std::fs;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono;
use clap::ArgEnum;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use log::{debug, error, info, warn};
use rumqttc::{
    AsyncClient, Event, Incoming, MqttOptions, Outgoing, Packet, Publish, QoS, Transport,
};
use serde::{Deserialize, Serialize};

use crate::config::{MQTTConfig, PrintNannyConfig};
use printnanny_api_client::models;
use printnanny_api_client::models::PolymorphicEvent;

use super::printnanny_api::ApiService;
#[derive(Copy, Eq, PartialEq, Debug, Clone, ArgEnum)]
pub enum MqttAction {
    Publish,
    Subscribe,
}

impl MqttAction {
    pub fn possible_values() -> impl Iterator<Item = clap::PossibleValue<'static>> {
        MqttAction::value_variants()
            .iter()
            .filter_map(clap::ArgEnum::to_possible_value)
    }
}

impl std::str::FromStr for MqttAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid variant: {}", s))
    }
}

/// Our claims struct, it needs to derive `Serialize` and/or `Deserialize`
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Claims {
    aud: String, // Google Cloud Project id
    iat: i64,    // Issued At (as UTC timestamp)
    exp: i64,    // Expiration
}

#[derive(Debug, Clone)]
pub struct MQTTWorker {
    service: ApiService,
    config: PrintNannyConfig,
    claims: Claims,
    config_topic: String,
    event_topic: String,
    command_topic: String,
    state_topic: String,
    mqttoptions: MqttOptions,
}

fn encode_jwt(private_key: &str, claims: &Claims) -> Result<String> {
    let contents =
        fs::read(private_key).context(format!("Failed to read file {:?}", private_key))?;
    let key = EncodingKey::from_ec_pem(&contents)
        .context(format!("Failed to encode EC pem from {:#?}", private_key))?;
    let result = encode(&Header::new(Algorithm::ES256), &claims, &key)?;
    Ok(result)
}

impl MQTTWorker {
    fn mqttoptions(
        cloudiot_device: &models::CloudiotDevice,
        config: &MQTTConfig,
        token: &str,
    ) -> Result<MqttOptions> {
        let mqtt_port = u16::try_from(cloudiot_device.mqtt_bridge_port)?;

        let mut mqttoptions = MqttOptions::new(
            &cloudiot_device.mqtt_client_id,
            &cloudiot_device.mqtt_bridge_hostname,
            mqtt_port,
        );
        mqttoptions.set_keep_alive(Duration::new(config.keepalive, 0));
        mqttoptions.set_credentials("unused", token);

        let mut roots = rustls::RootCertStore::empty();

        for cert in config.ca_certs.iter() {
            let root_ca_bytes =
                std::fs::read(cert).context(format!("Failed to read file {:?}", cert))?;
            let root_cert = rustls::Certificate(root_ca_bytes);
            roots.add(&root_cert)?;
        }

        let mut client_config = rumqttc::ClientConfig::new();
        client_config.root_store = roots;
        client_config.versions = vec![rustls::ProtocolVersion::TLSv1_2];
        mqttoptions.set_transport(Transport::tls_with_config(client_config.into()));
        Ok(mqttoptions)
    }

    pub async fn new(config: PrintNannyConfig) -> Result<MQTTWorker> {
        let service = ApiService::new(config.clone())?;
        let device = service.device_setup().await?;
        info!(
            "Initializing subscription from models::CloudiotDevice {:?}",
            device.cloudiot_device
        );
        let cloudiot_device = device.cloudiot_device.as_ref().unwrap();
        let gcp_project_id: String = cloudiot_device.gcp_project_id.clone();

        let iat = chrono::offset::Utc::now().timestamp(); // issued at (seconds since epoch)
        let exp = iat + 86400; // 24 hours later
        let claims = Claims {
            iat,
            exp,
            aud: gcp_project_id,
        };
        let token = encode_jwt(&config.mqtt.private_key, &claims)?;
        let mqttoptions = MQTTWorker::mqttoptions(&cloudiot_device, &config.mqtt, &token)?;

        let result = MQTTWorker {
            service,
            claims,
            config,
            state_topic: cloudiot_device.state_topic.clone(),
            command_topic: cloudiot_device.command_topic.clone(),
            config_topic: cloudiot_device.config_topic.clone(),
            event_topic: cloudiot_device.event_topic.clone(),
            mqttoptions,
        };
        Ok(result)
    }

    fn deserialize_event(&self, event: &Publish) -> Result<PolymorphicEvent> {
        let data = serde_json::from_slice::<PolymorphicEvent>(event.payload.as_ref())?;
        info!(
            "Deserialized command data={:?} from event={:?}",
            data, event
        );
        Ok(data)
    }

    fn handle_ping(&self, event: models::TestEvent) -> Result<()> {
        // mark ping as received
        // let req = models::TestEventRequest {
        //     status: Some(models::EventStatus::Ack),
        // };

        Ok(())
    }

    async fn handle_command(&self, event: &Publish) -> Result<()> {
        let data = self.deserialize_event(event)?;
        // match data {
        //     PolymorphicEvent::TestEvent(ref mut e) => {}
        //     _ => warn!("No handler configured for command, ignoring {:?}", data),
        // };
        Ok(())
    }

    async fn handle_event(&self, event: &Publish) -> Result<()> {
        info!("Handling event {:?}", event);
        match &event.topic {
            _ if &event.topic == &self.config_topic => {
                warn!("Ignored msg on config topic {:?}", event)
            }
            _ if &event.topic == &self.event_topic => {
                warn!("Ignored msg on event topic {:?}", event)
            }
            _ if &event.topic == &self.state_topic => {
                warn!("Ignored msg on state topic {:?}", event)
            }
            _ if self.command_topic.contains(&event.topic) => {
                self.handle_command(event).await?;
            }
            _ => warn!("Ignored published event {:?}", event),
        };
        Ok(())
    }

    pub async fn run(self) -> Result<()> {
        let (client, mut eventloop) = AsyncClient::new(self.mqttoptions.clone(), 64);
        client
            .subscribe(&self.config_topic, QoS::AtLeastOnce)
            .await
            .unwrap();
        client
            .subscribe(&self.command_topic, QoS::AtLeastOnce)
            .await
            .unwrap();
        client
            .subscribe(&self.state_topic, QoS::AtLeastOnce)
            .await
            .unwrap();
        loop {
            let notification = eventloop.poll().await?;
            match &notification {
                Event::Incoming(Packet::PingResp) => {
                    debug!("Received = {:?}", &notification)
                }
                Event::Outgoing(Outgoing::PingReq) => {
                    debug!("Received = {:?}", &notification)
                }
                Event::Incoming(Incoming::Publish(e)) => self.handle_event(&e).await?,
                _ => info!("Received = {:?}", &notification),
            }
        }
    }
}
