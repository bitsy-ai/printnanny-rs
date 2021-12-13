use std::fs::{ read_to_string, OpenOptions };

use anyhow::{ anyhow, Context, Result };
use clap::arg_enum;
use log::{ info };
use procfs::{ CpuInfo, Meminfo };

use printnanny_api_client::models::print_nanny_api_config::PrintNannyApiConfig;
use printnanny_api_client::apis::configuration::Configuration;
use printnanny_api_client::apis::devices_api::{
    devices_tasks_status_create,
    devices_active_license_retrieve,
    devices_retrieve,
    device_info_update_or_create
};
use printnanny_api_client::apis::licenses_api::{
    license_activate,
};
use printnanny_api_client::models::{ 
    Device, 
    DeviceInfo, 
    DeviceInfoRequest,
    License,
    Task,
    TaskStatus,
    TaskStatusRequest,
    TaskStatusType,
};
use crate::paths::{ PrintNannyPath };

#[derive(Debug, Clone)]
pub struct PrintNannyService {
    pub api_config: PrintNannyApiConfig,
    pub request_config: Configuration,
    pub paths: PrintNannyPath,
}

arg_enum!{
    #[derive(PartialEq, Debug, Clone)]
    pub enum ApiModel{
        Device,
        License,
    }
}

arg_enum!{
    #[derive(PartialEq, Debug, Clone)]
    pub enum ApiAction{
        Create,
        Get,
        PartialUpdate,
        Update,
        Delete,
    }
}

async fn device_api(config: &str, save: &bool, action: &ApiAction) -> Result<String> {
    let service = PrintNannyService::new(&config)?;
    match action {
        ApiAction::Get => {
            match save {
                true => Ok(service.save_device_json().await?),
                false => Ok(service.get_device_json().await?)
            }
        },
        _ => unimplemented!()
    }
}

async fn license_api(config: &str, save: &bool, action: &ApiAction) -> Result<String>{
    let service = PrintNannyService::new(&config)?;
    match action {
        ApiAction::Get => {
            match save {
                true => {
                    service.refresh_license_json().await?;
                    Ok(service.read_license_json().await?)
                },
                false => Ok(service.read_license_json().await?)
            }
        },
        _ => unimplemented!()
    }
}

pub async fn printnanny_api_call(config: &str, save: &bool, action: &ApiAction, model: &ApiModel) -> Result<String> {
    match model {
        ApiModel::Device => Ok(device_api(config, save, action).await?),
        ApiModel::License => Ok(license_api(config, save, action).await?)
    }
}

impl PrintNannyService {

    pub fn new(config: &str) -> Result<PrintNannyService>{
        let paths = PrintNannyPath::new(config);
        // read api_config.json from disk
        let api_config = serde_json::from_str::<PrintNannyApiConfig>(
            &read_to_string(&paths.api_config_json)
            .context(format!("Failed to read {:?}", paths.device_json))?
            )?;

        let request_config = Configuration{ 
            base_path: api_config.base_path.clone(),
            bearer_access_token: Some(api_config.bearer_access_token.clone()),
            ..Configuration::default()
        };

        Ok(PrintNannyService{request_config, api_config, paths})
    }

    pub async fn save(&self) -> Result<()> {
        self.paths.save()?;
        self.save_device().await?;
        Ok(())
    }
    pub async fn get_device(&self) -> Result<Device> {
        let device = devices_retrieve(&self.request_config, self.api_config.device_id).await?;
        Ok(device)
    }

    pub async fn get_device_json(&self) -> Result<String> {
        let device = devices_retrieve(&self.request_config, self.api_config.device_id).await?;
        let result = serde_json::to_string(&device)?;
        Ok(result)
    }

