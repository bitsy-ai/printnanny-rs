use std::path::{ PathBuf };
use std::fs;
use log::{ info, error, debug, warn };
use glob::glob;

use thiserror::Error;
use anyhow::{ anyhow, Context, Result };
use dialoguer::{ Input, Confirm };
use dialoguer::theme::{ ColorfulTheme };
use serde::{ Serialize, Deserialize };
use config::{ConfigError, Config, File as ConfigFile, Environment};

use print_nanny_client::apis::auth_api::{ auth_email_create, auth_token_create };
use crate::keypair::KeyPair;

#[derive(Error, Debug)]
pub enum AlreadyExistsError {
    #[error("🔴 Resource already exists {0}")]
    Required(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalConfig {
    
    #[serde(default)]
    pub api_base_path: String,

    #[serde(default, skip_serializing_if="Option::is_none")]
    pub api_token: Option<String>,
    #[serde(default)]
    pub config_path: String,

    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub data_path: String,

    #[serde(default)]
    pub hostname: Option<String>,

    #[serde(default, skip_serializing_if="Option::is_none")]
    pub appliance: Option<print_nanny_client::models::Appliance>,
    #[serde(default, skip_serializing_if="Option::is_none")]
    pub user: Option<print_nanny_client::models::User>,
    #[serde(default, skip_serializing_if="Option::is_none")]
    pub keypair: Option<KeyPair>,
}

impl ::std::default::Default for LocalConfig {
    fn default() -> Self { Self { 
        api_base_path: "https://print-nanny.com".to_string(),
        api_token: None,
        config_path: ".tmp".to_string(),
        hostname: None,
        data_path: ".tmp".to_string(),
        appliance: None,
        email: None,
        user: None,
        keypair: None,
    }}
}

#[derive(Debug, Clone)]
pub struct SetupPrompter {
    pub config: LocalConfig
}

impl SetupPrompter {
    pub fn new() -> Result<SetupPrompter> {
        let config = LocalConfig::from()?;
        info!("Read config {:?}", &config);
        Ok(SetupPrompter { config })
    }


    // Basic flow goess
    // if <field> not exist -> prompt for config
    // if <field> exist, print config -> prompt to use Y/n -> prompt for config OR proceed
    
    // async fn get_or_create_camera(&self) -> Result<print_nanny_client::models::Camera> {
    //     let
    // }

    async fn get_or_create_appliance(&self) -> Result<print_nanny_client::models::Appliance> {
        let hostname = self.config.hostname.as_ref().unwrap();
        let api_config = self.config.api_config();
        let req = print_nanny_client::models::ApplianceRequest{hostname: hostname.to_string()};
        match print_nanny_client::apis::appliances_api::appliances_create(&api_config, req.clone()).await {
            Ok(appliance) => return Ok(appliance),

            Err(e) => {
                let context = format!("appliances_create returned error for request {:?}", &req);
                if let print_nanny_client::apis::Error::ResponseError(t) = &e {      
                    match t.status {
                        http::status::StatusCode::CONFLICT => {
                            let warn_msg = format!("Found existing settings for {}", hostname);
                            let overwrite = self.prompt_overwrite(&warn_msg).unwrap();
                            match overwrite {
                                true => {
                                    info!("New host key will be generated for {}", &hostname);
                                    let appliance = print_nanny_client::apis::appliances_api::appliances_retrieve_hostname(&api_config, hostname).await?;
                                    return Ok(appliance);
                                },
                                false => {
                                    error!("{:?}", &t.entity);
                                }
                            }
                        }
                        _ => ()
                    }    
                }
                return Err(anyhow::Error::from(e).context(context));
            }
        };
    }

    fn prompt_overwrite(&self, warn_msg: &str) -> Result<bool> {
        warn!("{}",warn_msg);
        let prompt = "Do you want to overrite? Settings will be backed up";
        let proceed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .default(true)
            .interact()?;
        debug!("prompt_overwrite received input {}", proceed);
        Ok(proceed)
    }

    fn prompt_camera_name(&self) -> Result<String> {
        let hostname = sys_info::hostname()?;
        LocalConfig::print_spacer();
        let prompt = "📷 Enter a name for this camera";
        let default = format!("{}-camera-0", hostname);
        let input = Input::new()
            .default(default)
            .with_prompt(prompt)
            .interact_text()
            .unwrap();
        Ok(input)
    }

    fn prompt_camera_source(&self) -> String {
        LocalConfig::print_spacer();
        let prompt = "📷 Enter camera source";
        let default = "/dev/video0";
        let result: String = Input::with_theme(&ColorfulTheme::default())
            // .default(default)
            .with_prompt(prompt)
            .interact_text()
            .unwrap();
        return result
    }

    pub async fn setup(mut self) -> Result<()>{
        if self.config.email.is_none() {
            self.config.email = Some(self.prompt_email());
        };
        if self.config.api_token.is_none() {
            LocalConfig::verify_2fa_send_email(&self.config).await?;
            let opt_token = self.prompt_token_input()?;
            let token_res = LocalConfig::verify_2fa_code(&self.config, opt_token).await?;
            self.config.api_token = Some(token_res.token);
        };
        if self.config.user.is_none(){
            let user = self.config.get_user().await?;
            self.config.user = Some(user);
            info!("✅ Sucess! Verified identity {:?}", self.config.email);
            self.config.save_settings("local.json")?;
            info!("💜 Saved API config to {:?}", self.config.config_path);
            info!("💜 Proceeding to device setup");
        };
        if self.config.appliance.is_none(){
            self.config.hostname = Some(self.prompt_hostname()?);
            self.config.appliance = Some(self.get_or_create_appliance().await?);
            let appliance_id = self.config.appliance.as_ref().unwrap().id.unwrap();
            let keypair = KeyPair::create(
                PathBuf::from(&self.config.data_path),
                &self.config.api_config(),
                &appliance_id
            ).await?;
            self.config.keypair = Some(keypair);
            info!("✅ Sucess! Registered your device {:?}", &self.config.appliance);
            self.config.save_settings("local.json")?;
            info!("💜 Saved config to {:?}", self.config.config_path);


        };
        // if self.config.appliance.is_some(){
        //     let add_camera = if self.config.appliance.cameras.is_some(){
        //         self.prompt_add_camera();
        //     } else { true };
        //     let camera_name = self.prompt_camera_name();
        //     let camera_source = self.prompt_camera_source();

        // }

        // LocalConfig::print_spacer();
        // info!("✅ Sucess! Verified identity {:?}", self.config.email);
        // self.config.save_settings("local.json");
        // info!("💜 Saved API config to {:?}", self.config.config_path);
        // LocalConfig::print_spacer();
        // info!("💜 Proceeding to device setup");
        Ok(())
    }

    fn prompt_add_camera(&self) -> Result<bool> {
        let prompt = "Do you want to add another camera?";
        let proceed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .default(true)
            .interact()?;
        debug!("prompt_overwrite received input {}", proceed);
        Ok(proceed)
    }
    fn prompt_hostname(&self) -> Result<String> {
        let hostname = sys_info::hostname()?;
        let prompt = "Please enter a name for this device";
        let input : String = Input::with_theme(&ColorfulTheme::default())
            .default(hostname)
            .with_prompt(prompt)
            .interact_text()
            .unwrap();
        info!("Received input code {}", input);
        Ok(input)
    }

    fn prompt_email(&self) -> String {
        LocalConfig::print_spacer();
        let prompt = "📨 Enter your email address";
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .interact_text()
            .unwrap()
    }
    fn prompt_token_input(&self) -> Result<String> {
        match &self.config.email {
            Some(email) => {
                let prompt = format!("⚪ Enter the 6-digit code emailed to {}", email);
                let input : String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt(prompt)
                    .interact_text()
                    .unwrap();
                info!("Received input code {}", input);
                Ok(input)
            }
            None => Err(anyhow!("SetupPrompter.prompt_token_input requires email to be set"))
        }
    }
}

impl LocalConfig {
    /// Serializes settings stored in ~/.printnanny/settings/*json
    
    pub fn from() -> Result<Self, ConfigError> {
        let mut s = Config::default();
        // call Config::set_default for default in from LocalConfig::default()
        let defaults = LocalConfig::default();
        s.set_default("api_base_path", defaults.api_base_path.clone())?;
        s.set_default("config_path", defaults.config_path.clone())?;
        s.set_default("key_path", defaults.data_path.clone())?;

        // https://github.com/mehcode/config-rs/blob/master/examples/hierarchical-env/src/settings.rs
        // Start off by merging in the "default" configuration file
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        s.merge(Environment::with_prefix("PRINTNANNY"))?;

        // glob all files in config directory
        let glob_pattern = format!("{}/*", &defaults.config_path);
        info!("Loading config from {}", &glob_pattern);

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
    
    async fn appliances_create(&self) -> Result<print_nanny_client::models::Appliance> {
        match &self.hostname {
            Some(hostname) => {
                let req = print_nanny_client::models::ApplianceRequest{hostname: hostname.to_string()};
                let res = print_nanny_client::apis::appliances_api::appliances_create(&self.api_config(), req.clone()).await
                    .context(format!("🔴 Failed to create appliance from request {:?}", req))?;
                Ok(res)
            }
            None => Err(anyhow!("Could not detect hostname. Please try running `printnanny setup` again."))
        }
    }

    async fn verify_2fa_send_email(&self) -> Result<print_nanny_client::models::DetailResponse> {
        // Sends an email containing an expiring one-time password (6 digits)
        match &self.email {
            Some(email) => {
                let req =  print_nanny_client::models::EmailAuthRequest{email: email.to_string()};
                let res = auth_email_create(&self.api_config(), req).await
                    .context(format!("🔴 Failed to send verification email to {:?}", self))?;
                info!("SUCCESS auth_email_create detail {:?}", serde_json::to_string(&res));
                Ok(res)
            }
            None => Err(anyhow!("LocalConfig.verify_2fa_send_email requires email to be set"))
        }

    }

    async fn verify_2fa_code(&self, token: String) -> Result<print_nanny_client::models::TokenResponse> {
        match &self.email {
            Some(email) => {
                let req = print_nanny_client::models::CallbackTokenAuthRequest{mobile: None, token, email:Some(email.to_string())};
                let res = auth_token_create(&self.api_config(), req).await
                    .context("🔴 Verification failed. Please try again or contact leigh@print-nanny.com for help.")?;
                info!("SUCCESS auth_verify_create detail {:?}", serde_json::to_string(&res));
                Ok(res)
            }
            None => Err(anyhow!("LocalConfig.verify_2fa_code requires email to be set"))

        }

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
    // pub async fn update_or_create_appliance(&self) -> Result<Appliance>{
    //     let res = print_nanny_client::apis::users_api::users_me_retrieve(
    //         &self.api_config()
    //     ).await.context(format!("🔴 Failed to retreive user {:#?}", self.email))?;
    //     Ok(res)
    // }

    pub async fn get_user(&self) -> Result<print_nanny_client::models::User> {
        let res = print_nanny_client::apis::users_api::users_me_retrieve(
            &self.api_config()
        ).await.context(format!("🔴 Failed to retreive user {:#?}", self.email))?;
        Ok(res)
    }
    
    pub fn save_settings(&self, filename: &str) -> Result<()>{
        let save_path = PathBuf::from(&self.config_path).join(filename);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&save_path)
            .context(format!("🔴 Failed to create file handle {:?}", save_path))?;
        // File::create("/home/leigh/.printnanny/settings.json")
        //     .context(format!("🔴 Failed to create file handle {:#?}",&self.config_path))?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }

}
