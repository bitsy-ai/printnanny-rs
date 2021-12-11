use std::convert::TryInto;
use std::process::{ Command, Stdio };

use anyhow::{ Result };
use env_logger::Builder;
use log::LevelFilter;
use clap::{ 
    Arg, App, AppSettings, SubCommand, 
    value_t, crate_version, crate_authors, crate_description
};

// use printnanny::mqtt:: { MQTTWorker };
use printnanny::janus::{ JanusAdminEndpoint, janus_admin_api_call };
use printnanny::license:: { activate_license };
use printnanny::service::{ printnanny_api_call, ApiModel, ApiAction };


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
        .arg(Arg::with_name("config")
        .short("c")
        .long("config")
        .takes_value(true)
        .help("Path to Print Nanny installation")
        .default_value("/opt/printnanny"))
        // activate
        .subcommand(SubCommand::with_name("activate")
            .about("Activate license and send device info to Print Nanny API"))
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
                .hide_default_value(true)
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
                .hide_default_value(true)
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
        .subcommand(SubCommand::with_name("api")
            .about("Retrieve Print Nanny REST API JSON responses")
            .arg(Arg::with_name("action")
                .possible_values(&ApiAction::variants())
                .case_insensitive(true)
            )
            // model
            .arg(Arg::with_name("model")
                .possible_values(&ApiModel::variants())
                .case_insensitive(true)
            )
            .arg(Arg::with_name("save")
                .long("save")
                .takes_value(false)
                .required(false)
                .help("Cache API response to /opt/printnanny/data (requires filesystem write permission)")))
        // run system updates
        .subcommand(SubCommand::with_name("system-update")
        .about("Update Print Nanny software"))
        // mqtt <subscribe|publish>
        .subcommand(SubCommand::with_name("mqtt")
            .about("Publish or subscribe to MQTT messages")
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

    let config = app_m.value_of("config").unwrap();
    
    match app_m.subcommand() {
        // ("mqtt", Some(_sub_m)) => {
        //     let worker = MQTTWorker::new().await?;
        //     // worker.run().await?;
        //     worker.run().await?;
        // },
        ("activate", Some(_sub_m)) => {
            activate_license(&config).await?;
        },
        ("api", Some(sub_m)) => {
            let action = value_t!(sub_m, "action", ApiAction).unwrap_or_else(|e| e.exit());
            let model = value_t!(sub_m, "model", ApiModel).unwrap_or_else(|e| e.exit());
            let save = value_t!(sub_m, "save", bool).unwrap_or_default();
            
            let jsonstr = printnanny_api_call(&config, &save, &action, &model).await?;
            print!("{}", jsonstr)

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

    // refresh local config after any command

    Ok(())
}
