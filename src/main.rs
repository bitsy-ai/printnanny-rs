use anyhow::{ Result };
use std::process::{Command, Stdio};
use env_logger::Builder;
use log::LevelFilter;
use clap::{ Arg, App, SubCommand, value_t };
// use printnanny::mqtt:: { MQTTWorker };
use printnanny::license:: { activate_license };
use printnanny::service::{ printnanny_api_call, ApiModel, ApiAction };


#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::new();
    let app_name = "printnanny";
    
    let app = App::new(app_name)
        .version("0.5.1")
        .author("Leigh Johnson <leigh@bitsy.ai>")
        .about("Official Print Nanny CLI https://print-nanny.com")
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
        // janus-admin
        .subcommand(SubCommand::with_name("janus-admin")
            .about("Interact with Janus admin/monitoring APIs https://janus.conf.meetecho.com/docs/auth.html#token")
            .arg(Arg::with_name("host")
            .long("host")
            .short("h")
            .takes_value(true)
            .default_value("localhost:8088"))
            // add-token
            .subcommand(SubCommand::with_name("add-token")
                .about("Add memory-stored token")
                .arg(Arg::with_name("admin_secret")
                    .long("admin-secret")
                    .takes_value(true)
                    .required(true)
                )
                .arg(Arg::with_name("token")
                    .long("token")
                    .takes_value(true)
                    .required(true)
                )
                .arg(Arg::with_name("plugins")
                    .long("plugins")
                    .takes_value(true)
                    .required(true)
                    .use_delimiter(true)
                    .help("Comma-separated list of plugins used to scope token access. E.g janus.plugin.streaming,janus.plugin.videoroom")
                    .default_value("janus.plugin.echotest,janus.plugin.streaming")
                )
            )
            // list tokens
            .subcommand(SubCommand::with_name("list-tokens")
                .about("List tokens stored in memory")
                .arg(Arg::with_name("admin_secret")
                .long("admin-secret")
                .takes_value(true)
                .required(true)
                ))
            // remove token
            .subcommand(SubCommand::with_name("remove-token")
                .about("Remove stored in memory without restarting Janus service")
                .arg(Arg::with_name("admin_secret")
                .long("admin-secret")
                .takes_value(true)
                .required(true)
                ))
            )
            // ping & info
            .subcommand(SubCommand::with_name("info"))
            .subcommand(SubCommand::with_name("ping"))

            .subcommand(SubCommand::with_name("test-stun"))
        // api endpoints (used by ansible facts.d)
        .subcommand(SubCommand::with_name("api")
            .about("Retrieve Print Nanny REST API JSON responses")
            .arg(Arg::with_name("action")
                .long("action")
                .short("a")
                .takes_value(true)
                // possible values is case-sensitive
                // https://github.com/clap-rs/clap/issues/891#issuecomment-285545880
                .possible_values(&ApiAction::variants())
                .case_insensitive(true)
                //.possible_value("create")
                //.possible_value("update")
                //.possible_value("delete")
            )
            .arg(Arg::with_name("save")
                .long("save")
                .takes_value(false)
                .required(false)
                .help("Cache API response to /opt/printnanny/data (requires filesystem write permission)"))
            // model
            .arg(Arg::with_name("model")
                .long("model")
                .short("m")
                .takes_value(true)
                // possible values is case-sensitive
                // https://github.com/clap-rs/clap/issues/891#issuecomment-285545880
                .possible_values(&ApiModel::variants())
                .case_insensitive(true)
            ))
        // run system updates
        .subcommand(SubCommand::with_name("system-update")
        .about("Update Print Nanny software"))
        // mqtt <subscribe|publish>
        .subcommand(SubCommand::with_name("mqtt")
            .about("Publish or subscribe to MQTT messages")
        );  
    let app_m = app.get_matches();

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'printnanny -v -v -v' or 'printnanny -vvv' vs 'printnanny -v'
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
        ("system-update", Some(_sub_m)) => {
            let mut cmd =
            Command::new("systemctl")
            .args(&["start", "printnanny-updater"])
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
