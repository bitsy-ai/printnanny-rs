#[macro_use] extern crate clap;

use anyhow::{ Result };
use env_logger::Builder;
use log::{ LevelFilter};
use clap::{ 
    Arg, Command
};
use rocket_dyn_templates::Template;

use git_version::git_version;

use printnanny_dash::issue;
use printnanny_dash::auth;
use printnanny_dash::debug;
use printnanny_dash::home;

use printnanny_services::config::ConfigFormat;
use printnanny_services::janus::{ JanusAdminEndpoint, janus_admin_api_call };
use printnanny_services::mqtt::{ MQTTWorker };
use printnanny_cli::config::{ConfigAction, handle_check_license};
use printnanny_api_client::models;

#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::new();
    let app_name = "printnanny";

    let version = Box::leak(format!("{} {}", crate_version!(), git_version!()).into_boxed_str());

    let app = Command::new(app_name)
        .subcommand_required(true)
        .author(crate_authors!())
        .about(crate_description!())
        .version(&version[..])
        .arg(Arg::new("v")
        .short('v')
        .multiple_occurrences(true)
        .help("Sets the level of verbosity"))

        // dash
        .subcommand(Command::new("dash")
            .author(crate_authors!())
            .about(crate_description!())
            .version(&version[..]))

        // janusadmin
        .subcommand(Command::new("janus-admin")
            .author(crate_authors!())
            .about(crate_description!())
            .version(&version[..])
            .arg_required_else_help(true)
            .about("Interact with Janus admin/monitoring APIs https://janus.conf.meetecho.com/docs/auth.html#token")
            .arg(Arg::new("endpoint")
                .possible_values(JanusAdminEndpoint::possible_values())
                .help("Janus admin/monitoring API endpoint")
                .default_value("add-token")
                .ignore_case(true)
            ) 
            .arg(Arg::new("plugins")
                .takes_value(true)
                .required_if_eq_any(&[
                    ("endpoint", "addtoken"),
                    ("endpoint", "add-token"),
                    ("endpoint", "AddToken"),
                ])
                .use_value_delimiter(true)
                .help("Commaseparated list of plugins used to scope token access.")
                .default_value("janus.plugin.echotest,janus.plugin.streaming")
                    ))
        // config
        .subcommand(Command::new("config")
            .author(crate_authors!())
            .about(crate_description!())
            .version(&version[..])
            .arg_required_else_help(true)
            .about("Interact with PrintNanny config")
            .subcommand(Command::new("get")
                .author(crate_authors!())
                .about(crate_description!())
                .version(&version[..])
                .about("Print PrintNanny config to console")
                .arg(Arg::new("key").required(true))
                .arg(Arg::new("format")
                    .short('F')
                    .long("format")
                    .takes_value(true)
                    .possible_values(ConfigFormat::possible_values())
                    .default_value("toml")
                    .help("Overwrite any existing configuration")
                )
            )
            .subcommand(Command::new("show")
                .author(crate_authors!())
                .about(crate_description!())
                .version(&version[..])
                .about("Print PrintNanny config to console")
                .arg(Arg::new("format")
                    .short('F')
                    .long("format")
                    .takes_value(true)
                    .possible_values(ConfigFormat::possible_values())
                    .default_value("toml")
                    .help("Overwrite any existing configuration")
                )            
            )
            .subcommand(Command::new("init")
                .author(crate_authors!())
                .about(crate_description!())
                .version(&version[..])
                .about("Initialize PrintNanny config")
                .arg(Arg::new("force")
                    .short('f')
                    .long("force")
                    .takes_value(false)
                    .help("Overwrite any existing configuration")
                )
                .arg(Arg::new("format")
                    .short('F')
                    .long("format")
                    .takes_value(true)
                    .possible_values(ConfigFormat::possible_values())
                    .default_value("toml")
                    .help("Overwrite any existing configuration")
                )
                .arg(Arg::new("output")
                .short('o')
                .long("output")
                .required(true)
                .takes_value(true)
                .help("Write generated config to output file")
            ))
            .subcommand(Command::new("generate-keys")
                .author(crate_authors!())
                .about(crate_description!())
                .version(&version[..])
                .about("Generate PrintNanny keypair")
                .arg(Arg::new("force")
                    .short('f')
                    .long("force")
                    .takes_value(false)
                    .help("Overwrite existing keys")
                )
                .arg(Arg::new("output")
                .short('o')
                .long("output")
                .required(true)
                .takes_value(true)
                .help("Write generated config to output file")
            )))
        // mqtt <subscribe|publish>
        .subcommand(Command::new("event")
            .author(crate_authors!())
            .about(crate_description!())
            .version(&version[..])
            .about("Interact with MQTT pub/sub service")
            .subcommand_required(true)
            .subcommand(
                Command::new("publish")
                .arg(Arg::new("data")
                    .short('d')
                    .long("data")
                    .takes_value(true)
            ))
            .subcommand(
                Command::new("subscribe")
            ))

        .subcommand(Command::new("remote")
            .author(crate_authors!())
            .about(crate_description!())
            .version(&version[..])
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
        )
        .subcommand(Command::new("check-license")
            .author(crate_authors!())
            .about(crate_description!())
            .version(&version[..])
            .about("Exchange license key for a short-lived PrintNanny API credential")
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
        Some(("dash", _)) => {
            rocket::build()
            .mount("/", home::routes())
            .mount("/debug", debug::routes())
            .mount("/issue", issue::routes())
            .mount("/login", auth::routes())
            .attach(Template::fairing())
            .launch()
            .await?;
        },
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
        Some(("check-license", _)) => {
            handle_check_license().await?;
        },
        _ => {}
    };
    Ok(())
}
