#[macro_use] extern crate clap;

use anyhow::{ Result };
use env_logger::Builder;
use log::{ LevelFilter, error};
use clap::{ 
    Arg, Command
};
use git_version::git_version;

use printnanny_services::printnanny_api::ApiService;
use printnanny_services::setup::printnanny_os_init;
use printnanny_settings::{SettingsFormat};
use printnanny_services::janus::{ JanusAdminEndpoint, janus_admin_api_call };
use printnanny_settings::printnanny::PrintNannySettings;

use printnanny_cli::cam::CameraCommand;
use printnanny_cli::settings::{SettingsCommand};
use printnanny_cli::cloud_data::CloudDataCommand;
use printnanny_cli::os::{OsCommand};

use printnanny_gst_pipelines::factory::H264_RECORDING_PIPELINE;

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
            ))
            .subcommand(Command::new("start-pipelines")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("Start all PrintNanny Vision pipelines")      
                .arg(
                    Arg::new("http-address")
                    .takes_value(true)
                    .long("http-address")
                    .default_value("127.0.0.1")
                    .help("Attach to the server through a given address"))
                .arg(
                        Arg::new("http-port")
                        .takes_value(true)
                        .long("http-port")
                        .default_value("5001")
                        .help("Attach to the server through a given port")
            ))
            .subcommand(Command::new("stop-pipelines")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("Stop all PrintNanny Vision pipelines")      
                .arg(
                    Arg::new("http-address")
                    .takes_value(true)
                    .long("http-address")
                    .default_value("127.0.0.1")
                    .help("Attach to the server through a given address"))
                .arg(
                        Arg::new("http-port")
                        .takes_value(true)
                        .long("http-port")
                        .default_value("5001")
                        .help("Attach to the server through a given port")
            ))
            .subcommand(Command::new("list-pipelines")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("List all PrintNanny Vision pipelines")      
                .arg(
                    Arg::new("http-address")
                    .takes_value(true)
                    .long("http-address")
                    .default_value("127.0.0.1")
                    .help("Attach to the server through a given address"))
                .arg(
                        Arg::new("http-port")
                        .takes_value(true)
                        .long("http-port")
                        .default_value("5001")
                        .help("Attach to the server through a given port")
            ))
            .subcommand(Command::new("start-multifilesink-listener")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("Sync local video recording fragments to PrintNanny Cloud")      
                .arg(
                    Arg::new("http-address")
                    .takes_value(true)
                    .long("http-address")
                    .default_value("127.0.0.1")
                    .help("Attach to the gstd server through a given address"))
                .arg(
                        Arg::new("http-port")
                        .takes_value(true)
                        .long("http-port")
                        .default_value("5001")
                        .help("Attach to the gstd server through a given port"))
                .arg(
                    Arg::new("pipeline")
                    .takes_value(true)
                    .long("pipeline")
                    .default_value(H264_RECORDING_PIPELINE)
                    .help("Name of pipeline to watch")
                )
            )
        )
        .subcommand(Command::new("crash-report")
            .author(crate_authors!())
            .about("Submit a crash report via PrintNanny Cloud API")
            .version(GIT_VERSION)
            .arg(Arg::new("id")
                .takes_value(true)
                .long("id")
                .short('i')
                .help("Provide an id to attach system logs to a specific report")
            ) 
        )

        .subcommand(Command::new("init")
            .author(crate_authors!())
            .about("Initialize PrintNanny OS")
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
                .about("Print PrintNanny Cloud models to console")    
                
            )
            .subcommand(Command::new("sync-models")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("Sync PrintNanny Cloud models (Pi, SystemInfo, etc")          
            )
            .subcommand(Command::new("sync-videos")
                .author(crate_authors!())
                .about(crate_description!())
                .version(GIT_VERSION)
                .about("Sync PrintNanny Cloud models (Pi, SystemInfo, etc")          
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
        // os <issue|motd>
        .subcommand(Command::new("os")
            .author(crate_authors!())
            .about("Interact with PrintNanny OS")
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
            .subcommand(
                Command::new("shutdown")
                .about("Cleanup tasks that run before shutdown/restart/halt (final.target)")
            )
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
            CameraCommand::handle(sub_m).await?;
        },
        Some(("crash-report", sub_m)) => {
            let id = sub_m.value_of("id");

            let settings = match PrintNannySettings::new().await {
                Ok(settings) => settings,
                Err(e) => {
                    error!("Failed to initialize PrintNannySettings with error={}. Falling back to PrintNannySettings::default()", e);
                    PrintNannySettings::default()
                }
            };

            let api_service = ApiService::from(&settings);
            let crash_report_paths = settings.paths.crash_report_paths();

            let report = match id {
                Some(id) => api_service.crash_report_update(id, crash_report_paths).await,
                None => api_service.crash_report_create(None, None, None, None, None, None, settings.paths.crash_report_paths()).await
            }?;
            let report_json = serde_json::to_string_pretty(&report)?;
            println!("Submitted crash report:");
            println!("{}", report_json);
        },
        Some(("init", _sub_m)) => {
            printnanny_os_init().await?;
        }

        Some(("settings", subm)) => {
            SettingsCommand::handle(subm).await?;
        },
        Some(("cloud", subm)) => {
            CloudDataCommand::handle(subm).await?;
        },

        Some(("os", subm)) => {
            OsCommand::handle(subm).await?;
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
