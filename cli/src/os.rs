use anyhow::{anyhow, Result};
use clap::ArgMatches;
use log::error;
use std::fs;

use printnanny_services::metadata;
use printnanny_settings::printnanny::PrintNannySettings;
use printnanny_settings::SettingsFormat;
pub struct OsCommand;

const MTOD_HEADER: &str = r"

_____      _       _   _   _                         
|  __ \    (_)     | | | \ | |                        
| |__) | __ _ _ __ | |_|  \| | __ _ _ __  _ __  _   _ 
|  ___/ '__| | '_ \| __| . ` |/ _` | '_ \| '_ \| | | |
| |   | |  | | | | | |_| |\  | (_| | | | | | | | |_| |
|_|   |_|  |_|_| |_|\__|_| \_|\__,_|_| |_|_| |_|\__, |
                                                 __/ |
                                                |___/ 
";

fn handle_issue() -> Result<()> {
    let config = PrintNannySettings::new().await?;
    let result = fs::read_to_string(&config.paths.issue_txt);
    let output = match result {
        Ok(content) => content,
        Err(e) => {
            let msg = format!(
                "Error reading file={:?} error={:?}",
                &config.paths.issue_txt, e
            );
            error!(
                "Error reading file={:?} error={:?}",
                &config.paths.issue_txt, e
            );
            msg
        }
    };
    print!("{}", output);
    Ok(())
}

fn handle_motd() -> Result<()> {
    print!("{}", &MTOD_HEADER);
    handle_issue()
}

fn handle_system_info(args: &ArgMatches) -> Result<()> {
    let system_info = metadata::system_info()?;
    let format = args.value_of_t::<SettingsFormat>("format")?;
    let output = match format {
        SettingsFormat::Json => serde_json::to_string(&system_info)?,
        SettingsFormat::Toml => toml::ser::to_string(&system_info)?,
        SettingsFormat::Ini | SettingsFormat::Yaml => todo!(),
    };
    print!("{}", &output);
    Ok(())
}

impl OsCommand {
    pub fn handle(sub_m: &clap::ArgMatches) -> Result<()> {
        match sub_m.subcommand() {
            Some(("issue", _args)) => handle_issue(),
            Some(("motd", _args)) => handle_motd(),
            Some(("system-info", args)) => handle_system_info(args),

            _ => Err(anyhow!("Unhandled subcommand")),
        }
    }
}