    pub async fn save_device(&self) -> Result<Device> {
        let device = devices_retrieve(&self.request_config, self.api_config.device_id).await?;
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.paths.device_json)?;
        serde_json::to_writer(&file, &device)?;
        Ok(device)
    }

    pub async fn save_device_json(&self) -> Result<String> {
        let device = self.save_device().await?;
        let result = serde_json::to_string(&device)?;
        Ok(result)
    }

    pub async fn update_task_status(
        &self,
        task: &Task,
        status: Option<TaskStatusType>,
        detail: Option<String>,
        wiki_url: Option<String>,
    ) -> Result<TaskStatus> {
        
        let request = TaskStatusRequest{
            detail, wiki_url, status, task: task.id
        };
        let device_id = task.device.to_string();
        let result = devices_tasks_status_create(
            &self.request_config, &device_id, task.id, request).await?;
        info!("Created TaskStatus {:?}", result);
        Ok(result)
    }

    pub async fn check_license(&self) -> Result<License>{
        let device = self.load_device_json().await?;
        let license = self.load_license_json().await?;
        let active_license = devices_active_license_retrieve(&self.request_config, device.id).await
            .context(format!("Failed to retrieve device with id={}", device.id))?;
        
        let remote_device = active_license.device.as_ref().unwrap();
        
        // device id mismatch (usually indicates wrong printnanny_license.zip copied)
        if device.id != remote_device.id {
            return Err(anyhow!("Device id mismatch {} {}", &device.id, &remote_device.id))
        }

        if active_license.fingerprint == license.fingerprint {
            Ok(active_license)
        } else {
            return Err(anyhow!("License fingerprint {} did not match Device.active_license for device with id={}", license.fingerprint, device.id))
        }
    }

    pub async fn activate_license(&self) -> Result<License> {
        let check = self.check_license().await?;
        let license = license_activate(&self.request_config, check.id, None).await?;
        Ok(license)
    }


    pub async fn refresh_license(&self) -> Result<License> {
        let license = self.check_license().await?;

        // test serde_json serialization before truncating file
        serde_json::to_string(&license)?;

        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.paths.license_json)?;
        serde_json::to_writer(&file, &license)?;
        Ok(license)
    }

    pub async fn load_license_json(&self) -> Result<License> {
        let result = serde_json::from_str::<License>(
            &read_to_string(&self.paths.license_json)
            .context(format!("Failed to read {:?}", self.paths.license_json))?
            )?;
        Ok(result)
    }

    pub async fn load_device_json(&self) -> Result<Device> {
        let result = serde_json::from_str::<Device>(
            &read_to_string(&self.paths.device_json)
            .context(format!("Failed to read {:?}", self.paths.device_json))?
            )?;
        Ok(result)
    }

    pub async fn read_license_json(&self) -> Result<String> {
        let license = self.check_license().await?;
        let result = serde_json::to_string(&license)?;
        Ok(result)
    }

    pub async fn refresh_license_json(&self) -> Result<String>{
        self.refresh_license().await?;
        let result = self.read_license_json().await?;
        Ok(result)
    }

    pub async fn device_info_update_or_create(&self) -> Result<DeviceInfo> {
        let machine_id = read_to_string("/etc/machine-id")?;
        let image_version = read_to_string("/boot/image_version.txt")?;
        let cpuinfo = CpuInfo::new()?;
        let unknown = "Unknown".to_string();
        let revision = cpuinfo.fields.get("Revision").unwrap_or(&unknown);
        let hardware = cpuinfo.fields.get("Hardware").unwrap_or(&unknown);
        let model = cpuinfo.fields.get("Model").unwrap_or(&unknown);
        let serial = cpuinfo.fields.get("Serial").unwrap_or(&unknown);
        let cores = cpuinfo.num_cores();
        let meminfo = Meminfo::new()?;
        let ram = meminfo.mem_total;
        let device_id = self.api_config.device_id;
        let req = DeviceInfoRequest{
            cores: cores as i32,
            device: device_id,
            hardware: hardware.to_string(),
            machine_id: machine_id,
            model: model.to_string(),
            ram: ram as i64,
            revision: revision.to_string(),
            serial: serial.to_string(),
            image_version: image_version
        };
        let res = device_info_update_or_create(&self.request_config, device_id, req).await?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.paths.device_info_json)?;
        serde_json::to_writer(file, &res)
            .context(format!("Failed to save DeviceInfo to {:?}", &self.paths.device_info_json))?;
        info!("Created DeviceInfo {:?}", res);
        Ok(res)
    }    
}