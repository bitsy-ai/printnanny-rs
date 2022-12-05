use std::path::PathBuf;

use anyhow::Result;
use clap::{crate_authors, crate_description, Arg, Command};
use env_logger::Builder;
use futures_util::StreamExt;
use git_version::git_version;
use log::LevelFilter;
use log::{info, warn};
use tokio::time::{sleep, Duration};

use printnanny_dbus;
use printnanny_dbus::zbus;
use printnanny_dbus::zbus_systemd;

use printnanny_settings::printnanny_asyncapi_models::{
    SystemdUnit, SystemdUnitActiveStateChanged, SystemdUnitFileStateChanged,
};

use printnanny_nats::client::try_init_nats_client;

const DEFAULT_NATS_URI: &str = "nats://localhost:4223";
const GIT_VERSION: &str = git_version!();

async fn receive_active_state_change(unit_name: String) -> Result<()> {
    let connection = zbus::Connection::system().await?;
    let manager = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
    let unit_path = manager.get_unit(unit_name.to_string()).await?;
    let unit_proxy = zbus_systemd::systemd1::UnitProxy::new(&connection, unit_path.clone()).await?;
    let mut stream = unit_proxy.receive_active_state_changed().await;
    info!("Subscribed to {} ActiveState changes", unit_name);

    let tasks = while let Some(change) = stream.next().await {
        let result = change.get().await?;
        info!("{} ActiveState changed to {:?}", unit_name, &result);
        let unit = printnanny_dbus::systemd1::models::SystemdUnit::from_owned_object_path(
            unit_path.clone(),
        )
        .await?;
        let unit = SystemdUnit::from(unit);
    };
    Ok(())
}

async fn receive_unit_file_state_change(unit_name: String) -> Result<()> {
    let connection = zbus::Connection::system().await?;
    let manager = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
    let unit_path = manager.get_unit(unit_name.to_string()).await?;
    let unit_proxy = zbus_systemd::systemd1::UnitProxy::new(&connection, unit_path).await?;
    let mut stream = unit_proxy.receive_unit_file_state_changed().await;
    info!("Subscribed to {} UnitFileState changes", unit_name);

    let tasks = vec![while let Some(change) = stream.next().await {
        let result = change.get().await?;
        info!("Received signal: {:?}", result);
    }];
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::new();

    let app = Command::new("dbus-systemd-nats-adapter")
        .author(crate_authors!())
        .about(crate_description!())
        .version(GIT_VERSION)
        .arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .help("Sets the level of verbosity. Info: -v Debug: -vv Trace: -vvv"),
        )
        .about("Run NATS-based pub/sub workers")
        .arg(
            Arg::new("nats_server_uri")
                .long("nats-server-uri")
                .takes_value(true)
                .default_value(DEFAULT_NATS_URI),
        )
        .arg(Arg::new("nats_creds").long("nats-creds").takes_value(true));

    let app_m = app.get_matches();
    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'printnanny v v v' or 'printnanny vvv' vs 'printnanny v'
    let verbosity = app_m.occurrences_of("v");
    match verbosity {
        0 => {
            builder.filter_level(LevelFilter::Warn).init();
        }
        1 => {
            builder.filter_level(LevelFilter::Info).init();
        }
        2 => {
            builder.filter_level(LevelFilter::Debug).init();
        }
        _ => builder.filter_level(LevelFilter::Trace).init(),
    };

    let nats_server_uri = app_m.value_of("nats_server_uri").unwrap();
    let nats_creds = app_m.value_of("nats_creds").map(|v| PathBuf::from(v));

    let mut nats_client: Option<async_nats::Client> = None;
    while nats_client.is_none() {
        match try_init_nats_client(nats_server_uri, nats_creds.clone(), false).await {
            Ok(nc) => {
                nats_client = Some(nc);
            }
            Err(_) => {
                warn!(
                    "Waiting for NATS server to be available before initializing dbus subscriber threads"
                );
                sleep(Duration::from_millis(2000)).await;
            }
        }
    }

    let connection = zbus::Connection::system().await?;
    let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;

    let unit_names: Vec<String> = vec![
        // "cloud-config.service",
        // "cloud-final.service",
        // "cloud-init-local.service",
        // "janus-gateway.service",
        "klipper.service".into(),
        // "nginx.service",
        "moonraker.service".into(),
        "octoprint.service".into(),
        "printnanny-cloud-sync.service".into(),
        "printnanny-edge-nats.service".into(),
        "printnanny-nats-server.service".into(),
        "printnanny-dash.service".into(),
        "syncthing@printnanny.service".into(),
        "tailscaled.service".into(),
    ];
    let mut tasks = Vec::with_capacity(unit_names.len());
    for unit_name in unit_names {
        tasks.push(tokio::spawn(receive_active_state_change(unit_name)));
    }

    let mut res = Vec::with_capacity(tasks.len());
    for f in tasks.into_iter() {
        res.push(f.await?);
    }
    info!("Finished tasks: {:#?}", res);
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

    // let futures = try_join_all(tasks).await?;

    Ok(())
}
