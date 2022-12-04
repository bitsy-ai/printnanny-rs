use anyhow::Result;
use env_logger;
use log::info;
use zbus::export::futures_util::{future::try_join_all, StreamExt};
use zbus_systemd::zvariant::OwnedObjectPath;

async fn receive_all_signals(
    unit: (
        String,
        String,
        String,
        String,
        String,
        String,
        OwnedObjectPath,
        u32,
        String,
        OwnedObjectPath,
    ),
) -> Result<()> {
    let (
        unit_name,
        unit_description,
        load_state,
        active_state,
        sub_state,
        _follow_unit,
        unit_object_path,
        _job_id,
        _job_type,
        job_object_path,
    ) = unit;
    let connection = zbus::Connection::system().await?;
    let manager_proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
    manager_proxy.subscribe().await?;

    let unit_proxy =
        zbus_systemd::systemd1::UnitProxy::new(&connection, unit_object_path.clone()).await?;
    let mut stream = unit_proxy.receive_all_signals().await?;
    info!("Subscribed to signals for {:?}", unit_object_path);
    while let Some(signal) = stream.next().await {
        info!("Received signal: {:?}", signal)
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let connection = zbus::Connection::system().await?;
    let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;

    let units = proxy.list_units().await?;

    let mut tasks = Vec::with_capacity(units.len());
    for unit_result in units {
        tasks.push(tokio::spawn(receive_all_signals(unit_result)))
    }

    // let subscribers = units
    //     .iter()
    //     .map(|result| async {
    //         let (
    //             unit_name,
    //             unit_description,
    //             load_state,
    //             active_state,
    //             sub_state,
    //             _follow_unit,
    //             unit_object_path,
    //             _job_id,
    //             _job_type,
    //             job_object_path,
    //         ) = result;

    //         let unit_proxy = zbus_systemd::systemd1::UnitProxy::new(&connection, unit_object_path)
    //             .await
    //             .unwrap();

    //         let stream = unit_proxy.receive_all_signals().await.unwrap();
    //         tokio::spawn(async {
    //             while let Some(signal) = stream.next().await {
    //                 info!("Received signal: {:?}", signal)
    //             }
    //         })
    //     })
    //     .map(flatten);

    let futures = try_join_all(tasks).await;

    Ok(())
}
