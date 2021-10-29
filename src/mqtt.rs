use std::include_bytes;
use chrono;
use rumqttc::{MqttOptions, AsyncClient, Client, QoS, ClientError, LastWill};
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
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: i32, // Subject (whom token refers to)
    iat: i64, // Issued At (as UTC timestamp)
    exp: i64, // Expiration
}

#[derive(Debug, Clone)]
pub struct MQTTClient {
    device: Device,
    keypair: KeyPair,
    mqtt_client_id: String,
    mqtt_hostname: String,
    mqtt_port: i32, 
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

impl MQTTClient {
    pub fn new(config: LocalConfig) -> Result<MQTTClient> {
        match config.device {
            Some(device) => {
                let hostname = device.cloudiot_device.as_ref().unwrap().mqtt_bridge_hostname.as_ref().unwrap();
                let port = device.cloudiot_device.as_ref().unwrap().mqtt_bridge_port.as_ref().unwrap();
                let mqtt_client_id = device.cloudiot_device.as_ref().unwrap().mqtt_client_id.as_ref().unwrap();
                let result = MQTTClient{ 
                    device: device.clone(), 
                    keypair: config.keypair.unwrap(),
                    mqtt_hostname: hostname.to_string(), 
                    mqtt_port: *port,   
                    mqtt_client_id: mqtt_client_id.to_string()
                };
                Ok(result)
            }
            None => Err(anyhow!("Device is not registered. Please run `printnanny setup` and try again"))
        }

    }

    async fn client_loop(&self, device: &Device, keypair: &KeyPair) -> Result<()> {
        let iat = chrono::offset::Utc::now().timestamp(); // issued at (seconds since epoch)
        let exp = iat + 86400; // 24 hours later
        let claims = Claims { iat: iat, exp: exp, sub: device.id.unwrap() };
        let key = keypair.read_private_key()?;
        let mut mqttoptions = MqttOptions::new(&self.mqtt_client_id, &self.mqtt_hostname,self.mqtt_port as u16);
        mqttoptions.set_keep_alive(5);
        let token = encode(&Header::new(Algorithm::ES256), &claims, &EncodingKey::from_ec_pem(&key)?)?;

        let (mut client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
        client.subscribe("hello/rumqtt", QoS::AtLeastOnce).await.unwrap();
        loop {
            let notification = eventloop.poll().await.unwrap();
            println!("Received = {:?}", notification);
        }
        Ok(())
    }

    pub async fn run(&self) -> Result<()> {
        let config = LocalConfig::new()?;
        match config.device {
            Some(device) => {
                match config.keypair {
                    Some(keypair) => self.client_loop(&device, &keypair).await?,
                    None => {
                        return Err(anyhow!("Missing device config. Please run `printnanny setup` to configure your device first!"));
                    }
                }
            },
            None => {
                return Err(anyhow!("Missing device config. Please run `printnanny setup` to configure your device first!"))
            }
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


        Ok(())
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