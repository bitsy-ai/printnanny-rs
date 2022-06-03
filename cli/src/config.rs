use clap::ArgEnum;

#[derive(Copy, Eq, PartialEq, Debug, Clone, clap::ArgEnum)]
pub enum ConfigAction {
    Show,
    Init,
}

impl ConfigAction {
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
