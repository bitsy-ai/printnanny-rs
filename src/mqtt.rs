use std::include_bytes;
use std::convert::TryFrom;

use chrono;
use rumqttc::{MqttOptions, AsyncClient, Client, QoS, ClientError };
use tokio::{task, time};
use anyhow::{ anyhow, Result };
use log::{ info, error, debug, warn };
use std::thread;
use serde::{Serialize, Deserialize};
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use print_nanny_client::models::Device;

use std::time::Duration;
use crate::config::LocalConfig;
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

fn publish(mut client: Client) {
    client.subscribe("hello/+/world", QoS::AtMostOnce).unwrap();
    for i in 0..10 {
        let payload = vec![1; i as usize];
        let topic = format!("hello/{}/world", i);
        let qos = QoS::AtLeastOnce;

        client.publish(topic, qos, true, payload).unwrap();
    }

    thread::sleep(Duration::from_secs(1));
}

fn encode_jwt(keypair: &KeyPair, claims: &Claims) -> Result<String> {
    let key = EncodingKey::from_ec_pem(&keypair.read_private_key()?)?;
    let result = encode(&Header::new(Algorithm::ES256), &claims, &key)?;
    Ok(result)
}

impl MQTTWorker {

    pub fn new(config: LocalConfig) -> Result<MQTTWorker> {
        if config.device.is_some() || config.keypair.is_some(){
            let device = config.device.unwrap();
            if device.desired_config_topic.is_none() {
                return Err(anyhow!("Device is not registered. Please run `printnanny setup` and try again."))
            } else {
                let desired_config_topic = device.desired_config_topic.unwrap();
                let keypair = config.keypair.unwrap();
    
                let mqtt_hostname = device.cloudiot_device.as_ref().unwrap().mqtt_bridge_hostname.as_ref().unwrap().to_string();
                let mqtt_port = u16::try_from(*device.cloudiot_device.as_ref().unwrap().mqtt_bridge_port.as_ref().unwrap())?;
                let mqtt_client_id = device.cloudiot_device.as_ref().unwrap().mqtt_client_id.as_ref().unwrap().to_string();
        
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
                
                let result = MQTTWorker{
                    claims,
                    desired_config_topic,
                    mqttoptions
                };
                Ok(result)
            }
   
        } else {
            Err(anyhow!("Device is not registered. Please run `printnanny setup` and try again."))
        }
    }

    pub async fn run(&self) -> Result<()> {

        let (mut client, mut eventloop) = AsyncClient::new(self.mqttoptions.clone(), 10);
        client.subscribe(&self.desired_config_topic, QoS::AtLeastOnce).await.unwrap();
        loop {
            let notification = eventloop.poll().await.unwrap();
            println!("Received = {:?}", notification);
        }

        // let transport = Transport::Tls(TlsConfiguration::Simple {
        //     ca: ca.to_vec(),
        //     alpn: None,
        //     client_auth: Some((client_cert.to_vec(), Key::RSA(client_key.to_vec()))),
        // });
    
        // task::spawn(async move {
        //     for i in 0..10 {
        //         let res = client.publish("hello/rumqtt", QoS::AtLeastOnce, false, vec![i; i as usize]).await;
        //         match res {
        //             Ok(res) => info!("Publish success {:?}", res),
        //             Err(e) => {
        //                 match e {
        //                     // rumqttc::SendError<rumqttc::Request>
        //                     ClientError::Request(send_error) => error!("SendError {}", send_error),
        //                     _ => error!("Publish failed"),
        //                 }
        //             }
        //         }
        //         time::sleep(Duration::from_millis(100)).await;
        //     }
        // });
        // loop {
        //     let notification = eventloop.poll().await.unwrap();
        //     println!("Received = {:?}", notification);
        // }
        // Ok(())
    }
}



// let mut mqttoptions = MqttOptions::new("rumqtt-async", "test.mosquitto.org", 1883);
// mqttoptions.set_keep_alive(5);

// let (mut client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
// client.subscribe("hello/rumqtt", QoS::AtMostOnce).await.unwrap();

// task::spawn(async move {
//     for i in 0..10 {
//         client.publish("hello/rumqtt", QoS::AtLeastOnce, false, vec![i; i as usize]).await.unwrap();
//         time::sleep(Duration::from_millis(100)).await;
//     }
// });

// loop {
//     let notification = eventloop.poll().await.unwrap();
//     println!("Received = {:?}", notification);
// }