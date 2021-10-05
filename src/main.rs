use anyhow::{ Result };
use log::{ info };
use env_logger::Builder;
use log::LevelFilter;
use clap::{ Arg, App, SubCommand };
use printnanny::auth::{ auth };
use printnanny::config:: { LocalConfig };


async fn handle_auth(config: LocalConfig) -> Result<LocalConfig>{
    if config.api_token.is_none() {
        let updated_config = auth(config).await?;
        updated_config.print();
        Ok(updated_config)
    } else {
        config.print();
        Ok(config)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::new();
    let app_name = "printnanny";
    let app = App::new(app_name)
        .version("0.1.0")
        .author("Leigh Johnson <leigh@bitsy.ai>")
        .about("Official Print Nanny CLI https://print-nanny.com")
        .arg(Arg::with_name("api-url")
            .long("api-url")
            .help("Specify api_url")
            .value_name("API_URL")
            .takes_value(true))
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .help("Load custom config file")
            .value_name("FILE")
            .takes_value(true))
        .arg(Arg::with_name("v")
        .short("v")
        .multiple(true)
        .help("Sets the level of verbosity"))
        .subcommand(SubCommand::with_name("auth")
            .about("Connect your Print Nanny account"));
        
    let app_m = app.get_matches();

    let default_config_name = "default";
    let config_name = app_m.value_of("config").unwrap_or(default_config_name);
    info!("Using config file: {}", config_name);

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'printnanny -v -v -v' or 'printnanny -vvv' vs 'printnanny -v'
    let verbosity = app_m.occurrences_of("v");
    match verbosity {
        0 => builder.filter_level(LevelFilter::Warn).init(),
        1 => builder.filter_level(LevelFilter::Info).init(),
        2 => builder.filter_level(LevelFilter::Debug).init(),
        3 | _ => builder.filter_level(LevelFilter::Trace).init(),
    };
    
    let config = LocalConfig::load(app_name)?;

    match app_m.subcommand() {
        ("auth", Some(_sub_m)) => {
            handle_auth(config).await?;
        },
        _ => {}
    }
    Ok(())
}
