#[macro_use] extern crate clap;
use std::process::{ Command, Stdio };

use anyhow::{ Result };
use env_logger::Builder;
use log::{ info, LevelFilter};
use clap::{ 
    Arg, App, AppSettings
};

use printnanny_services::config::{ PrintNannyConfig};
use printnanny_services::janus::{ JanusAdminEndpoint, janus_admin_api_call };
use printnanny_services::mqtt::{ MQTTWorker };
use printnanny_services::remote;
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
        .arg(Arg::new("config")
            .long("config")
            .short('c')
            .takes_value(true)
            .help("Path to Config.toml (see env/ for examples)"))
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
        // api endpoints (used by ansible facts.d)
        .subcommand(App::new("factsd")
            .author(crate_authors!())
            .about(crate_description!())
            .version(crate_version!())
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("Config serializer (JSON) intended for use with Ansible facts.d")
            .arg(Arg::new("output")
                .short('o')
                .long("output")
                .takes_value(true)
            ))

        // config
        .subcommand(App::new("config")
            .author(crate_authors!())
            .about(crate_description!())
            .version(crate_version!())
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("Show PrintNanny config")
            .arg(Arg::new("action")
                .possible_values(ConfigAction::possible_values())
                .ignore_case(true)
            ))
        // device
        .subcommand(App::new("device")
            .author(crate_authors!())
            .about(crate_description!())
            .version(crate_version!())
            .setting(AppSettings::ArgRequiredElseHelp)
            .about("Interact with device REST API")
            // model
            .arg(Arg::new("action")
                .possible_values(DeviceAction::possible_values())
                .ignore_case(true)
            )
            .arg(Arg::new("output")
                .short('o')
                .long("output")
                .takes_value(true)
            ))
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
    info!("testing");

    let conf_file = app_m.value_of("config");

    let config: PrintNannyConfig = PrintNannyConfig::new(conf_file)?;

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
                        config,
                    ).await?;
                    worker.subscribe().await?;
                }
                Some(("publish", event_m)) => {
                    let worker = MQTTWorker::new(
                        config,
                    ).await?;
                    let data = event_m.value_of("data").expect("Expected --data argument passed");
                    let event: models::PolymorphicEventRequest = serde_json::from_str(data).expect("Failed to deserialize event data");
                    let value: serde_json::Value = serde_json::to_value(event)?;
                    worker.publish(value).await?;
                },
                _ => panic!("Expected publish|subscribe subcommand")
            }
        },
        Some(("config", _)) => {
            println!("{}",serde_json::to_string_pretty(&config)?);
        },
        Some(("device", sub_m)) => {
            let action: DeviceAction = sub_m.value_of_t("action").unwrap_or_else(|e| e.exit());
            let cmd = DeviceCmd::new(action, config).await?;
            let result = cmd.handle().await?;
            println!("{}", result)
        }, 
        Some(("janus-admin", sub_m)) => {
            let endpoint: JanusAdminEndpoint = sub_m.value_of_t("endpoint").unwrap_or_else(|e| e.exit());
            let janus_config = config.janus_local.expect("janus_local config is not set");
            let res = janus_admin_api_call(
                endpoint,
                &janus_config
            ).await?;
            println!("{}", res);

        },

        Some(("remote", sub_m)) => {
            let dryrun = sub_m.is_present("dryrun");
            let json_str = sub_m.value_of("event").expect("--event argument is required");
            let event: models::PolymorphicEvent = serde_json::from_str(json_str).expect("Failed to deserialize event data");
            remote::handle_event(event, config, dryrun)?;
        }

        Some(("system-update", _sub_m)) => {
            let mut cmd =
            Command::new("systemctl")
            .args(&["start", "printnannyupdater"])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();

            let status = cmd.wait();
            println!("System update exited with status {:?}", status);
        },
        _ => {}
    }
    Ok(())
}
