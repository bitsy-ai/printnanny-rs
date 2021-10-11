use std::path::{ PathBuf };
use std::{ env }; 
use std::fs::File;
use log::{ info };
use glob::glob;

use thiserror::Error;
use anyhow::{ Context, Result };
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
    pub config_name: String,

    #[serde(default)]
    pub email: String,

    #[serde(default, skip_serializing_if="Option::is_none")]
    pub appliance: Option<Appliance>,
    #[serde(default, skip_serializing_if="Option::is_none")]
    pub user: Option<User>
}

impl ::std::default::Default for LocalConfig {
    fn default() -> Self { Self { 
        api_base_path: "https://print-nanny.com".to_string(),
        api_token: None,
        config_name: "settings".to_string(),
        appliance: None,
        email: "".to_string(),
        user: None
    }}
}

impl LocalConfig {

    pub fn settings_base_string(config_name: &str) -> String {
        let base_path = LocalConfig::settings_base_path(config_name);
        base_path.into_os_string().into_string().unwrap()
    }

    pub fn settings_base_path(config_name: &str) -> PathBuf {
        let mode = env::var("RUN_MODE").unwrap_or_else(|_| config_name.to_string());
        let mode_dir = format!(".printnanny/{}", &mode);
        let path = dirs::home_dir().unwrap_or(PathBuf::from(".")).join(mode_dir);
        path
    }
    pub fn new(config_name: &str) -> Result<Self, ConfigError> {
        let mut s = Config::default();

        // https://github.com/mehcode/config-rs/blob/master/examples/hierarchical-env/src/settings.rs
        // Start off by merging in the "default" configuration file

        // glob all files in base directory
        // Default to "settings" but allows for variants like:
        // RUN_MODE="sandbox" RUN_MODE="prod-account-A"
        let base_path = LocalConfig::settings_base_string(config_name);
        let glob_pattern = format!("{}/*", &base_path);

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
        s.set("config_name", config_name)?;

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

    // pub fn api_config(&self) -> print_nanny_client::apis::configuration::Configuration {
    //     if self.api_token.is_none(){
    //         print_nanny_client::apis::configuration::Configuration{
    //             base_path:self.api_url.to_string(), 
    //             ..Default::default()
    //         }
    //     } else {
    //         print_nanny_client::apis::configuration::Configuration{
    //             base_path:self.api_url.to_string(),
    //             bearer_access_token:self.api_token.clone(),
    //             ..Default::default()
    //         }
    //     }
    // }
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

    // pub fn load(app_name: &str) -> Result<LocalConfig, confy::ConfyError> {
    //     confy::load(app_name)
    // }

    // pub fn save(&self) -> Result<(), confy::ConfyError> {
    //     confy::store(&self.app_name, self)
    // }

    // pub async fn get_user(&self) -> Result<User>{
    //     let api_config = LocalConfig::api_config(self);
    //     let res = print_nanny_client::apis::users_api::users_me_retrieve(
    //         &api_config
    //     ).await.context(format!("🔴 Failed to retreive user {:#?}", self.email))?;
    //     Ok(res)
    // }
    
    pub fn write_settings(&self, filename: &str) -> Result<()>{
        let path = LocalConfig::settings_base_path(&self.config_name);
        let target = path.join(filename).into_os_string().into_string().unwrap();
        let file = &File::create(&target)
        .context(format!("🔴 Failed to create file handle {:#?}", &target))?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }

    pub async fn prompt_2fa(mut self) -> Result<Self> {
        self.email = LocalConfig::prompt_email();
        self.verify_2fa_send_email().await?;
        let otp_token = self.prompt_token_input();
        let res: TokenResponse = self.verify_2fa_code(otp_token).await?;
        
        self.api_token = Some(res.token);
        self.write_settings("api.json")?;
        Ok(self)
    }

    async fn verify_2fa_send_email(&self) -> Result<DetailResponse> {
        let api_config = print_nanny_client::apis::configuration::Configuration{
            base_path: self.api_base_path.clone(), 
            ..Default::default()
        };

        // Sends an email containing an expiring one-time password (6 digits)
        let req =  EmailAuthRequest{email: self.email.clone()};
        let res = auth_email_create(&api_config, req).await
            .context(format!("🔴 Failed to send verification email to {}", self.email))?;
        info!("SUCCESS auth_email_create detail {:?}", serde_json::to_string(&res));
        Ok(res)
    }
    
    async fn verify_2fa_code(&self, token: String) -> Result<TokenResponse> {
        let api_config = print_nanny_client::apis::configuration::Configuration{
            base_path: self.api_base_path.clone(), 
            ..Default::default()
        };
        let req = CallbackTokenAuthRequest{mobile: None, token, email:Some(self.email.to_string())};
        let res = auth_token_create(&api_config, req).await
            .context("🔴 Verification failed. Please try again or contact leigh@print-nanny.com for help.")?;
        info!("SUCCESS auth_verify_create detail {:?}", serde_json::to_string(&res));
        Ok(res)
    }

    pub fn prompt_email() -> String {
        LocalConfig::print_spacer();
        let prompt = "⚪ Enter your email address";
        Input::new()
            .with_prompt(prompt)
            .interact_text()
            .unwrap()
    }

    pub fn prompt_token_input(&self) -> String {
        let prompt = format!("⚪ Enter the 6-digit code emailed to {}", self.email);
        let input : String = Input::new()
            .with_prompt(prompt)
            .interact_text()
            .unwrap();
        info!("Received input code {}", input);
        input
    }
}

