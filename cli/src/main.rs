#[macro_use] extern crate clap;
use std::process::{ Command, Stdio };

use anyhow::{ Result };
use env_logger::Builder;
use log::{ LevelFilter};
use clap::{ 
    Arg, App, AppSettings
};

use printnanny_services::config::{ PrintNannyConfig};
use printnanny_services::janus::{ JanusAdminEndpoint, janus_admin_api_call };
use printnanny_services::mqtt::{ MQTTWorker };
use printnanny_services::versioninfo::VersionInfo;
use printnanny_cli::device::{DeviceCmd, DeviceAction };
use printnanny_cli::config::{ConfigAction};
use printnanny_api_client::models;

#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::new();
    let app_name = "printnanny";

    let app = App::new(app_name)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .author(crate_authors!())
        .about(crate_description!())
        .version(crate_version!())
        .arg(Arg::new("v")
        .short('v')
        .multiple_occurrences(true)
        .help("Sets the level of verbosity"))
        // janusadmin
        .subcommand(App::new("janus-admin")
            .author(crate_authors!())
            .about(crate_description!())
            .version(crate_version!())
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("Interact with Janus admin/monitoring APIs https://janus.conf.meetecho.com/docs/auth.html#token")
            .arg(Arg::new("endpoint")
                .possible_values(JanusAdminEndpoint::possible_values())
                .help("Janus admin/monitoring API endpoint")
                .default_value("add-token")
                .ignore_case(true)
            ) 
            .arg(Arg::new("plugins")
                .long("plugins")
                .takes_value(true)
                .required_if_eq_any(&[
                    ("endpoint", "addtoken"),
                    ("endpoint", "add-token"),
                    ("endpoint", "AddToken"),
                ])
                .use_delimiter(true)
                .help("Commaseparated list of plugins used to scope token access.")
                .default_value("janus.plugin.echotest,janus.plugin.streaming")
                    ))
        // config
        .subcommand(App::new("config")
            .author(crate_authors!())
            .about(crate_description!())
            .version(crate_version!())
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("Interact with PrintNanny config")
            .subcommand(App::new("show")
                .author(crate_authors!())
                .about(crate_description!())
                .version(crate_version!())
                .about("Print PrintNanny config to console"))
            .subcommand(App::new("init")
                .author(crate_authors!())
                .about(crate_description!())
                .version(crate_version!())
                .about("Initialize PrintNanny config")
                .arg(Arg::new("force")
                    .short('f')
                    .long("force")
                    .help("Overwrite any existing configuration")
                )))
        // mqtt <subscribe|publish>
        .subcommand(App::new("event")
            .author(crate_authors!())
            .about(crate_description!())
            .version(crate_version!())
            .about("Interact with MQTT pub/sub service")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(
                App::new("publish")
                .arg(Arg::new("data")
                    .short('d')
                    .long("data")
                    .takes_value(true)
            ))
            .subcommand(
                App::new("subscribe")
            ))
        .subcommand(App::new("version")
            .author(crate_authors!())
            .about(crate_description!())
            .version(crate_version!())
            .about("Get VersionInfo for PrintNanny components"))

        .subcommand(App::new("remote")
            .author(crate_authors!())
            .about(crate_description!())
            .version(crate_version!())
            .about("Run pre-configured remote event/command handler")
            .arg(Arg::new("event")
                .help("JSON-serialized PrintNanny Event. See /api/events schema for supported events")
                .short('e')
                .long("event")
                .required(true)
                .takes_value(true))
            .arg(Arg::new("dryrun")
                .help("Print output but do not run. Ansible playbooks executed with --check flag")
                .short('d')
                .takes_value(false)
                .long("dryrun"))
        );
    
    let app_m = app.get_matches();


    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'printnanny v v v' or 'printnanny vvv' vs 'printnanny v'
    let verbosity = app_m.occurrences_of("v");
    match verbosity {
        0 => builder.filter_level(LevelFilter::Warn).init(),
        1 => builder.filter_level(LevelFilter::Info).init(),
        2 => builder.filter_level(LevelFilter::Debug).init(),
        _ => builder.filter_level(LevelFilter::Trace).init(),
    };

    match app_m.subcommand() {
        Some(("event", sub_m)) => {
            match sub_m.subcommand() {
                Some(("subscribe", _event_m)) => {
                    let worker = MQTTWorker::new(
                    ).await?;
                    worker.subscribe().await?;
                }
                Some(("publish", event_m)) => {
                    let worker = MQTTWorker::new(
                    ).await?;
                    let data = event_m.value_of("data").expect("Expected --data argument passed");
                    let event: models::PolymorphicEventCreateRequest = serde_json::from_str(data).expect("Failed to deserialize event data");
                    let value: serde_json::Value = serde_json::to_value(event)?;
                    worker.publish(value).await?;
                },
                _ => panic!("Expected publish|subscribe subcommand")
            }
        },
        Some(("config", subm)) => {
            ConfigAction::handle(subm)?;
        },
        Some(("janus-admin", sub_m)) => {
            let endpoint: JanusAdminEndpoint = sub_m.value_of_t("endpoint").unwrap_or_else(|e| e.exit());
            let res = janus_admin_api_call(
                endpoint,
            ).await?;
            println!("{}", res);

        },
        Some(("system-update", _sub_m)) => {
            let mut cmd =
            Command::new("systemctl")
            .args(&["start", "printnanny-update"])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();

            let status = cmd.wait();
            println!("System update exited with status {:?}", status);
        },
        Some(("version", _sub_m)) => {
            let versioninfo = VersionInfo::new();
            println!("{}", serde_json::to_string_pretty(&versioninfo)?);
        },
        _ => {}
    }
    Ok(())
}
