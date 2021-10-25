use rumqttc::{MqttOptions, AsyncClient, QoS};
use tokio::{task, time};
use std::time::Duration;
use std::error::Error;
use print_nanny_client::models::Device;
use crate::config::LocalConfig;
use crate::keypair::KeyPair;

#[derive(Debug, Clone)]
pub struct MQTTClient {
    device: Device,
    keypair: KeyPair,
    mqtt_client_id: String,
    mqtt_hostname: String,
    mqtt_port: i32, 
    mqtt_options: MqttOptions
}

impl MQTTClient {
    pub fn new(device: Device, keypair: KeyPair, mqtt_hostname: &str, mqtt_port: i32, mqtt_client_id: &str) -> Self {
        let mut mqttoptions = MqttOptions::new(mqtt_client_id.to_string(),  mqtt_hostname, mqtt_port as u16);
        mqttoptions.set_keep_alive(5);
        Self { 
            device: device, 
            keypair: keypair,
            mqtt_hostname: mqtt_hostname.to_string(), 
            mqtt_port: mqtt_port, 
            mqtt_client_id: mqtt_client_id.to_string(),
            mqtt_options: mqttoptions
        }
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