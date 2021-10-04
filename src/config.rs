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
    pub config_name: Option<String>,

    pub appliance: Option<Appliance>,
    pub user: Option<User>
}

impl LocalConfig {
        fn print(self){
        println!("💜 Print Nanny config:");
        println!("{:#?}", self);
    }

    // fn load_config(self -> Result<LocalConfig, confy::ConfyError> {
    //     match self.config_name {
    //         Some(config_name) => {
    //             confy::load(&config_name)
    //         }
    //         _ => confy::load();
    //     }
    // }
}

impl ::std::default::Default for LocalConfig {
    fn default() -> Self { Self { 
        api_url: "https://www.print-nanny.com".to_string(),
        api_token: None,
        app_name: "printnanny".to_string(),
        appliance: None,
        config_name: None,
        user: None
    }}
}
