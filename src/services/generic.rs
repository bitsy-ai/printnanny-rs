
use std::fs::{ read_to_string, OpenOptions };
use std::convert::From;
use std::path::{ PathBuf };

use anyhow::{ anyhow, Context, Result };
use async_trait::async_trait;
use clap::arg_enum;
use serde::{ Serialize, Deserialize };

use printnanny_api_client::models::print_nanny_api_config::PrintNannyApiConfig;
use printnanny_api_client::apis::configuration::Configuration;

use printnanny_api_client::models::{ 
    License,
};
use crate::paths::{ PrintNannyPath };

#[derive(Debug, Clone)]
pub struct PrintNannyService<T>{
    pub api_config: PrintNannyApiConfig,
    pub request_config: Configuration,
    pub paths: PrintNannyPath,
    pub config: String,
    pub item: Option<T>
}

impl<T> PrintNannyService<T> {
    pub fn new(config: &str) -> Result<PrintNannyService<T>> {
        let paths = PrintNannyPath::new(config);

        // api_config.json is bundled in printnanny_license.zip
        let api_config = serde_json::from_str::<PrintNannyApiConfig>(
            &read_to_string(&paths.api_config_json)
            .context(format!("Failed to read {:?}", paths.device_json))?
            )?;

        let request_config = Configuration{ 
            base_path: api_config.base_path.clone(),
            bearer_access_token: Some(api_config.bearer_access_token.clone()),
            ..Configuration::default()
        };

        Ok(PrintNannyService{request_config, api_config, paths, item: None, config: config.to_string() })
    }
}

#[async_trait]
pub trait ApiService<T:serde::de::DeserializeOwned + Serialize> {
    // async fn create<T, R>(&self, request: R) -> Result<T>;
    async fn retrieve(&self) -> Result<T>;
    // async fn partial_update<T, R>(&self, id: &i32, rquest: R) -> Result<T>;
    // async fn update<T, R>(&self, id: &i32, request: R) -> Result<T>;

    fn read_json(&self, path: &PathBuf) -> Result<T> {
        let result = serde_json::from_str::<T>(
            &read_to_string(path)
            .context(format!("Failed to read {:?}", path))?
            )?;
        Ok(result)
    }
    fn to_string_pretty(&self, item: T) -> Result<String> {
        Ok(serde_json::to_string_pretty::<T>(&item)?)
    }
}
