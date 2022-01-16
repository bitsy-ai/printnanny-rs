use std::process::{ Command, Stdio };
use std::path::{ PathBuf};

use anyhow::{ Result };
use env_logger::Builder;
use log::LevelFilter;
use clap::{ 
    Arg, App, AppSettings, SubCommand, 
    value_t, crate_version, crate_authors, crate_description
};

use printnanny_services::printnanny_api::{ ApiConfig, read_model_json };
use printnanny_services::janus::{ JanusAdminEndpoint, janus_admin_api_call };
use printnanny_services::mqtt::{ MQTTWorker, MqttAction };
use printnanny_cli::device::{DeviceCmd, DeviceAction };
use printnanny_cli::license::{ LicenseCmd, LicenseAction};

#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::new();
    let app_name = "printnanny";

    let app = App::new(app_name)
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .author(crate_authors!())
        .about(crate_description!())
        .arg(Arg::with_name("v")
        .short("v")
        .multiple(true)
        .help("Sets the level of verbosity"))
        .arg(Arg::with_name("api_config")
            .long("api-config")
            .takes_value(true)
            .help("Path to api_config.json")
            .default_value("https://print-nanny.com"))
        .arg(Arg::with_name("api_url")
            .long("api-url")
            .takes_value(true)
            .conflicts_with("api_config")
            .help("Base PrintNanny url")
            .default_value("https://print-nanny.com"))
        .arg(Arg::with_name("api_token")
            .long("api-token")
            .takes_value(true)
            .conflicts_with("api_config")
            .help("Base PrintNanny api token"))
        .arg(Arg::with_name("data-dir")
            .short("d")
            .long("data-dir")
            .takes_value(true)
            .help("Path to Print Nanny installation")
            .default_value("/opt/printnanny/data"))
        // janusadmin
        .subcommand(SubCommand::with_name("janus-admin")
            .about("Interact with Janus admin/monitoring APIs https://janus.conf.meetecho.com/docs/auth.html#token")
            .arg(Arg::with_name("host")
                .long("host")
                .short("h")
                .takes_value(true)
                .default_value("http://localhost:7088/admin"))
            .arg(Arg::with_name("endpoint")
                .possible_values(&JanusAdminEndpoint::variants())
                .help("Janus admin/monitoring API endpoint")
                .default_value("janus.plugin.echotest,janus.plugin.streaming")
                .case_insensitive(true)
            ) 
            .arg(Arg::with_name("plugins")
                .long("plugins")
                .takes_value(true)
                .required_ifs(&[
                    ("endpoint", "addtoken"),
                    ("endpoint", "AddToken"),
                ])
                .use_delimiter(true)
                .help("Commaseparated list of plugins used to scope token access.")
                .default_value("janus.plugin.echotest,janus.plugin.streaming")
                    )
            .arg(Arg::with_name("token")
                .hide_env_values(true)
                .long("token")
                .takes_value(true)
                .required_ifs(&[
                    ("endpoint", "addtoken"),
                    ("endpoint", "AddToken"),
                    ("endpoint", "removetoken"),
                    ("endpoint", "RemoveToken")
                ])
                .env("JANUS_TOKEN")
            )
            .arg(Arg::with_name("admin_secret")
                .hide_env_values(true)
                .long("adminsecret")
                .takes_value(true)
                .required_ifs(&[
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
        .subcommand(SubCommand::with_name("factsd")
            .about("REST API JSON for Ansible facts.d")
            .arg(Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true)
            ))
        // device
        .subcommand(SubCommand::with_name("device")
            .about("Interact with device REST API")
            // model
            .arg(Arg::with_name("action")
                .possible_values(&DeviceAction::variants())
                .case_insensitive(true)
            )
            .arg(Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true)
            ))
        // license
        .subcommand(SubCommand::with_name("license")
            .about("Interact with device REST API")
            // model
            .arg(Arg::with_name("action")
                .possible_values(&LicenseAction::variants())
                .case_insensitive(true)
                .required(true)
            )
            .arg(Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true)
            ))
        // mqtt <subscribe|publish>
        .subcommand(SubCommand::with_name("mqtt")
            .arg(Arg::with_name("ca_certs")
                .long("ca-certs")
                .takes_value(true)
                .required(true)
                .env("MQTT_CA_CERTS"))
            .arg(Arg::with_name("private_key")
                .long("private-key")
                .takes_value(true)
                .required(true)
                .env("MQTT_PRIVATE_KEY"))
            .arg(Arg::with_name("public_key")
                .long("public-key")
                .takes_value(true)
                .required(true)
                .env("MQTT_PUBLIC_KEY"))
            .arg(Arg::with_name("action")
                .possible_values(&MqttAction::variants())
                .case_insensitive(true)
            ))

        .subcommand(SubCommand::with_name("monitor")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(
                SubCommand::with_name("start")
                .about("Start Print Nanny monitoring service"))
            .subcommand(
                SubCommand::with_name("stop")
                .about("Stop Print Nanny monitoring service"))
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

    let data_dir = app_m.value_of("data-dir").unwrap();
    let base_path = app_m.value_of("base_url").unwrap();
    let bearer_access_token = app_m.value_of("api_token").map(|api_token| api_token.to_string());

    let api_config: ApiConfig = match app_m.value_of("api_config"){
        Some(config_file) => read_model_json::<ApiConfig>(&PathBuf::from(config_file))?,
        None => ApiConfig{ base_path: base_path.to_string(), bearer_access_token }
    };

    
    match app_m.subcommand() {
        ("mqtt", Some(sub_m)) => {
            let action = value_t!(sub_m, "action", MqttAction).unwrap_or_else(|e| e.exit());
            let private_key = sub_m.value_of("public_key").unwrap();
            let public_key = sub_m.value_of("private_key").unwrap();
            let ca_certs = sub_m.value_of("ca_certs").unwrap();
            match action {
                MqttAction::Subscribe => {
                    let worker = MQTTWorker::new(
                        api_config,
                        data_dir,
                        private_key,
                        public_key,
                        ca_certs
                    ).await?;
                    worker.run().await?;
                },
                MqttAction::Publish => unimplemented!("mqtt publish is not implemented yet")
            }
        },

        ("license", Some(sub_m)) => {
            let action = value_t!(sub_m, "action", LicenseAction).unwrap_or_else(|e| e.exit());
            let cmd = LicenseCmd::new(action, api_config, data_dir).await?;
            let result = cmd.handle().await?;
            println!("{}", result)
        },
        ("device", Some(sub_m)) => {
            let action = value_t!(sub_m, "action", DeviceAction).unwrap_or_else(|e| e.exit());
            let cmd = DeviceCmd::new(action, api_config, data_dir).await?;
            let result = cmd.handle().await?;
            println!("{}", result)
        }, 
        ("janus-admin", Some(sub_m)) => {
            let endpoint = value_t!(sub_m, "endpoint", JanusAdminEndpoint).unwrap_or_else(|e| e.exit());
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

        ("monitor", Some(sub_m)) => {
            match sub_m.subcommand() {
                ("start", Some(_)) => println!("Starting Print Nanny monitoring"),
                ("stop", Some(_)) => println!("Stopping Print Nanny monitoring"),
                _ => panic!("Received unrecognized subcommand")
            };
        }

        ("system-update", Some(_sub_m)) => {
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
