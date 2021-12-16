
use std::fs::{ read_to_string, OpenOptions };
use std::path::{ PathBuf };

use anyhow::{ anyhow, Context, Result };
use async_trait::async_trait;
use serde::{ Serialize, Deserialize };

use printnanny_api_client::models::print_nanny_api_config::PrintNannyApiConfig;
use printnanny_api_client::apis::configuration::Configuration;

use printnanny_api_client::models::{ 
    Device, License, TaskType
};
use crate::paths::{ PrintNannyPath };

#[derive(Debug, Clone)]
pub struct PrintNannyService<T>{
    pub api_config: PrintNannyApiConfig,
    pub request_config: Configuration,
    pub paths: PrintNannyPath,
    pub config: String,
    pub item: Option<T>,
    pub license: License // loaded from license.json
}

fn read_model_json<T:serde::de::DeserializeOwned >(path: &PathBuf) -> Result<T> {
    let result = serde_json::from_str::<T>(
        &read_to_string(path)
        .context(format!("Failed to read {:?}", path))?
        )?;
    Ok(result)
}

impl<T> PrintNannyService<T> {
    pub fn new(config: &str) -> Result<PrintNannyService<T>> {
        let paths = PrintNannyPath::new(config);

        // api_config.json is bundled in printnanny_license.zip
        let api_config = read_model_json::<PrintNannyApiConfig>(&paths.api_config_json)?;
        
        // license.json is bundled in printnanny_license.zip
        let license = read_model_json::<License>(&paths.license_json)?;

        let request_config = Configuration{ 
            base_path: api_config.base_path.clone(),
            bearer_access_token: Some(api_config.bearer_access_token.clone()),
            ..Configuration::default()
        };

        Ok(PrintNannyService{request_config, api_config, paths, license, item: None, config: config.to_string() })
    }
}

#[async_trait]
pub trait ApiService<T:serde::de::DeserializeOwned + Serialize> {
    // async fn create<T, R>(&self, request: R) -> Result<T>;
    async fn retrieve(&self, id: i32) -> Result<T>;
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

    fn check_task_type(&self, device: &Device, expected_type: TaskType) -> Result<()>{
        match &device.last_task {
            Some(last_task) => {
                if last_task.task_type.unwrap() != expected_type {
                    return Err(anyhow!("Expected Device.last_task to be {:?} but received task {:?}", expected_type, last_task))
                } else { Ok(()) }
            },
            None => {
                return Err(anyhow!("Expected Device.last_task to be {:?} but received task None", expected_type))
            }
        }
    }
}
