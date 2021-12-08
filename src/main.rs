use anyhow::{ Result };
use std::process::{Command, Stdio};
use env_logger::Builder;
use log::LevelFilter;
use clap::{ Arg, App, SubCommand };
// use printnanny::mqtt:: { MQTTWorker };
use printnanny::license:: { activate_license };
use printnanny::service::PrintNannyService;

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
        .subcommand(SubCommand::with_name("factsd")
        .about("Retrieve serialized JSON, intended for use by /etc/ansible/facts.d/")
            .subcommand(SubCommand::with_name("device")
            .about("GET /api/devices/:id"))
            .subcommand(SubCommand::with_name("license")
            .about("GET /api/licenses/:id")))
        // device

        // license
        .subcommand(SubCommand::with_name("license")
        .about("Interact with /api/licenses endpoints"))
        // update
        .subcommand(SubCommand::with_name("update")
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
        ("factsd", Some(sub_m)) => {
            match sub_m.subcommand() {
                ("device", Some(_sub_m)) => {
                    let service = PrintNannyService::new(&config)?;
                    let device_json = service.refresh_device_json().await?;
                    print!("{}", device_json);
                },
                ("license", Some(_sub_m)) => {
                    let service = PrintNannyService::new(&config)?;
                    let license_json = service.refresh_license_json().await?;
                    print!("{}", license_json);
                },
                _ => {}
            }
        },
        ("update", Some(_sub_m)) => {
            let mut cmd =
            Command::new("systemctl")
            .args(&["start", "printnanny-manual-update"])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();

            let status = cmd.wait();
            println!("Update excited with status {:?}", status);
        },
        _ => {}
    }

    // refresh local config after any command

    Ok(())
}
