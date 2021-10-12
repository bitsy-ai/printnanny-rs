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
    pub config_path: PathBuf,

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
        config_path: PathBuf::from("."),
        appliance: None,
        email: "".to_string(),
        user: None
    }}
}

#[cfg(test)]
use mockall::{automock, mock, predicate::*};
#[cfg_attr(test, automock)]
pub trait ConfigPrompter {
    fn prompt_email() -> String;
    fn prompt_token_input(email: &str) -> String;
}
struct SetupPrompt {}

#[cfg(test)]
use mockall::{automock, mock, predicate::*};
#[cfg_attr(test, automock)]
impl ConfigPrompter for SetupPrompt {
    fn prompt_email() -> String {
        LocalConfig::print_spacer();
        let prompt = "⚪ Enter your email address";
        Input::new()
            .with_prompt(prompt)
            .interact_text()
            .unwrap()
    }
    fn prompt_token_input(email: &str) -> String {
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

    pub fn new(config_path: &PathBuf, config_name: &str) -> Result<Self, ConfigError> {
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

    pub async fn prompt_2fa(mut self) -> Result<Self> {
        // let prompt = SetupPrompt {};
        self.email = SetupPrompt::prompt_email();
        self.verify_2fa_send_email().await?;
        let otp_token = prompt.prompt_token_input(&self.email);
        let res: TokenResponse = self.verify_2fa_code(otp_token).await?;
        
        self.api_token = Some(res.token);
        self.write_settings("api.json")?;
        Ok(self)
    }

    async fn verify_2fa_send_email(&self) -> Result<DetailResponse> {
        // Sends an email containing an expiring one-time password (6 digits)
        let req =  EmailAuthRequest{email: self.email.clone()};
        let res = auth_email_create(&self.api_config(), req).await
            .context(format!("🔴 Failed to send verification email to {:?} {:?}", self, self.email))?;
        info!("SUCCESS auth_email_create detail {:?}", serde_json::to_string(&res));
        Ok(res)
    }
    
    async fn verify_2fa_code(&self, token: String) -> Result<TokenResponse> {
        let req = CallbackTokenAuthRequest{mobile: None, token, email:Some(self.email.to_string())};
        let res = auth_token_create(&self.api_config(), req).await
            .context("🔴 Verification failed. Please try again or contact leigh@print-nanny.com for help.")?;
        info!("SUCCESS auth_verify_create detail {:?}", serde_json::to_string(&res));
        Ok(res)
    }

}

// Basic flow goess
// if <field> not exist -> prompt for config
// if <field> exist, print config -> prompt to use Y/n -> prompt for config OR proceed
pub async fn handle_setup(config_path: &PathBuf, config_name: &str) -> Result<()>{
    let mut config = LocalConfig::new(&config_path, config_name)?;
    if config.api_token.is_none() {
        config = config.prompt_2fa().await?;
    };
    if config.user.is_none(){
        let user = config.get_user().await?;
        config.user = Some(user);
        config.write_settings("user.json")?;
    };
    config.print();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use assert_cmd::Command;
    use predicates::prelude::*; // https://docs.rs/predicates/2.0.3/predicates/
    use anyhow::{ Result, };
    use tempdir::TempDir;

    #[cfg(test)]
    #[tokio::test]
    async fn test_empty_config_prompts_all() -> Result<()>{
        let dir = TempDir::new("test_empty_config_prompts_all")?;
        let mut mock = MockConfigPrompter::new();
        mock.expect_prompt_email()
            .times(1)
            .return_const("test@print-nanny.com");

        let pathbuf = PathBuf::from(&dir.path());
        let pathname = dir.path().to_str().unwrap();
        handle_setup(&pathbuf,  pathname).await?;


        Ok(())
    }
}