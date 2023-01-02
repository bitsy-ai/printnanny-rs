#[macro_use] extern crate clap;

use anyhow::{ Result };
use env_logger::Builder;
use log::{ LevelFilter};
use clap::{ 
    Arg, Command
};
use git_version::git_version;


use printnanny_cli::cam::CameraCommand;
use printnanny_nats::cloud_publisher::DEFAULT_NATS_CLOUD_PUBLISHER_APP_NAME;
use printnanny_nats::subscriber::{ DEFAULT_NATS_EDGE_APP_NAME, NatsSubscriber};
use printnanny_nats::message_v2::{NatsReply, NatsRequest};
use printnanny_nats::cloud_worker::DEFAULT_NATS_CLOUD_APP_NAME;
use printnanny_settings::{SettingsFormat};
use printnanny_services::janus::{ JanusAdminEndpoint, janus_admin_api_call };
use printnanny_cli::settings::{SettingsCommand};
use printnanny_cli::cloud_data::CloudDataCommand;
use printnanny_cli::os::{OsCommand};

const GIT_VERSION: &str = git_version!();

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
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
        .subcommand(Command::new("cam")
            .author(crate_authors!())
            .about("Interact with PrintNanny camera/device APIs")
            .version(GIT_VERSION)
            .subcommand_required(true)
            .subcommand(Command::new("list")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("List devices/cameras compatible with PrintNanny Vision")      
                .arg(Arg::new("format")
                .short('f')
                .long("format")
                .takes_value(true)
                .possible_values(SettingsFormat::possible_values())
                .default_value("json")
                .help("Output format")
            )     
        ))
        .subcommand(Command::new("crash-report")
            .author(crate_authors!())
            .about("Submit a crash report via PrintNanny Cloud API")
            .version(GIT_VERSION) 
        )

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
        
        // cloud show|sync
        .subcommand(Command::new("cloud")
            .author(crate_authors!())
            .about(crate_description!())
            .version(GIT_VERSION)
            .arg_required_else_help(true)
            .about("Interact with PrintNanny device and user settings")
            .subcommand(Command::new("show")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("Print PrintNannyCloudData to console")
                .arg(Arg::new("format")
                    .short('f')
                    .long("format")
                    .takes_value(true)
                    .possible_values(SettingsFormat::possible_values())
                    .default_value("json")
                    .help("Output format")
                )            
            )
            .subcommand(Command::new("sync")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("Print PrintNanny config to console")
                .arg(Arg::new("format")
                    .short('f')
                    .long("format")
                    .takes_value(true)
                    .possible_values(SettingsFormat::possible_values())
                    .default_value("json")
                    .help("Output format")
                )            
            )
        )
        

        // settings get|set|show
        .subcommand(Command::new("settings")
            .author(crate_authors!())
            .about(crate_description!())
            .version(GIT_VERSION)
            .arg_required_else_help(true)
            .about("Interact with PrintNanny device and user settings")
            .subcommand(Command::new("clone")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("Git clone PrintNanny Settings repo (default settings files for PrintNanny, OctoPrint, Moonraker, Klipper)")
                .arg(Arg::new("dir")
                    .short('d')
                    .long("dir")
                    .takes_value(true)
                    .help("Directory to clone repo to")
                )            
            )
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
                    .possible_values(SettingsFormat::possible_values())
                    .default_value("json")
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
                    .possible_values(SettingsFormat::possible_values())
                    .default_value("json")
                    .help("Output format")
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
                    .possible_values(SettingsFormat::possible_values())
                    .default_value("json")
                    .help("Output format")
                )            
            ))

        // nats-edge-worker
        .subcommand(NatsSubscriber::<NatsRequest, NatsReply>::clap_command(Some(DEFAULT_NATS_EDGE_APP_NAME.to_string())))
        // TODO
        // .subcommand(printnanny_nats::subscriber::NatsSubscriber::<NatsRequest, NatsReply>::clap_command(None))
        // nats-cloud-worker
        .subcommand(printnanny_nats::cloud_worker::NatsCloudWorker::clap_command(Some(DEFAULT_NATS_CLOUD_APP_NAME.to_string())))
        // nats-cloud-publisher
        .subcommand(printnanny_nats::cloud_publisher::CloudEventPublisher::clap_command(Some(DEFAULT_NATS_CLOUD_PUBLISHER_APP_NAME.to_string())))
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
            .subcommand(
                Command::new("system-info")
                .about("Print SystemInfo")
                .arg(Arg::new("format")
                    .short('f')
                    .long("format")
                    .takes_value(true)
                    .possible_values(SettingsFormat::possible_values())
                    .default_value("json")
                    .help("Output format")
                )
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
        }
        1 => {
            builder.filter_level(LevelFilter::Info).init();
        }
        2 => {
            builder.filter_level(LevelFilter::Debug).init();
        }
        _ => {
            builder.filter_level(LevelFilter::Trace).init()
        }
    };

    match app_m.subcommand() {
        Some(("cam", sub_m)) => {
            CameraCommand::handle(sub_m)?;
        },
        Some(("nats-publisher", sub_m)) => {
            let app = printnanny_nats::cloud_publisher::CloudEventPublisher::new(sub_m)?;
            app.run().await?;
        },

        Some((DEFAULT_NATS_CLOUD_APP_NAME, sub_m)) => {
            let app = printnanny_nats::cloud_worker::NatsCloudWorker::new(sub_m).await?;
            app.run().await?;
        },

        Some((DEFAULT_NATS_EDGE_APP_NAME, sub_m)) => {
            let worker = NatsSubscriber::<NatsRequest, NatsReply>::new(sub_m);
            worker.run().await?;
        },

        Some(("settings", subm)) => {
            SettingsCommand::handle(subm).await?;
        },
        Some(("cloud", subm)) => {
            CloudDataCommand::handle(subm).await?;
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
