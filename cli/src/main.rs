#[macro_use] extern crate clap;

use anyhow::{ Result };
use env_logger::Builder;
use log::{ LevelFilter };
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
use printnanny_cli::config::{ConfigCommand};
use printnanny_cli::os::{OsCommand};
use printnanny_gst::cam;

const GIT_VERSION: &str = git_version!();

#[derive(Clone, clap::ValueEnum, Debug, PartialEq, Serialize, Deserialize)]
pub enum EventCategory {
    Command,
    Status,
}


#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::new();
    let app_name = "printnanny";
    let app = Command::new(app_name)
        .subcommand_required(true)
        .author(crate_authors!())
        .about(crate_description!())
        .version(GIT_VERSION)
        .arg(Arg::new("v")
        .short('v')
        .multiple_occurrences(true)
        .help("Sets the level of verbosity. Info: -v Debug: -vv Trace: -vvv"))

        // cam
        .subcommand(cam::PrintNannyCam::clap_command())

        // dash
        .subcommand(Command::new("dash")
            .author(crate_authors!())
            .about("PrintNanny device dashboard and system status")
            .version(GIT_VERSION))

        // janus-admin
        .subcommand(Command::new("janus-admin")
            .author(crate_authors!())
            .about("Interact with Janus admin/monitoring APIs")
            .version(GIT_VERSION)
            .arg_required_else_help(true)
            .about("Interact with Janus admin/monitoring APIs https://janus.conf.meetecho.com/docs/admin.html")
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
        // config get|set|init|sync|show
        .subcommand(Command::new("config")
            .author(crate_authors!())
            .about(crate_description!())
            .version(GIT_VERSION)
            .arg_required_else_help(true)
            .about("Interact with PrintNanny device configuration and user settings")
            .subcommand(Command::new("get")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("Print PrintNanny config to console")
                .arg(Arg::new("key").required(false))
                .arg(Arg::new("format")
                    .short('f')
                    .long("format")
                    .takes_value(true)
                    .possible_values(ConfigFormat::possible_values())
                    .default_value("toml")
                    .help("Output format")
                )
            )
            .subcommand(Command::new("set")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("Sets PrintNanny config fragments from environment variables")
                .arg(Arg::new("key").required(true))
                .arg(Arg::new("value").required(true))
                .arg(Arg::new("format")
                    .short('f')
                    .long("format")
                    .takes_value(true)
                    .possible_values(ConfigFormat::possible_values())
                    .default_value("toml")
                    .help("Output format")
                )
            )
            .subcommand(Command::new("init")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("Initialize config from printnanny.zip")
                .arg(Arg::new("force")
                    .short('F')
                    .long("force")
                    .help("Overwrite any existing configuration")
                )
            )
            .subcommand(Command::new("show")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("Print PrintNanny config to console")
                .arg(Arg::new("format")
                    .short('f')
                    .long("format")
                    .takes_value(true)
                    .possible_values(ConfigFormat::possible_values())
                    .default_value("toml")
                    .help("Output format")
                )            
            )
            .subcommand(Command::new("sync")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("Synchronize device with PrintNanny Cloud")
            ))
        // nats-worker
        .subcommand(printnanny_nats::worker::Worker::clap_command())

        // .subcommand(Command::new("nats-publish"))

        // nats-publish 
        // .author(crate_authors!())
            // .about(crate_description!())
            // .version(GIT_VERSION)
            // .about("Interact with PrintNanny async events/commands API")
            // // .subcommand_required(true)
            // .subcommand(printnanny_nats::worker::Worker::clap_command())
            // .subcommand(printnanny_nats::events::EventPublisher::clap_command())
        
        // os <issue|motd>
        .subcommand(Command::new("os")
            .author(crate_authors!())
            .about(crate_description!())
            .version(GIT_VERSION)
            .subcommand_required(true)
            .subcommand(
                Command::new("issue")
                .about("Show contents of /etc/issue")
            )
            .subcommand(
                Command::new("motd")
                .about("Show message of the day")
            )
            .about("Interact with PrintNanny OS")
        );
    
    
    let app_m = app.get_matches();


    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'printnanny v v v' or 'printnanny vvv' vs 'printnanny v'
    let verbosity = app_m.occurrences_of("v");
    match verbosity {
        0 => {
            builder.filter_level(LevelFilter::Warn).init();
            gst::debug_set_default_threshold(gst::DebugLevel::Warning);
        }
        1 => {
            builder.filter_level(LevelFilter::Info).init();
            gst::debug_set_default_threshold(gst::DebugLevel::Info);
        }
        2 => {
            builder.filter_level(LevelFilter::Debug).init();
            gst::debug_set_default_threshold(gst::DebugLevel::Debug);
        }
        _ => {
            gst::debug_set_default_threshold(gst::DebugLevel::Trace);
            builder.filter_level(LevelFilter::Trace).init()
        }
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
        // Some(("event", sub_m)) => {
        //     match sub_m.subcommand() {
        //         Some(("worker", args)) => {
        //             let app = printnanny_nats::worker::Worker::new(args).await?;
        //             app.run().await?;
        //         }
        //         Some(("create", args)) => {

        //             let app = printnanny_nats::events::EventPublisher::new(args)?;
        //             app.run().await?;
        //         },
        //         _ => panic!("Expected worker|create subcommand")
        //     }
        // },
        Some(("cam", subm)) => {
            let app = cam::PrintNannyCam::new(&subm);
            app.run()?;
        },
        Some(("config", subm)) => {
            ConfigCommand::handle(subm).await?;
        },
        Some(("os", subm)) => {
            OsCommand::handle(subm)?;
        },
        Some(("janus-admin", sub_m)) => {
            let endpoint: JanusAdminEndpoint = sub_m.value_of_t("endpoint").unwrap_or_else(|e| e.exit());
            let res = janus_admin_api_call(
                endpoint,
            ).await?;
            println!("{}", res);

        },
        _ => {}
    };
    Ok(())
}
