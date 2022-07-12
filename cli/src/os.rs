use anyhow::{anyhow, Result};
use log::error;
use printnanny_services::config::PrintNannyConfig;
use printnanny_services::error::ServiceError;
use std::fs;

pub struct OsCommand;

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

impl OsCommand {
    pub fn handle(sub_m: &clap::ArgMatches) -> Result<()> {
        match sub_m.subcommand() {
            Some(("issue", _args)) => handle_issue(),
            _ => Err(anyhow!("Unhandled subcommand")),
        }
    }
}
