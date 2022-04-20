use clap::ArgEnum;
use std::fmt;

#[derive(Copy, Eq, PartialEq, Debug, Clone, clap::ArgEnum)]
pub enum OctoPrintAction {
    PipInstall,
    PipUninstall,
}

impl OctoPrintAction {
    pub fn possible_values() -> impl Iterator<Item = clap::PossibleValue<'static>> {
        OctoPrintAction::value_variants()
            .iter()
            .filter_map(clap::ArgEnum::to_possible_value)
    }
}

impl std::str::FromStr for OctoPrintAction {
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

impl fmt::Display for OctoPrintAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OctoPrintAction::PipInstall => write!(f, "pipinstall"),
            OctoPrintAction::PipUninstall => write!(f, "pipuninstall"),
        }
    }
}
