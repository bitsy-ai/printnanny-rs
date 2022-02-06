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
use printnanny_services::mqtt::{ MQTTWorker, MqttAction };
use printnanny_cli::device::{DeviceCmd, DeviceAction };

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
            .about("Show PrintNanny config"))
        // device
        .subcommand(App::new("device")
            .author(crate_authors!())
            .about(crate_description!())
            .version(crate_version!())
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
        .subcommand(App::new("mqtt")
            .author(crate_authors!())
            .about(crate_description!())
            .version(crate_version!())
            .about("Interact with MQTT pub/sub service")
            .arg(Arg::new("action")
                .possible_values(MqttAction::possible_values())
                .ignore_case(true)
            ))

        .subcommand(App::new("monitor")
            .author(crate_authors!())
            .about(crate_description!())
            .version(crate_version!())
            .about("Interact with Print Nanny monitoring service")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(
                App::new("start")
                .about("Start Print Nanny monitoring service"))
            .subcommand(
                App::new("stop")
                .about("Stop Print Nanny monitoring service"))
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
        Some(("mqtt", sub_m)) => {
            let action: MqttAction = sub_m.value_of_t("action").unwrap_or_else(|e| e.exit());
            match action {
                MqttAction::Subscribe => {
                    let worker = MQTTWorker::new(
                        config,
                    ).await?;
                    worker.run().await?;
                },
                MqttAction::Publish => unimplemented!("mqtt publish is not implemented yet")
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
            let janus_config = config.janus;
            let res = janus_admin_api_call(
                endpoint,
                &janus_config
            ).await?;
            println!("{}", res);

        },

        Some(("monitor", sub_m)) => {
            match sub_m.subcommand() {
                Some(("start", _)) => println!("Starting Print Nanny monitoring"),
                Some(("stop", _)) => println!("Stopping Print Nanny monitoring"),
                _ => panic!("Received unrecognized subcommand")
            };
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
