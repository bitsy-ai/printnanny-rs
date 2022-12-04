// use anyhow::Result;
// use env_logger;
// use log::info;
// use zbus::export::futures_util::{future::try_join_all, StreamExt};
// use zbus_systemd::zvariant::OwnedObjectPath;

// async fn receive_all_signals(
//     unit: (
//         String,
//         String,
//         String,
//         String,
//         String,
//         String,
//         OwnedObjectPath,
//         u32,
//         String,
//         OwnedObjectPath,
//     ),
// ) -> Result<()> {
//     let (
//         unit_name,
//         unit_description,
//         load_state,
//         active_state,
//         sub_state,
//         _follow_unit,
//         unit_object_path,
//         _job_id,
//         _job_type,
//         job_object_path,
//     ) = unit;
//     let connection = zbus::Connection::system().await?;
//     let manager_proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
//     manager_proxy.subscribe().await?;

//     let unit_proxy =
//         zbus_systemd::systemd1::UnitProxy::new(&connection, unit_object_path.clone()).await?;
//     let mut stream = unit_proxy.receive_active_state_changed().await;
//     info!("Subscribed to properties for {:?}", unit_object_path);

//     let tasks = vec![while let Some(change) = stream.next().await {
//         let result = change.get().await?;
//         info!("Received signal: {:?}", result);
//     }];

//     try_join_all(tasks).await;

//     Ok(())
// }

// #[tokio::main]
// async fn main() -> Result<()> {
//     env_logger::init();

//     let connection = zbus::Connection::system().await?;
//     let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;

//     let unit_names: Vec<String> = vec![
//         // "cloud-config.service",
//         // "cloud-final.service",
//         // "cloud-init-local.service",
//         // "janus-gateway.service",
//         "klipper.service".into(),
//         // "nginx.service",
//         "moonraker.service".into(),
//         "octoprint.service".into(),
//         "printnanny-cloud-sync.service".into(),
//         "printnanny-edge-nats.service".into(),
//         "printnanny-nats-server.service".into(),
//         "printnanny-dash.service".into(),
//         "syncthing@printnanny.service".into(),
//     ];

//     let units = proxy.list_units_by_names(unit_names.to_vec()).await?;

//     let mut tasks = Vec::with_capacity(units.len());
//     for unit_result in units {
//         tasks.push(tokio::spawn(receive_all_signals(unit_result)))
//     }

//     let mut res = Vec::with_capacity(tasks.len());
//     for f in tasks.into_iter() {
//         res.push(f.await?);
//     }
//     info!("Finished tasks: {:#?}", res);
//     // let subscribers = units
//     //     .iter()
//     //     .map(|result| async {
//     //         let (
//     //             unit_name,
//     //             unit_description,
//     //             load_state,
//     //             active_state,
//     //             sub_state,
//     //             _follow_unit,
//     //             unit_object_path,
//     //             _job_id,
//     //             _job_type,
//     //             job_object_path,
//     //         ) = result;

//     //         let unit_proxy = zbus_systemd::systemd1::UnitProxy::new(&connection, unit_object_path)
//     //             .await
//     //             .unwrap();

//     //         let stream = unit_proxy.receive_all_signals().await.unwrap();
//     //         tokio::spawn(async {
//     //             while let Some(signal) = stream.next().await {
//     //                 info!("Received signal: {:?}", signal)
//     //             }
//     //         })
//     //     })
//     //     .map(flatten);

//     // let futures = try_join_all(tasks).await?;

//     Ok(())
// }
