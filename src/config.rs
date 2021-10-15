use std::path::{ PathBuf };
use std::{ env }; 
use std::fs::File;
use log::{ info };
use glob::glob;

use thiserror::Error;
use anyhow::{ anyhow, Context, Result };
use dialoguer::{ Input };
use serde::{ Serialize, Deserialize };
use config::{ConfigError, Config, File as ConfigFile, Environment};


use print_nanny_client::apis::auth_api::{ auth_email_create, auth_token_create };
use print_nanny_client::models::{ 
    CallbackTokenAuthRequest,
    DetailResponse,
    EmailAuthRequest,
    TokenResponse,
};
use print_nanny_client::models::{ 
    Appliance,
    User
};

#[derive(Error, Debug)]
pub enum PromptError {
    #[error("🔴 Please enter required field: {0}")]
    Required(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalConfig {
    
    #[serde(default)]
    pub api_base_path: String,

    #[serde(default, skip_serializing_if="Option::is_none")]
    pub api_token: Option<String>,
    #[serde(default)]
    pub config_path: PathBuf,

    #[serde(default)]
    pub email: Option<String>,

    #[serde(default, skip_serializing_if="Option::is_none")]
    pub appliance: Option<Appliance>,
    #[serde(default, skip_serializing_if="Option::is_none")]
    pub user: Option<User>
}

impl ::std::default::Default for LocalConfig {
    fn default() -> Self { Self { 
        api_base_path: "https://print-nanny.com".to_string(),
        api_token: None,
        config_path: PathBuf::from("."),
        appliance: None,
        email: None,
        user: None
    }}
}

#[derive(Debug, Clone)]
struct SetupPrompter {
    pub config: LocalConfig
}

impl SetupPrompter{
    fn new(self, 
        config_name: &str
    ) -> Result<SetupPrompter> {
        let config = LocalConfig::from(config_name)?;
        Ok(SetupPrompter { config })
    }


    // Basic flow goess
    // if <field> not exist -> prompt for config
    // if <field> exist, print config -> prompt to use Y/n -> prompt for config OR proceed
    pub async fn setup(mut self) -> Result<()>{
        if self.config.email.is_none() {
            self.config.email = Some(self.prompt_email());
        };
        if self.config.api_token.is_none() {
            LocalConfig::verify_2fa_send_email(&self.config).await?;
            let opt_token = self.prompt_token_input();
            let token_res = LocalConfig::verify_2fa_code(&self.config, opt_token).await?;
            self.config.api_token = Some(token_res.token);
        };
        if self.config.user.is_none(){
            let user = LocalConfig::get_user(&self.config).await?;
            self.config.user = Some(user);
            // self.config.write_settings("user.json")?;
        };
        if self.config.user.is_none(){
            let user = LocalConfig::get_user(&self.config).await?;
            self.config.user = Some(user);
            // self.config.write_settings("user.json")?;
        };
        // if self.config.appliance.is_none(){
        //     let appliance = LocalConfig::get_appliance(&self.config).await?;
        //     self.config.appliance = Some(appliance);
        // };
        LocalConfig::print_spacer();
        self.config.write_settings("settings.json");
        info!("💜 Saved config to {:?}", self.config.config_path);
        Ok(())
    }

    fn prompt_email(&self) -> String {
        LocalConfig::print_spacer();
        let prompt = "📨 Enter your email address";
        Input::new()
            .with_prompt(prompt)
            .interact_text()
            .unwrap()
    }
    fn prompt_token_input(&self) -> String {
        let email = self.config.email.unwrap();
        let prompt = format!("⚪ Enter the 6-digit code emailed to {}", email);
        let input : String = Input::new()
            .with_prompt(prompt)
            .interact_text()
            .unwrap();
        info!("Received input code {}", input);
        input
    }
}

impl LocalConfig {
    /// Serializes settings stored in ~/.printnanny/settings/*json

    async fn verify_2fa_send_email(&self) -> Result<DetailResponse> {
        // Sends an email containing an expiring one-time password (6 digits)
        match &self.email {
            Some(email) => {
                let req =  EmailAuthRequest{email: email.to_string()};
                let res = auth_email_create(&self.api_config(), req).await
                    .context(format!("🔴 Failed to send verification email to {:?}", self))?;
                info!("SUCCESS auth_email_create detail {:?}", serde_json::to_string(&res));
                Ok(res)
            }
            None => Err(anyhow!("LocalConfig.verify_2fa_send_email requires email to be set"))
        }

    }

    async fn verify_2fa_code(&self, token: String) -> Result<TokenResponse> {
        let req = CallbackTokenAuthRequest{mobile: None, token, email:Some(self.email.to_string())};
        let res = auth_token_create(&self.api_config(), req).await
            .context("🔴 Verification failed. Please try again or contact leigh@print-nanny.com for help.")?;
        info!("SUCCESS auth_verify_create detail {:?}", serde_json::to_string(&res));
        Ok(res)
    }
    pub fn from(config_name: &str) -> Result<Self, ConfigError> {
        let mut s = Config::default();
        // select Config::default from LocalConfig::default()
        
        s.set("config_path", config_name)?;

        // https://github.com/mehcode/config-rs/blob/master/examples/hierarchical-env/src/settings.rs
        // Start off by merging in the "default" configuration file

        // glob all files in base directory
        // Default to "settings" but allows for variants like:
        // RUN_MODE="sandbox" RUN_MODE="prod-account-A"
        let glob_pattern = format!("{}/*", config_name);

        // Glob all configuration files in base directory
        s
        .merge(glob(&glob_pattern)
                   .unwrap()
                   .map(|path| ConfigFile::from(path.unwrap()))
                   .collect::<Vec<_>>())
        .unwrap();

        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        s.merge(Environment::with_prefix("PRINTNANNY"))?;

        // You may also programmatically change settings
        // s.set("config_path", config_path)?;

        // Now that we're done, let's access our configuration

        // You can deserialize (and thus freeze) the entire configuration as
        s.try_into()

    }

    // pub fn reset() -> Self {
    //     let defaults = LocalConfig::new();
    //     defaults.save();
    //     print_reset();
    //     Ok(defaults)
    // }

    pub fn api_config(&self) -> print_nanny_client::apis::configuration::Configuration {
        if self.api_token.is_none(){
            print_nanny_client::apis::configuration::Configuration{
                base_path:self.api_base_path.to_string(), 
                ..Default::default()
            }
        } else {
            print_nanny_client::apis::configuration::Configuration{
                base_path:self.api_base_path.to_string(),
                bearer_access_token:self.api_token.clone(),
                ..Default::default()
            }
        }
    }
    pub fn print_reset(&self) {
        LocalConfig::print_spacer();
        info!("💜 Config was reset!");
        info!("💜 To ");      
        LocalConfig::print_spacer();
    }
    
    pub fn print_spacer() {
        let (w, _) = term_size::dimensions().unwrap_or((24,24));
        let spacer = (0..w/2).map(|_| "-").collect::<String>();
        info!("{}", spacer);
    }

    pub fn print_user(&self) {
        LocalConfig::print_spacer();
        info!("💜 Logged in as user:");
        info!("💜 {:#?}", self.user);        
        LocalConfig::print_spacer();
    }

    pub fn print(&self) {
        LocalConfig::print_spacer();
        info!("💜 Print Nanny config:");
        info!("💜 {:#?}", self);
        LocalConfig::print_spacer();
    }
    pub async fn get_appliance(&self) -> Result<Appliance>{
        let res = print_nanny_client::apis::users_api::users_me_retrieve(
            &self.api_config()
        ).await.context(format!("🔴 Failed to retreive user {:#?}", self.email))?;
        Ok(res)
    }

    pub async fn get_user(&self) -> Result<User>{
        let res = print_nanny_client::apis::users_api::users_me_retrieve(
            &self.api_config()
        ).await.context(format!("🔴 Failed to retreive user {:#?}", self.email))?;
        Ok(res)
    }
    
    pub fn write_settings(&self, filename: &str) -> Result<()>{
        let filepath = PathBuf::from(&self.config_path).join(filename);
        let file = &File::create(filepath)
            .context(format!("🔴 Failed to create file handle {:#?}",&self.config_path))?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }

}
