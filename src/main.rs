use std::path::{ PathBuf };
use anyhow::{ Result };
use env_logger::Builder;
use log::LevelFilter;
use clap::{ Arg, App, SubCommand };
use printnanny::config:: { LocalConfig, SetupPrompter };

// resets config back to default values
// async fn handle_reset(config_name: &str) -> Result<LocalConfig>{
//     // let config = LocalConfig::load(app_name)?;

//     let defaults = LocalConfig::new();
//     defaults.save();
//     Ok(defaults)
// }

// #[test]
// fn test_print_help() -> Result<()>{
//     let mut cmd = Command::cargo_bin("printnanny")?;
//     cmd.args(&["--help"]);

//     cmd.assert()
//         .success()
//         .stdout(predicate::str::contains("Official Print Nanny CLI https://print-nanny.com"));
//     Ok(())
// }

#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::new();
    let app_name = "printnanny";
    
    let default_path = dirs::home_dir()
    .unwrap_or_else(|| PathBuf::from("."))
    .join(".printnanny/settings");
    let default_config = default_path.into_os_string().into_string().unwrap();

    let app = App::new(app_name)
        .version("0.1.0")
        .author("Leigh Johnson <leigh@bitsy.ai>")
        .about("Official Print Nanny CLI https://print-nanny.com")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .help("Load custom config file")
            .value_name("FILE")
            .takes_value(true)
            .default_value(&default_config)
        )
        .arg(Arg::with_name("v")
        .short("v")
        .multiple(true)
        .help("Sets the level of verbosity"))
        .subcommand(SubCommand::with_name("setup")
            .about("Connect your Print Nanny account"))
        .subcommand(SubCommand::with_name("reset")
        .about("Reset your Print Nanny setup"))
        .subcommand(SubCommand::with_name("update")
        .about("Update Print Nanny system"));    
    let app_m = app.get_matches();

    let config_file = app_m.value_of("config").unwrap_or(&default_config);
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
        ("setup", Some(_sub_m)) => {
            let prompter = SetupPrompter::new(config_file);
            prompter.setup().await?;
        },
        // ("reset", Some(_sub_m)) => {
        //     handle_reset(config_name).await?;
        // },
        ("update", Some(_sub_m)) => {
            unimplemented!();
        },
        _ => {}
    }
    Ok(())
}
