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
            .required(true)
            .help("Path to Config.toml (see env/ for examples)"))
        // janusadmin
        .subcommand(App::new("janus-admin")
            .author(crate_authors!())
            .about(crate_description!())
            .version(crate_version!())
            .about("Interact with Janus admin/monitoring APIs https://janus.conf.meetecho.com/docs/auth.html#token")
            .arg(Arg::new("host")
                .long("host")
                .short('H')
                .takes_value(true)
                .default_value("http://localhost:7088/admin"))
            .arg(Arg::new("endpoint")
                .possible_values(JanusAdminEndpoint::possible_values())
                .help("Janus admin/monitoring API endpoint")
                .default_value("janus.plugin.echotest,janus.plugin.streaming")
                .ignore_case(true)
            ) 
            .arg(Arg::new("plugins")
                .long("plugins")
                .takes_value(true)
                .required_if_eq_any(&[
                    ("endpoint", "addtoken"),
                    ("endpoint", "AddToken"),
                ])
                .use_delimiter(true)
                .help("Commaseparated list of plugins used to scope token access.")
                .default_value("janus.plugin.echotest,janus.plugin.streaming")
                    )
            .arg(Arg::new("token")
                .hide_env_values(true)
                .long("token")
                .takes_value(true)
                .required_if_eq_any(&[
                    ("endpoint", "addtoken"),
                    ("endpoint", "AddToken"),
                    ("endpoint", "removetoken"),
                    ("endpoint", "RemoveToken")
                ])
                .env("JANUS_TOKEN")
            )
            .arg(Arg::new("admin_secret")
                .hide_env_values(true)
                .long("adminsecret")
                .takes_value(true)
                .required_if_eq_any(&[
                    ("endpoint", "addtoken"),
                    ("endpoint", "AddToken"),
                    ("endpoint", "removetoken"),
                    ("endpoint", "RemoveToken"),
                    ("endpoint", "listtokens"),
                    ("endpoint", "ListTokens"),
                ])
                .env("JANUS_ADMIN_SECRET")
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
            .arg(Arg::new("ca_certs")
                .long("ca-certs")
                .takes_value(true)
                .required(true)
                .env("MQTT_CA_CERTS"))
            .arg(Arg::new("private_key")
                .long("private-key")
                .takes_value(true)
                .required(true)
                .env("MQTT_PRIVATE_KEY"))
            .arg(Arg::new("public_key")
                .long("public-key")
                .takes_value(true)
                .required(true)
                .env("MQTT_PUBLIC_KEY"))
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
            let private_key = sub_m.value_of("public_key").unwrap();
            let public_key = sub_m.value_of("private_key").unwrap();
            let ca_certs = sub_m.value_of("ca_certs").unwrap();
            match action {
                MqttAction::Subscribe => {
                    let worker = MQTTWorker::new(
                        config,
                        private_key,
                        public_key,
                        ca_certs
                    ).await?;
                    worker.run().await?;
                },
                MqttAction::Publish => unimplemented!("mqtt publish is not implemented yet")
            }
        },
        Some(("device", sub_m)) => {
            let action: DeviceAction = sub_m.value_of_t("action").unwrap_or_else(|e| e.exit());
            let cmd = DeviceCmd::new(action, config).await?;
            let result = cmd.handle().await?;
            println!("{}", result)
        }, 
        Some(("janus-admin", sub_m)) => {
            let endpoint: JanusAdminEndpoint = sub_m.value_of_t("endpoint").unwrap_or_else(|e| e.exit());
            let token = sub_m.value_of("token");
            let admin_secret = sub_m.value_of("admin_secret");
            let host = sub_m.value_of("host").unwrap();
            let res = janus_admin_api_call(
                host.to_string(), 
                endpoint,
                token.map(|s| s.into()),
                admin_secret.map(|s| s.into()),
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
