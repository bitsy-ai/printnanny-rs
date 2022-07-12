use log::error;
use printnanny_services::config::PrintNannyConfig;
use printnanny_services::error::ServiceError;
use std::fs;

pub struct OsCommand;

impl OsCommand {
    pub fn handle(_sub_m: &clap::ArgMatches) -> Result<(), ServiceError> {
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
}
