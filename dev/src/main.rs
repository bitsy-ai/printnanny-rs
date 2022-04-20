#[macro_use]
extern crate clap;
use std::process::{Command, Stdio};

use anyhow::Result;
use clap::{App, AppSettings, Arg};
use env_logger::Builder;
use log::{info, LevelFilter};

use printnanny_dev::octoprint::{OctoPrintAction, OctoPrintCmd};
use printnanny_services::config::PrintNannyConfig;

#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::new();
    let app_name = "printnanny-dev";

    let app = App::new(app_name)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .author(crate_authors!())
        .about(crate_description!())
        .version(crate_version!())
        .arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .takes_value(true)
                .help("Path to Config.toml (see env/ for examples)"),
        )
        // janusadmin
        .subcommand(
            App::new("octoprint")
                .author(crate_authors!())
                .about(crate_description!())
                .version(crate_version!())
                .setting(AppSettings::ArgRequiredElseHelp)
                .about("Interact with OctoPrint installation")
                // model
                .arg(
                    Arg::new("action")
                        .possible_values(OctoPrintAction::possible_values())
                        .ignore_case(true)
                        .required_ifs(&[("action", "pip-install"), ("action", "pip-remove")]),
                )
                .arg(
                    Arg::new("package")
                        .short('p')
                        .long("package")
                        .takes_value(true),
                ),
        );
    let app_m = app.get_matches();

    let conf_file = app_m.value_of("config");

    let config: PrintNannyConfig = PrintNannyConfig::new(conf_file)?;

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
        Some(("octprint", sub_m)) => {
            let action: OctoPrintAction = sub_m.value_of_t("action").unwrap_or_else(|e| e.exit());
            let package = sub_m.value_of("package").map(|s| s.to_string());
            let cmd = OctoPrintCmd::new(action, config, package);
            let result = cmd.handle()?;
            println!("{:?}", result)
        }
        _ => {}
    }
    Ok(())
}
