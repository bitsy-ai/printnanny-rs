use print_nanny_client::models::{ 
    Appliance,
    User
};

pub struct LocalConfig {
    #[serde(default)]
    pub api_url: String,
    #[serde(default)]
    pub api_token: Option<String>,
    #[serde(default)]
    pub app_name: String,
    #[serde(default)]
    pub config_name: Option<String>,

    #[serde(default)]
    pub appliance: Option<Appliance>,
    #[serde(default)]
    pub user: Option<User>

    fn print(self){
        println!("💜 Print Nanny config:");
        println!("{:#?}", self);
    }

    fn load_config(self -> Result<LocalConfig, confy::ConfyError> {
        return confy::load(self.app_name, self.config_name); // platform-specific default config path
    }
    
}

impl ::std::default::Default for LocalConfig {
    fn default() -> Self { Self { 
        api_url: "https://www.print-nanny.com".to_string(),
        api_token: None,
        app_name: "printnanny",
        appliance: None,
        config_name: None,
        user: None
    }}
}
