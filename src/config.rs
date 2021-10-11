use log::{ info };
use thiserror::Error;
use anyhow::{ Context, Result };
use dialoguer::{ Input };
use serde::{ Serialize, Deserialize };
use config::{ConfigError, Config, File, Environment};


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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct LocalConfig {
    #[serde(default)]
    pub api_url: String,
    #[serde(default)]
    pub api_token: Option<String>,
    #[serde(default)]
    pub app_name: String,
    #[serde(default)]
    pub email: String,

    pub appliance: Option<Appliance>,
    pub user: Option<User>
}

impl ::std::default::Default for LocalConfig {
    fn default() -> Self { Self { 
        api_url: "https://www.print-nanny.com".to_string(),
        api_token: None,
        app_name: "printnanny".to_string(),
        appliance: None,
        email: "".to_string(),
        user: None
    }}
}

impl LocalConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = LocalConfig::default();

        // https://github.com/mehcode/config-rs/blob/master/examples/hierarchical-env/src/settings.rs
        // Start off by merging in the "default" configuration file
        s.merge(File::with_name(dirs::home_dir().join('.printnanny/settings/default')))?;

        // Add in the current environment file
        // Default to 'development' env
        // Note that this file is _optional_
        let env = env::var("RUN_MODE").unwrap_or_else(|_| "sandbox".into());
        s.merge(File::with_name(&format!("config/{}", env)).required(false))?;

        // Add in a local configuration file
        // This file shouldn't be checked in to git
        s.merge(File::with_name("config/local").required(false))?;

        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        s.merge(Environment::with_prefix("PRINTNANNY"))?;

        // Add in data models
        s.merge(File::with_name(dirs::home_dir().join('.printnanny/data/appliance')))?;
        s.merge(File::with_name(dirs::home_dir().join('.printnanny/data/user')))?;

        // You may also programmatically change settings
        // s.set("database.url", "postgres://")?;

        // Now that we're done, let's access our configuration
        LocalConfig::print(&s);
        // You can deserialize (and thus freeze) the entire configuration as
        s.try_into()
    }

    pub fn reset() -> Self {
        let defaults = LocalConfig::new();
        defaults.save();
        print_reset();
        Ok(defaults)
    }

    pub fn api_config(&self) -> print_nanny_client::apis::configuration::Configuration {
        if self.api_token.is_none(){
            print_nanny_client::apis::configuration::Configuration{
                base_path:self.api_url.to_string(), 
                ..Default::default()
            }
        } else {
            print_nanny_client::apis::configuration::Configuration{
                base_path:self.api_url.to_string(),
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

    pub fn load(app_name: &str) -> Result<LocalConfig, confy::ConfyError> {
        confy::load(app_name)
    }

    pub fn save(&self) -> Result<(), confy::ConfyError> {
        confy::store(&self.app_name, self)
    }

    pub async fn get_user(&self) -> Result<User>{
        let api_config = LocalConfig::api_config(self);
        let res = print_nanny_client::apis::users_api::users_me_retrieve(
            &api_config
        ).await.context(format!("🔴 Failed to retreive user {:#?}", self.email))?;
        Ok(res)
    }

    pub async fn auth(mut self) -> Result<LocalConfig> {
        self.email = LocalConfig::prompt_email();
        LocalConfig::verify_2fa_send_email(&self).await?;
        let otp_token = LocalConfig::prompt_token_input(&self);
        let res: TokenResponse = LocalConfig::verify_2fa_code(&self, otp_token).await?;
        self.api_token = Some(res.token);
        self.user = Some(LocalConfig::get_user(&self).await?);
        LocalConfig::save(&self)?;
        Ok(self)
    }

    async fn verify_2fa_send_email(&self) -> Result<DetailResponse> {
        let api_config = LocalConfig::api_config(self);
        // Sends an email containing an expiring one-time password (6 digits)
        let req =  EmailAuthRequest{email: self.email.clone()};
        let res = auth_email_create(&api_config, req).await
            .context(format!("🔴 Failed to send verification email to {}", self.email))?;
        info!("SUCCESS auth_email_create detail {:?}", serde_json::to_string(&res));
        Ok(res)
    }
    
    async fn verify_2fa_code(&self, token: String) -> Result<TokenResponse> {
        let api_config = LocalConfig::api_config(self);
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

