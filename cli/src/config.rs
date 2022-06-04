use clap::ArgEnum;
use printnanny_services::config::{PrintNannyConfig, PrintNannyConfigError};

#[derive(Copy, Eq, PartialEq, Debug, Clone, clap::ArgEnum)]
pub enum ConfigAction {
    Show,
    Init,
}

impl ConfigAction {
    pub fn handle(sub_m: &clap::ArgMatches) -> Result<(), PrintNannyConfigError> {
        let config: PrintNannyConfig = PrintNannyConfig::new()?;
        match sub_m.subcommand() {
            Some(("init", init_m)) => {
                let output = init_m.value_of("output").unwrap();
                let config = PrintNannyConfig::new()?;
                config.try_init(output.into())?;
                Ok(())
            }
            Some(("show", _)) => {
                println!("{}", toml::ser::to_string_pretty(&config)?);
                Ok(())
            }
            _ => panic!("Expected init|subscribe subcommand"),
        }
    }
    pub fn possible_values() -> impl Iterator<Item = clap::PossibleValue<'static>> {
        ConfigAction::value_variants()
            .iter()
            .filter_map(clap::ArgEnum::to_possible_value)
    }
}

impl std::str::FromStr for ConfigAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid variant: {}", s))
    }
}
