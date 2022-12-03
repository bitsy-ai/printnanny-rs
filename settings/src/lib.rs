pub mod cam;
pub mod error;
pub mod klipper;
pub mod mainsail;
pub mod moonraker;
pub mod octoprint;
pub mod printnanny;
pub mod vcs;

// re-export crates
pub use clap;
pub use figment;
pub use git2;
pub use printnanny_asyncapi_models;
pub use serde;
pub use serde_json;
pub use sys_info;
pub use toml;

use clap::{ArgEnum, PossibleValue};
use serde::{Deserialize, Serialize};

use printnanny_asyncapi_models::SettingsFormat as SettingsFormatPayload;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ArgEnum, Deserialize, Serialize)]
pub enum SettingsFormat {
    #[serde(rename = "ini")]
    Ini,
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "toml")]
    Toml,
    #[serde(rename = "yaml")]
    Yaml,
}

impl SettingsFormat {
    pub fn possible_values() -> impl Iterator<Item = PossibleValue<'static>> {
        SettingsFormat::value_variants()
            .iter()
            .filter_map(ArgEnum::to_possible_value)
    }
}

impl std::fmt::Display for SettingsFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

impl std::str::FromStr for SettingsFormat {
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

impl From<SettingsFormat> for SettingsFormatPayload {
    fn from(f: SettingsFormat) -> SettingsFormatPayload {
        match f {
            SettingsFormat::Ini => SettingsFormatPayload::Ini,
            SettingsFormat::Json => SettingsFormatPayload::Json,
            SettingsFormat::Toml => SettingsFormatPayload::Toml,
            SettingsFormat::Yaml => SettingsFormatPayload::Yaml,
        }
    }
}
