use anyhow::{anyhow, Result};
use log::error;
use printnanny_services::config::PrintNannyConfig;
use std::fs;

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
    let config = PrintNannyConfig::new()?;
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

impl OsCommand {
    pub fn handle(sub_m: &clap::ArgMatches) -> Result<()> {
        match sub_m.subcommand() {
            Some(("issue", _args)) => handle_issue(),
            Some(("motd", _args)) => handle_motd(),
            _ => Err(anyhow!("Unhandled subcommand")),
        }
    }
}
