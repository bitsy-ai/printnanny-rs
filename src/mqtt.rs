use std::convert::TryFrom;
use std::fs::File;
use std::io::BufReader;

use chrono;
use rumqttc::{MqttOptions, AsyncClient, QoS, Transport, TlsConfiguration };
use anyhow::{ anyhow, Context, Result };
use log::{ error };
use serde::{Serialize, Deserialize};
use jsonwebtoken::{encode, Header, Algorithm, EncodingKey};
use crate::config::DeviceInfo;
use crate::keypair::KeyPair;

/// Our claims struct, it needs to derive `Serialize` and/or `Deserialize`
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Claims {
    aud: String, // Google Cloud Project id
    iat: i64, // Issued At (as UTC timestamp)
    exp: i64, // Expiration
}

#[derive(Debug, Clone)]
pub struct MQTTWorker {
    claims: Claims,
    desired_config_topic: String,
    mqttoptions: MqttOptions, 
}

fn encode_jwt(keypair: &KeyPair, claims: &Claims) -> Result<String> {
    let key = EncodingKey::from_ec_pem(&keypair.read_private_key()?)
        .context(format!("Failed to encode EC pem from {:#?}", &keypair))?;
    let result = encode(&Header::new(Algorithm::ES256), &claims, &key)?;
    Ok(result)
}

impl MQTTWorker {

    pub async fn new() -> Result<MQTTWorker> {
        let mut config = DeviceInfo::new()?;
        config = config.refresh().await?;
        if config.device.is_some() || config.keypair.is_some(){
            let device = config.device.unwrap();
            let cloudiot_device = device.cloudiot_device.unwrap();
            let desired_config_topic = cloudiot_device.desired_config_topic.unwrap();
            let keypair = config.keypair.unwrap();

            let mqtt_hostname = cloudiot_device.mqtt_bridge_hostname.as_ref().unwrap().to_string();
            let mqtt_port = u16::try_from(*cloudiot_device.mqtt_bridge_port.as_ref().unwrap())?;
            let mqtt_client_id = cloudiot_device.mqtt_client_id.as_ref().unwrap().to_string();
    
            let iat = chrono::offset::Utc::now().timestamp(); // issued at (seconds since epoch)
            let exp = iat + 86400; // 24 hours later
            let claims = Claims { iat: iat, exp: exp, aud: config.gcp_project };
            let token = encode_jwt(&keypair, &claims)?;

            let mut mqttoptions = MqttOptions::new(
                &mqtt_client_id, 
                &mqtt_hostname,
                mqtt_port
            );
            mqttoptions.set_keep_alive(5);
            mqttoptions.set_credentials("unused", &token);

            // configure tls
            let mut roots = rustls::RootCertStore::empty();
            let file = File::open(&keypair.ca_certs_path)
                .context(format!("Failed to read file {:?}", &keypair.ca_certs_path))?;

            let mut bufreader = BufReader::new(file);
            let certs = rustls_pemfile::read_all(&mut bufreader)?;
            for cert in certs {
                match cert {
                    rustls_pemfile::Item::X509Certificate(bytes) => roots.add(&rustls::Certificate(bytes)),
                    other => {
                        error!("Unrecognized root certificate format {:#?}", other);
                        Ok(())
                    }
                }?;
            };

            let mut rustls_client_config = rustls::ClientConfig::new();
            rustls_client_config.root_store = roots;
            rustls_client_config.versions = vec!(rustls::ProtocolVersion::TLSv1_2);
            let mqtt_tls_config = TlsConfiguration::from(rustls_client_config);
            mqttoptions.set_transport(Transport::tls_with_config(mqtt_tls_config));

            let result = MQTTWorker{
                claims,
                desired_config_topic,
                mqttoptions
            };
            Ok(result)
        } else {
            Err(anyhow!("Device is not registered. Please run `printnanny setup` and try again."))
        }
    }


    pub async fn run(self) -> Result<()> {


        
        let (client, mut eventloop) = AsyncClient::new(self.mqttoptions.clone(), 64);

        client.subscribe(&self.desired_config_topic, QoS::AtLeastOnce).await.unwrap();
        loop {
            let notification = eventloop.poll().await.unwrap();
            println!("Received = {:?}", notification);
        }


    }
}
