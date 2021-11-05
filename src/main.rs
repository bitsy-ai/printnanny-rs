use anyhow::{ Result };
use env_logger::Builder;
use log::LevelFilter;
use clap::{ Arg, App, SubCommand };
use printnanny::config:: { SetupPrompter };
use printnanny::mqtt:: { MQTTWorker };
use printnanny::config:: { DeviceInfo };
use printnanny::http:: { handle_probe };
use warp::Filter;

#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::new();
    let app_name = "printnanny";
    
    let app = App::new(app_name)
        .version("0.1.7")
        .author("Leigh Johnson <leigh@bitsy.ai>")
        .about("Official Print Nanny CLI https://print-nanny.com")
        .arg(Arg::with_name("v")
        .short("v")
        .multiple(true)
        .help("Sets the level of verbosity"))
        .subcommand(SubCommand::with_name("ansible-extra-vars")
            .about("Output device config as Ansible Facts"))
        .subcommand(SubCommand::with_name("http")
            .about("Serves HTTP requests. Used for device registration and status checks.")
        )
        .subcommand(SubCommand::with_name("setup")
            .about("Connect your Print Nanny account"))
        .subcommand(SubCommand::with_name("reset")
            .about("Reset your Print Nanny setup"))
        .subcommand(SubCommand::with_name("update")
            .about("Update Print Nanny system"))  
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
    
    match app_m.subcommand() {
        ("ansible-extra-vars", Some(_sub_m)) => {
            let mut config = DeviceInfo::new()?;
            config = config.refresh().await?;
            let release = config.release.unwrap();
            println!("{:?}", serde_json::to_string(&release.ansible_extra_vars)?)
        },
        ("mqtt", Some(_sub_m)) => {
            let worker = MQTTWorker::new().await?;
            // worker.run().await?;
            worker.run().await?;
        },
        ("setup", Some(_sub_m)) => {
            let prompter = SetupPrompter::new()?;
            prompter.setup().await?;
            let config = DeviceInfo::new()?;
            config.refresh().await?;
            
        },
        ("http", Some(sub_m)) => {
            let routes = warp::path!("config")
                .map(handle_probe);
            let addr = std::net::Ipv4Addr::new(0,0,0,0);
            warp::serve(routes).run((addr, 8331)).await;
        },
        ("reset", Some(_sub_m)) => {
            let mut prompter = SetupPrompter::new()?;
            prompter = prompter.reset()?;
            prompter.setup().await?;
        },
        ("update", Some(_sub_m)) => {
            unimplemented!();
        },

        _ => {}
    }

    // refresh local config after any command

    Ok(())
}
