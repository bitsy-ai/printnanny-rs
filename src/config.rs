use serde::{ Serialize, Deserialize };

use print_nanny_client::models::{ 
    Appliance,
    User
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct LocalConfig {
    #[serde(default)]
    pub api_url: String,
    #[serde(default)]
    pub api_token: Option<String>,
    #[serde(default)]
    pub app_name: String,
    #[serde(default)]
    pub config_name: String,

    pub appliance: Option<Appliance>,
    pub user: Option<User>
}

impl LocalConfig {

    pub fn print_spacer() {
        let (w, _) = term_size::dimensions().unwrap_or((24,24));
        let spacer = (0..w).map(|_| "=").collect::<String>();
        println!("{}", spacer);
    }

    pub fn print(&self) {
        LocalConfig::print_spacer();
        println!("💜 Print Nanny config:");
        println!("💜 {:#?}", self);
        LocalConfig::print_spacer();
    }

    pub fn load(app_name: &str) -> Result<LocalConfig, confy::ConfyError> {
        confy::load(app_name)
    }
}

impl ::std::default::Default for LocalConfig {
    fn default() -> Self { Self { 
        api_url: "https://www.print-nanny.com".to_string(),
        api_token: None,
        app_name: "printnanny".to_string(),
        appliance: None,
        config_name: "default".to_string(),
        user: None
    }}
}
