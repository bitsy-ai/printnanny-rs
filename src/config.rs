use std::fs;
use std::path::{ PathBuf };
use log::{ info, error, debug, warn };
use glob::glob;

use printnanny_api_client::apis::auth_api::{ auth_email_create, auth_token_create };
use printnanny_api_client::apis::releases_api::{ releases_latest_retrieve };

use printnanny_api_client::apis::configuration::{ Configuration as APIConfiguration};

use thiserror::Error;
use anyhow::{ anyhow, Context, Result };
use dialoguer::{ Input, Confirm };
use dialoguer::theme::{ ColorfulTheme };
use serde::{ Serialize, Deserialize };
use config::{ConfigError, Config, File as ConfigFile, Environment};
use procfs::{ CpuInfo, Meminfo };

use crate::keypair::KeyPair;

#[derive(Error, Debug)]
pub enum AlreadyExistsError {
    #[error("🔴 Resource already exists {0}")]
    Required(String),
}

fn default_dot_path(suffix: &str) -> String {
    let dir = dirs::home_dir().unwrap().join(".printnanny").join(suffix);
    dir.into_os_string().into_string().unwrap()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigDirs {
    #[serde(default)]
    pub backups: String,
    #[serde(default)]
    pub base: String, 
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub secrets: String,
    #[serde(default)]
    pub settings: String,
}

impl ::std::default::Default for ConfigDirs {
    fn default() -> Self { Self { 
        base: default_dot_path(""),
        backups: default_dot_path("backups"),
        settings: default_dot_path("settings"),
        data: default_dot_path("data"),
        secrets: default_dot_path("secrets"),
    }}
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceInfo {
     
    #[serde(default)]
    pub api_config: APIConfiguration,

    #[serde(default)]
    pub dirs: ConfigDirs,

    #[serde(default)]
    pub gcp_project: String,

    #[serde(default, skip_serializing_if="Option::is_none")]
    pub device: Option<printnanny_api_client::models::Device>,
    #[serde(default, skip_serializing_if="Option::is_none")]
    pub user: Option<printnanny_api_client::models::User>,

    #[serde(default, skip_serializing_if="Option::is_none")]
    pub release: Option<printnanny_api_client::models::Release>,

    #[serde(default, skip_serializing_if="Option::is_none")]
    pub keypair: Option<KeyPair>,
}

impl ::std::default::Default for DeviceInfo {

    fn default() -> Self { Self { 
        api_config: APIConfiguration {
            base_path: "https://print-nanny.com".to_string(),
            ..Default::default()
        },
        dirs: ConfigDirs { ..Default::default() },
        device: None,
        gcp_project: "print-nanny".to_string(),
        user: None,
        keypair: None,
        release: None,
    }}
}


#[derive(Debug, Clone)]
pub struct SetupPrompter {
    pub config: DeviceInfo
}

impl SetupPrompter {
    pub fn new() -> Result<SetupPrompter> {
        let config = DeviceInfo::new()?;
        info!("Read config {:?}", config);
        Ok(SetupPrompter { config })
    }

    fn rm_dirs(&self) -> Result<()>{
        fs::remove_dir_all(&self.config.dirs.settings)
            .context(format!("Failed to rm dir {}", &self.config.dirs.settings))?;
        fs::create_dir(&self.config.dirs.settings)
            .context(format!("Failed to create dir {}", &self.config.dirs.settings))?;
        info!("Recreated settings dir {}", &self.config.dirs.settings);
        fs::remove_dir_all(&self.config.dirs.data)?;
        fs::create_dir(&self.config.dirs.data)?;
        info!("Recreated data dir {}", &self.config.dirs.data);
        Ok(())
    }

    pub fn reset(&self) -> Result<SetupPrompter> {
        let prompt = "Do you want to reset your Print Nanny settings?";
        let proceed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .default(true)
            .interact()?;
        match proceed {
            true => {
                self.rm_dirs()?;
                let prompter = SetupPrompter::new()?;
                Ok(prompter)
            }
            false => {
                Err(anyhow!("Failed to delete config {:?}", self.config))
            }
        }
    }

    // Basic flow goess
    // if <field> not exist -> prompt for config
    // if <field> exist, print config -> prompt to use Y/n -> prompt for config OR proceed
    
    // async fn get_or_create_camera(&self) -> Result<printnanny_api_client::models::Camera> {
    //     let
    // }

    // async fn get_or_create_device(&self, hostname: &str) -> Result<printnanny_api_client::models::Device> {
    //     let cpuinfo = CpuInfo::new()?;
    //     let unknown = "Unknown".to_string();
    //     let revision = cpuinfo.fields.get("Revision").unwrap_or(&unknown);
    //     let hardware = cpuinfo.fields.get("Hardware").unwrap_or(&unknown);
    //     let model = cpuinfo.fields.get("Model").unwrap_or(&unknown);
    //     let serial = cpuinfo.fields.get("Serial").unwrap_or(&unknown);
    //     let cores = cpuinfo.num_cores();
    //     let meminfo = Meminfo::new()?;
    //     let ram = meminfo.mem_total;

    //     let req = printnanny_api_client::models::DeviceRequest{
    //         cores: cores as i32,
    //         hostname: hostname.to_string(),
    //         hardware: hardware.to_string(),
    //         model: model.to_string(),
    //         serial: serial.to_string(),
    //         ram: ram as i64,
    //         revision: revision.to_string(),
    //         ..Default::default()
    //     };
    //     match printnanny_api_client::apis::devices_api::devices_create(&self.config.api_config, req.clone()).await {
    //         Ok(device) => return Ok(device),

    //         Err(e) => {
    //             let context = format!("devices_create returned error for request {:?}", &req);
    //             if let printnanny_api_client::apis::Error::ResponseError(t) = &e {      
    //                 match t.status {
    //                     http::status::StatusCode::CONFLICT => {
    //                         let warn_msg = format!("Found existing settings for {}", hostname);
    //                         let overwrite = self.prompt_overwrite(&warn_msg).unwrap();
    //                         match overwrite {
    //                             true => {
    //                                 info!("New host key will be generated for {}", &hostname);
    //                                 let device = printnanny_api_client::apis::devices_api::devices_retrieve_hostname(&self.config.api_config, hostname).await?;
    //                                 return Ok(device);
    //                             },
    //                             false => {
    //                                 error!("{:?}", &t.entity);
    //                             }
    //                         }fapi
    //                     }
    //                     _ => ()
    //                 }    
    //             }
    //             return Err(anyhow::Error::from(e).context(context));
    //         }
    //     };
    // }

    fn prompt_overwrite(&self, warn_msg: &str) -> Result<bool> {
        warn!("{}",warn_msg);
        let prompt = "Do you want to overwrite?";
        let proceed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .default(true)
            .interact()?;
        debug!("prompt_overwrite received input {}", proceed);
        Ok(proceed)
    }

    // pub async fn setup(mut self) -> Result<()>{
    //     if self.config.user.is_none() {
    //         let email = self.prompt_email();
    //         DeviceInfo::verify_2fa_send_email(&self.config, &email).await?;
    //         let opt_token = self.prompt_token_input(&email)?;
    //         let token_res = DeviceInfo::verify_2fa_code(&self.config, &email, opt_token).await?;
    //         self.config.api_config.bearer_access_token = Some(token_res.token);
    //         let user = self.config.get_user().await?;
    //         self.config.user = Some(user);
    //         info!("✅ Sucess! Verified identity {:?}", email);
    //         self.config.save_settings("local.json")?;
    //         info!("💜 Saved API config to {:?}", self.config.dirs.settings);
    //         info!("💜 Proceeding to device setup");
    //     };
    //     if self.config.device.is_none(){
    //         let hostname = self.prompt_hostname()?;
    //         let device = self.get_or_create_device(&hostname).await?;
    //         let device_id = device.id.unwrap();
    //         let keypair = KeyPair::create(
    //             PathBuf::from(&self.config.dirs.data),
    //             &self.config.api_config,
    //             &device_id
    //         ).await?;
    //         self.config.keypair = Some(keypair);
    //         self.config.device = Some(printnanny_api_client::apis::devices_api::devices_retrieve(
    //             &self.config.api_config,
    //             device_id
    //         ).await?);
    //         info!("✅ Sucess! Registered your device {:?}", &self.config.device);
    //         self.config.save_settings("local.json")?;
    //         info!("💜 Saved config to {:?}", self.config.dirs.settings);


    //     };
    //     Ok(())
    // }
    
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
        DeviceInfo::print_spacer();
        let prompt = "📨 Enter your email address";
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .interact_text()
            .unwrap()
    }
    fn prompt_token_input(&self, email: &str) -> Result<String> {
        let prompt = format!("⚪ Enter the 6-digit code emailed to {}", email);
        let input : String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .interact_text()
            .unwrap();
        info!("Received input code {}", input);
        Ok(input)
    }
}

impl DeviceInfo {
    /// Serializes settings stored in ~/.printnanny/settings/*json
    
    pub async fn refresh(mut self) -> Result<Self> {
        match &self.user {
            Some(_) => {
                self.user = Some(self.get_user().await?);
            },
            None => info!("No user detected in DeviceInfo.refresh()")
        }
        match &self.device {
            Some(device) => {
                let release_channel = &device.release_channel.as_ref().unwrap().to_string();
                self.device = Some(self.get_device(device.id.unwrap()).await?);
                self.release = Some(self.get_latest_release(&release_channel).await?);
            },
            None => {
                self.release = Some(self.get_latest_release(&printnanny_api_client::models::ReleaseChannelEnum::Stable.to_string()).await?);
            }
        }
        info!("Refreshed config from remote {:?}", &self);
        self.save_settings("local.json")?;
        Ok(self)
    }

    pub async fn get_latest_release(&self, release_channel: &str) -> Result<printnanny_api_client::models::Release> {
        let res = releases_latest_retrieve(&self.api_config, release_channel).await
            .context(format!("🔴 Failed to retreive latest for release_channel={}", release_channel))?;
        info!("SUCCESS auth_verify_create detail {:?}", serde_json::to_string(&res));
        Ok(res)
    }
    
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::default();
        // call Config::set_default for default in from DeviceInfo::default()
        let defaults = DeviceInfo::default();

        // https://github.com/mehcode/config-rs/blob/master/examples/hierarchical-env/src/settings.rs
        // Start off by merging in the "default" configuration file
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        // s.merge(Environment::with_prefix("PRINTNANNY").separator("_"))?;

        // glob all files in config directory
        let glob_pattern = format!("{}/*", defaults.dirs.settings);
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
        s.merge(Environment::with_prefix("PRINTNANNY").separator("__"))?;

        // You may also programmatically change settings
        // s.set("dirs.settings", dirs.settings)?;

        // Now that we're done, let's access our configuration

        // You can deserialize (and thus freeze) the entire configuration as
        s.try_into()
    }

    async fn verify_2fa_send_email(&self, email: &str) -> Result<printnanny_api_client::models::DetailResponse> {
        // Sends an email containing an expiring one-time password (6 digits)
        let req =  printnanny_api_client::models::EmailAuthRequest{email: email.to_string()};
        let res = auth_email_create(&self.api_config, req).await
            .context(format!("🔴 Failed to send verification email to {:?}", self))?;
        info!("SUCCESS auth_email_create detail {:?}", serde_json::to_string(&res));
        Ok(res)
    }

    async fn verify_2fa_code(&self, email: &str, token: String) -> Result<printnanny_api_client::models::TokenResponse> {
        let req = printnanny_api_client::models::CallbackTokenAuthRequest{mobile: None, token, email:Some(email.to_string())};
        let res = auth_token_create(&self.api_config, req).await
            .context("🔴 Verification failed. Please try again or contact leigh@print-nanny.com for help.")?;
        info!("SUCCESS auth_verify_create detail {:?}", serde_json::to_string(&res));
        Ok(res)
    }

    pub fn print_reset(&self) {
        DeviceInfo::print_spacer();
        info!("💜 Config was reset!");
        info!("💜 To ");      
        DeviceInfo::print_spacer();
    }
    
    pub fn print_spacer() {
        let (w, _) = term_size::dimensions().unwrap_or((24,24));
        let spacer = (0..w/2).map(|_| "-").collect::<String>();
        info!("{}", spacer);
    }

    pub fn print_user(&self) {
        DeviceInfo::print_spacer();
        info!("💜 Logged in as user:");
        info!("💜 {:#?}", self.user);        
        DeviceInfo::print_spacer();
    }

    pub fn print(&self) {
        DeviceInfo::print_spacer();
        info!("💜 Print Nanny config:");
        info!("💜 {:#?}", self);
        DeviceInfo::print_spacer();
    }

    pub async fn get_user(&self) -> Result<printnanny_api_client::models::User> {
        let res = printnanny_api_client::apis::users_api::users_me_retrieve(
            &self.api_config
        ).await.context(format!("Failed to retreive user"))?;
        Ok(res)
    }

    pub async fn get_device(&self, device_id: i32) -> Result<printnanny_api_client::models::Device> {
        let res = printnanny_api_client::apis::devices_api::devices_retrieve(
            &self.api_config,
            device_id
        ).await.context(format!("Failed to retreive device id={}", device_id))?;
        Ok(res)
    }

    pub fn save_settings(&self, filename: &str) -> Result<()>{
        let save_path = PathBuf::from(&self.dirs.settings).join(filename);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&save_path)
            .context(format!("🔴 Failed to create file handle {:?}", save_path))?;
        // File::create("/home/leigh/.printnanny/settings.json")
        //     .context(format!("🔴 Failed to create file handle {:#?}",&self.dirs.settings))?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }

}
